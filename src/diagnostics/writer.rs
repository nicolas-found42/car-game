use super::{
    EvidenceStream, RecorderError,
    health::HealthState,
    recording::{Job, Record},
};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
    },
    time::{Duration, Instant},
};

pub struct Writer {
    health: Arc<HealthState>,
    files: BTreeMap<EvidenceStream, BufWriter<File>>,
    periodic_pending: Arc<AtomicBool>,
    last_tick: u64,
}
impl Writer {
    pub fn open(
        health: Arc<HealthState>,
        periodic_pending: Arc<AtomicBool>,
    ) -> Result<Self, RecorderError> {
        let mut files = BTreeMap::new();
        for stream in [
            EvidenceStream::Events,
            EvidenceStream::Commands,
            EvidenceStream::Physics,
            EvidenceStream::Input,
            EvidenceStream::Log,
        ] {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(health.dir.join(stream.file()))
                .map_err(|e| RecorderError::new("open", stream.name(), e.kind()))?;
            files.insert(stream, BufWriter::with_capacity(65536, file));
        }
        Ok(Self {
            health,
            files,
            periodic_pending,
            last_tick: 0,
        })
    }
    pub fn run(mut self, receiver: Receiver<Job>) {
        let mut last_flush = Instant::now();
        loop {
            let next = receiver.recv_timeout(Duration::from_millis(50));
            self.handle_panic();
            match next {
                Ok(Job::Record(record)) => self.write(record),
                Ok(Job::Incident {
                    reason,
                    snapshot,
                    ack,
                }) => {
                    let seq = self.health.update(|d| d.last_written_seq);
                    self.flush(seq);
                    let result = self.health.incident_published.load(Ordering::Acquire)
                        || super::incident::publish(&self.health, &reason, snapshot);
                    if result {
                        self.health
                            .incident_published
                            .store(true, Ordering::Release);
                    }
                    self.health.persist();
                    let _ = ack.try_send(result);
                }
                Ok(Job::Flush {
                    tick,
                    seq,
                    stop,
                    ack,
                }) => {
                    self.last_tick = tick;
                    if stop {
                        self.health.update(|d| {
                            d.phase = if self.health.failed() {
                                "incomplete".into()
                            } else {
                                "drained".into()
                            }
                        });
                    }
                    let result = self.flush(seq);
                    self.periodic_pending.store(false, Ordering::Release);
                    if let Some(ack) = ack {
                        let _ = ack.try_send(result);
                    }
                    if stop {
                        break;
                    }
                    last_flush = Instant::now();
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    self.health.error(
                        "drop",
                        "recorder",
                        "writer_disconnected_without_finalization",
                    );
                    self.health.update(|d| d.phase = "incomplete".into());
                    self.flush(0);
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => (),
            }
            if last_flush.elapsed() >= Duration::from_millis(self.health.config.flush_interval_ms) {
                let seq = self.health.update(|d| d.last_written_seq);
                self.flush(seq);
                last_flush = Instant::now();
            }
        }
    }
    fn handle_panic(&mut self) {
        if !self.health.panic_requested.swap(false, Ordering::AcqRel) {
            return;
        }
        self.health.error("panic", "recorder", "panic_requested");
        self.health.update(|data| {
            data.shutdown_reason = "panic".into();
            data.loss_ranges_unknown = true;
        });
        self.health.write_json(
            &self.health.dir.join("panic.json"),
            &serde_json::json!({"schema":1,"reason":"panic","at":super::now(),"context":self.health.panic_context()}),
        );
        if !self.health.incident_published.load(Ordering::Acquire) {
            let snapshot = self
                .health
                .history
                .try_lock()
                .ok()
                .map(|history| history.snapshot());
            if super::incident::publish(&self.health, "panic", snapshot) {
                self.health
                    .incident_published
                    .store(true, Ordering::Release);
            }
        }
        self.health.persist();
    }
    fn write(&mut self, record: Arc<Record>) {
        let start = Instant::now();
        if self.health.config.writer_delay_ms > 0
            && !self.health.cancel_requested.load(Ordering::Acquire)
        {
            std::thread::sleep(Duration::from_millis(self.health.config.writer_delay_ms));
        }
        if self.health.cancel_requested.load(Ordering::Acquire) {
            self.health.update(|data| {
                data.queued_records -= 1;
                data.queued_bytes -= record.cost();
            });
            self.health
                .loss(record.seq, record.tick, "shutdown_deadline");
            return;
        }
        let stage = if self.injected(super::RecorderFaultStage::Drop, record.seq) {
            Some("drop")
        } else if self.injected(super::RecorderFaultStage::Write, record.seq) {
            Some("write")
        } else {
            None
        };
        let result = if stage.is_some() {
            Err(std::io::Error::other("controlled_storage_failure"))
        } else {
            self.files
                .get_mut(&record.stream)
                .expect("opened stream")
                .write_all(&record.bytes)
        };
        self.health.update(|data| {
            data.queued_records -= 1;
            data.queued_bytes -= record.cost();
        });
        match result {
            Ok(()) => self.health.update(|data| {
                data.written += 1;
                data.last_written_seq = record.seq;
                let stream = data.streams.entry(record.stream.name().into()).or_default();
                stream.written += 1;
                stream.written_seq = record.seq;
            }),
            Err(error) => {
                self.health
                    .error(stage.unwrap_or("write"), record.stream.name(), error.kind());
                self.health
                    .loss(record.seq, record.tick, stage.unwrap_or("write"));
            }
        }
        self.health.observe("writer", start);
    }
    fn injected(&self, stage: super::RecorderFaultStage, count: u64) -> bool {
        self.health
            .config
            .fault
            .as_ref()
            .is_some_and(|fault| fault.stage == stage && count >= fault.after_records)
    }
    fn flush(&mut self, seq: u64) -> bool {
        let start = Instant::now();
        let mut success = true;
        let count = self.health.update(|d| d.written);
        let fail_flush = self.injected(super::RecorderFaultStage::Flush, count);
        let fail_sync = self.injected(super::RecorderFaultStage::Sync, count);
        for (stream, file) in &mut self.files {
            let flushed = if fail_flush {
                Err(std::io::Error::other("controlled_flush_failure"))
            } else {
                file.flush()
            };
            if let Err(error) = flushed {
                self.health.error("flush", stream.name(), error.kind());
                success = false;
                continue;
            }
            self.health.update(|data| {
                let s = data.streams.entry(stream.name().into()).or_default();
                s.flushed_seq = s.written_seq;
            });
            let synced = if fail_sync {
                Err(std::io::Error::other("controlled_sync_failure"))
            } else {
                file.get_ref().sync_all()
            };
            if let Err(error) = synced {
                self.health.error("sync", stream.name(), error.kind());
                success = false;
                continue;
            }
            self.health.update(|data| {
                let s = data.streams.entry(stream.name().into()).or_default();
                s.synced_seq = s.flushed_seq;
            });
        }
        let previous = self.health.update(|data| {
            let old = (data.last_durable_tick, data.durable_seq);
            if success && !self.health.failed() {
                data.last_durable_tick = self.last_tick;
                data.durable_seq = seq;
            }
            old
        });
        if !self.health.persist() {
            self.health.update(|d| {
                d.last_durable_tick = previous.0;
                d.durable_seq = previous.1;
            });
            success = false;
        }
        self.health.observe("flush_sync", start);
        success
    }
}
