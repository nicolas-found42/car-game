use super::RecorderConfig;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

#[derive(Clone, Default, Serialize)]
pub struct StreamHealth {
    pub accepted: u64,
    pub written: u64,
    pub written_seq: u64,
    pub flushed_seq: u64,
    pub synced_seq: u64,
}
#[derive(Clone, Serialize)]
pub struct Loss {
    pub first_seq: u64,
    pub last_seq: u64,
    pub first_tick: u64,
    pub last_tick: u64,
    pub reason: String,
}
#[derive(Clone, Default)]
pub struct Latency {
    count: u64,
    total_ns: u64,
    max_ns: u64,
    buckets: BTreeMap<u32, u64>,
}
impl Latency {
    fn observe(&mut self, ns: u64) {
        self.count += 1;
        self.total_ns = self.total_ns.saturating_add(ns);
        self.max_ns = self.max_ns.max(ns);
        *self.buckets.entry(64 - ns.leading_zeros()).or_default() += 1;
    }
    fn quantile(&self, percent: u64) -> u64 {
        let target = (self.count * percent).div_ceil(100);
        let mut count = 0;
        for (bucket, n) in &self.buckets {
            count += n;
            if count >= target {
                return self
                    .max_ns
                    .min(1u64.checked_shl(*bucket).unwrap_or(u64::MAX));
            }
        }
        0
    }
    fn summary(&self) -> Value {
        json!({"count":self.count,"total_ns":self.total_ns,"max_ns":self.max_ns,
            "p50_upper_ns":self.quantile(50),"p95_upper_ns":self.quantile(95),"p99_upper_ns":self.quantile(99)})
    }
}
#[derive(Clone, Default)]
pub struct HealthData {
    pub phase: String,
    pub shutdown_reason: String,
    pub signal_support: bool,
    pub produced: u64,
    pub accepted: u64,
    pub written: u64,
    pub queued_bytes: usize,
    pub queued_records: usize,
    pub queue_high_bytes: usize,
    pub queue_high_records: usize,
    pub dropped: u64,
    pub losses: Vec<Loss>,
    pub loss_ranges_truncated: bool,
    pub loss_ranges_unknown: bool,
    pub errors: Vec<Value>,
    pub streams: BTreeMap<String, StreamHealth>,
    pub completed_tick: u64,
    pub last_durable_tick: u64,
    pub durable_seq: u64,
    pub last_written_seq: u64,
    pub metrics: BTreeMap<String, Latency>,
}
pub struct HealthState {
    pub failed: AtomicBool,
    pub panic_requested: AtomicBool,
    pub cancel_requested: AtomicBool,
    pub incident_published: AtomicBool,
    pub data: Mutex<HealthData>,
    pub config: RecorderConfig,
    pub dir: PathBuf,
    persist_gate: Mutex<()>,
    pub history: Mutex<super::history::History>,
}
impl HealthState {
    pub fn new(dir: PathBuf, config: RecorderConfig) -> Self {
        Self {
            failed: AtomicBool::new(false),
            panic_requested: AtomicBool::new(false),
            cancel_requested: AtomicBool::new(false),
            incident_published: AtomicBool::new(false),
            data: Mutex::new(HealthData {
                phase: "recording".into(),
                shutdown_reason: "normal".into(),
                ..Default::default()
            }),
            dir,
            history: Mutex::new(super::history::History::new(config.clone())),
            config,
            persist_gate: Mutex::new(()),
        }
    }
    pub fn update<R>(&self, f: impl FnOnce(&mut HealthData) -> R) -> R {
        f(&mut self.data.lock().unwrap_or_else(|e| e.into_inner()))
    }
    pub fn error(&self, stage: &str, stream: &str, error: impl std::fmt::Display) {
        self.failed.store(true, Ordering::Release);
        self.update(|data| {
            if data.errors.iter().any(|e| e["stage"]==stage && e["stream"]==stream) { return; }
            let record=json!({"stage":stage,"stream":stream,"error":error.to_string().chars().take(256).collect::<String>()});
            eprintln!("recorder_error: {record}");
            if data.errors.len()<32 { data.errors.push(record); }
        });
    }
    pub fn loss(&self, seq: u64, tick: u64, reason: &str) {
        self.failed.store(true, Ordering::Release);
        self.update(|data| {
            data.dropped += 1;
            if let Some(last) = data.losses.last_mut()
                && last.last_seq + 1 == seq
                && last.reason == reason
            {
                last.last_seq = seq;
                last.last_tick = tick;
            } else if data.losses.len() < 64 {
                data.losses.push(Loss {
                    first_seq: seq,
                    last_seq: seq,
                    first_tick: tick,
                    last_tick: tick,
                    reason: reason.into(),
                });
            } else {
                data.loss_ranges_truncated = true;
            }
        });
    }
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
    pub fn observe(&self, metric: &str, start: Instant) {
        let ns = start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        self.update(|data| data.metrics.entry(metric.into()).or_default().observe(ns));
    }
    pub fn snapshot(&self) -> Value {
        let data = self.update(|data| data.clone());
        let history = self
            .history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status();
        json!({"schema":1,"failed":self.failed(),"phase":data.phase,"errors":data.errors,
            "limits":self.config,"produced":data.produced,"accepted":data.accepted,"written":data.written,
            "queued_bytes":data.queued_bytes,"queued_records":data.queued_records,"queue_high_bytes":data.queue_high_bytes,
            "queue_high_records":data.queue_high_records,"dropped":data.dropped,"losses":data.losses,"loss_ranges_truncated":data.loss_ranges_truncated,"loss_ranges_unknown":data.loss_ranges_unknown,
            "streams":data.streams,"completed_tick":data.completed_tick,"last_durable_tick":data.last_durable_tick,"durable_seq":data.durable_seq,
            "overhead":data.metrics.iter().map(|(k,v)|(k.clone(),v.summary())).collect::<BTreeMap<_,_>>(),
            "updated_at":super::now(),"history":history,"shutdown_reason":data.shutdown_reason,"signal_support":data.signal_support,"run_id":self.dir.file_name().map(|v|v.to_string_lossy()),"panic_pending":self.panic_requested.load(Ordering::Acquire),"drain_cancelled":self.cancel_requested.load(Ordering::Acquire)})
    }
    pub fn persist(&self) -> bool {
        if !self.dir.is_dir() {
            self.error("health_publish", "health", "run_directory_unavailable");
            return false;
        }
        let _guard = self.persist_gate.lock().unwrap_or_else(|e| e.into_inner());
        match super::write_json_atomic(&self.dir.join("recorder-health.json"), &self.snapshot()) {
            Ok(()) => true,
            Err(error) => {
                self.error("health_publish", "health", error.kind());
                false
            }
        }
    }
    pub fn write_json(&self, path: &std::path::Path, value: &impl Serialize) -> bool {
        match super::write_json_atomic(path, value) {
            Ok(()) => true,
            Err(error) => {
                self.error("publish", "metadata", error.kind());
                false
            }
        }
    }
}
#[derive(Debug, Serialize)]
pub struct RecorderError {
    pub stage: String,
    pub stream: String,
    pub error: String,
}
impl RecorderError {
    pub fn new(stage: &str, stream: &str, error: impl std::fmt::Display) -> Self {
        Self {
            stage: stage.into(),
            stream: stream.into(),
            error: error.to_string(),
        }
    }
}
impl std::fmt::Display for RecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            json!({"schema":1,"stage":self.stage,"stream":self.stream,"error":self.error})
        )
    }
}
impl std::error::Error for RecorderError {}
