use super::{RecorderError, health::HealthState};
use serde::Serialize;
use std::{
    io::{self, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStream {
    Events,
    Commands,
    Physics,
    Input,
    Log,
}
impl EvidenceStream {
    pub fn name(self) -> &'static str {
        match self {
            Self::Events => "events",
            Self::Commands => "commands",
            Self::Physics => "physics",
            Self::Input => "input",
            Self::Log => "log",
        }
    }
    pub fn file(self) -> &'static str {
        match self {
            Self::Events => "events.jsonl",
            Self::Commands => "commands.jsonl",
            Self::Physics => "physics.jsonl",
            Self::Input => "input.jsonl",
            Self::Log => "run.log",
        }
    }
}
pub struct Record {
    pub stream: EvidenceStream,
    pub tick: u64,
    pub seq: u64,
    pub bytes: Vec<u8>,
}
impl Record {
    pub fn cost(&self) -> usize {
        self.bytes.capacity() + 128
    }
}
pub enum Job {
    Record(Arc<Record>),
    Incident {
        reason: String,
        snapshot: Option<super::history::HistorySnapshot>,
        ack: SyncSender<bool>,
    },
    Flush {
        tick: u64,
        seq: u64,
        stop: bool,
        ack: Option<SyncSender<bool>>,
    },
}
struct LimitedBuffer {
    bytes: Vec<u8>,
    limit: usize,
    oversize: bool,
}
impl Write for LimitedBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            self.oversize = true;
            return Err(io::Error::other("record_too_large"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) fn bounded_json(
    payload: &impl Serialize,
    limit: usize,
    pretty: bool,
) -> Result<Vec<u8>, &'static str> {
    let mut buffer = LimitedBuffer {
        bytes: Vec::new(),
        limit,
        oversize: false,
    };
    let result = if pretty {
        serde_json::to_writer_pretty(&mut buffer, payload)
    } else {
        serde_json::to_writer(&mut buffer, payload)
    };
    match result {
        Ok(()) => Ok(buffer.bytes),
        Err(_) => Err(if buffer.oversize {
            "oversize"
        } else {
            "serialize"
        }),
    }
}
pub struct Recording {
    pub health: Arc<HealthState>,
    gate: Mutex<u64>,
    sender: SyncSender<Job>,
    closed: AtomicBool,
    panic_injected: AtomicBool,
    periodic_pending: Arc<AtomicBool>,
}
impl Recording {
    pub fn open(health: Arc<HealthState>) -> Result<Self, RecorderError> {
        let (sender, receiver) = mpsc::sync_channel(health.config.queue_records + 4);
        let pending = Arc::new(AtomicBool::new(false));
        let writer = super::writer::Writer::open(health.clone(), pending.clone())?;
        std::thread::Builder::new()
            .name("run-evidence".into())
            .spawn(move || writer.run(receiver))
            .map_err(|e| RecorderError::new("open", "writer_thread", e.kind()))?;
        Ok(Self {
            health,
            gate: Mutex::new(0),
            sender,
            closed: AtomicBool::new(false),
            panic_injected: AtomicBool::new(false),
            periodic_pending: pending,
        })
    }
    pub fn record(&self, stream: EvidenceStream, tick: u64, payload: &impl Serialize) -> bool {
        if self.closed.load(Ordering::Acquire) {
            eprintln!("recorder_closed: {}", stream.name());
            return false;
        }
        let start = Instant::now();
        let result = match bounded_json(payload, self.health.config.max_record_bytes - 1, false) {
            Ok(mut bytes) => {
                bytes.push(b'\n');
                self.bytes(stream, tick, bytes)
            }
            Err(stage) => {
                self.reject(stage, stream, tick);
                false
            }
        };
        self.health.observe("producer", start);
        result
    }
    fn reject(&self, stage: &str, stream: EvidenceStream, tick: u64) {
        let seq = {
            let mut seq = self.gate.lock().unwrap_or_else(|e| e.into_inner());
            *seq += 1;
            *seq
        };
        self.health.error(stage, stream.name(), stage);
        self.health.loss(seq, tick, stage);
    }
    pub fn bytes(&self, stream: EvidenceStream, tick: u64, bytes: Vec<u8>) -> bool {
        if self.closed.load(Ordering::Acquire) {
            eprintln!("recorder_closed: {}", stream.name());
            return false;
        }
        let mut seq = self.gate.lock().unwrap_or_else(|e| {
            self.health.error("producer", "recorder", "poisoned");
            e.into_inner()
        });
        if self.health.config.fault.as_ref().is_some_and(|fault| {
            fault.stage == super::RecorderFaultStage::Panic && *seq >= fault.after_records
        }) && !self.panic_injected.swap(true, Ordering::AcqRel)
        {
            self.health.failed.store(true, Ordering::Release);
            self.health.panic_requested.store(true, Ordering::Release);
            panic!("controlled recorder producer panic");
        }
        if self.closed.load(Ordering::Acquire) {
            eprintln!("recorder_closed: {}", stream.name());
            return false;
        }
        *seq += 1;
        let record = Arc::new(Record {
            stream,
            tick,
            seq: *seq,
            bytes,
        });
        self.health
            .history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(record.clone());
        let accepted = self.reserve(stream, record.cost());
        if !accepted {
            self.health
                .error("backpressure", stream.name(), "queue_limit");
            self.health.loss(*seq, tick, "queue_limit");
            return false;
        }
        let size = record.cost();
        if self.sender.try_send(Job::Record(record)).is_err() {
            self.release(size);
            self.health
                .error("enqueue", stream.name(), "writer_queue_unavailable");
            self.health.loss(*seq, tick, "enqueue");
            return false;
        }
        self.health.update(|data| {
            data.accepted += 1;
            data.streams
                .entry(stream.name().into())
                .or_default()
                .accepted += 1;
        });
        true
    }
    fn reserve(&self, _stream: EvidenceStream, size: usize) -> bool {
        self.health.update(|data| {
            data.produced += 1;
            if data.queued_records >= self.health.config.queue_records
                || data.queued_bytes + size > self.health.config.queue_bytes
            {
                return false;
            }
            data.queued_records += 1;
            data.queued_bytes += size;
            data.queue_high_records = data.queue_high_records.max(data.queued_records);
            data.queue_high_bytes = data.queue_high_bytes.max(data.queued_bytes);
            true
        })
    }
    fn release(&self, size: usize) {
        self.health.update(|d| {
            d.queued_records -= 1;
            d.queued_bytes -= size;
        });
    }
    pub fn completed(&self, tick: u64) {
        self.health.update(|data| data.completed_tick = tick);
        if let Ok(mut history) = self.health.history.try_lock() {
            history.advance(tick);
        }
        if tick.is_multiple_of(self.health.config.flush_ticks)
            && !self.periodic_pending.swap(true, Ordering::AcqRel)
            && !self.send_flush(false, None)
        {
            self.periodic_pending.store(false, Ordering::Release);
        }
    }
    fn send_flush(&self, stop: bool, ack: Option<SyncSender<bool>>) -> bool {
        let seq = self.gate.lock().unwrap_or_else(|e| e.into_inner());
        let tick = self.health.update(|data| data.completed_tick);
        if self
            .sender
            .try_send(Job::Flush {
                tick,
                seq: *seq,
                stop,
                ack,
            })
            .is_err()
        {
            self.health.error("drain", "recorder", "control_queue_full");
            return false;
        }
        true
    }
    pub fn flush(&self, stop: bool) -> bool {
        if self.closed.load(Ordering::Acquire) {
            return !self.health.failed();
        }
        if stop {
            self.closed.store(true, Ordering::Release);
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        if !self.send_flush(stop, Some(sender)) {
            if stop {
                self.health.cancel_requested.store(true, Ordering::Release);
            }
            return false;
        }
        match receiver.recv_timeout(Duration::from_millis(self.health.config.drain_timeout_ms)) {
            Ok(result) => result,
            Err(_) => {
                self.health.cancel_requested.store(true, Ordering::Release);
                self.health.error("drain", "recorder", "deadline_exceeded");
                self.health.update(|data| data.phase = "incomplete".into());
                false
            }
        }
    }
    pub fn incident(&self, reason: &str) -> bool {
        if self.health.incident_published.load(Ordering::Acquire) {
            return true;
        }
        if self.health.incident_pending.swap(true, Ordering::AcqRel) {
            return false;
        }
        if self.health.incident_published.load(Ordering::Acquire) {
            self.health.incident_pending.store(false, Ordering::Release);
            return true;
        }
        self.health.update(|data| data.incident_snapshots += 1);
        let snapshot = self
            .health
            .history
            .try_lock()
            .ok()
            .map(|history| history.snapshot());
        let (ack, receiver) = mpsc::sync_channel(1);
        if self
            .sender
            .try_send(Job::Incident {
                reason: reason.chars().take(128).collect(),
                snapshot,
                ack,
            })
            .is_err()
        {
            self.health.incident_pending.store(false, Ordering::Release);
            self.health
                .error("incident_export", "recorder", "control_queue_full");
            return false;
        }
        let success = receiver
            .recv_timeout(Duration::from_millis(self.health.config.drain_timeout_ms))
            .unwrap_or(false);
        if success {
            self.health
                .incident_published
                .store(true, Ordering::Release);
        } else {
            self.health
                .error("incident_export", "recorder", "deadline_or_export_failure");
        }
        success
    }
    pub fn closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        if !self.closed.load(Ordering::Acquire) {
            self.health
                .error("drop", "recorder", "recorder_dropped_without_finish");
            self.flush(true);
            self.health.update(|data| data.phase = "incomplete".into());
            self.health.persist();
        }
    }
}
