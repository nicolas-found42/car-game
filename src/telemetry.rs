//! Common clock/envelope for command ingress, effects and legacy diagnostic events.
use serde::Serialize;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

#[derive(Clone, Debug, Default, Serialize)]
pub struct SpanContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_id: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Envelope {
    pub schema_version: u32,
    pub run_id: String,
    pub actor_id: Option<String>,
    pub tick: u64,
    pub boundary: String,
    pub command_id: Option<String>,
    pub correlation_id: Option<String>,
    pub render_frame: u64,
    pub wall_timestamp: String,
    pub elapsed_ns: u128,
    #[serde(flatten)]
    pub span: SpanContext,
}
pub struct EvidenceClock {
    start: Instant,
    tick: AtomicU64,
    frame: AtomicU64,
    command: AtomicU64,
}
impl Default for EvidenceClock {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            tick: AtomicU64::new(0),
            frame: AtomicU64::new(0),
            command: AtomicU64::new(1),
        }
    }
}
impl EvidenceClock {
    pub fn completed(&self, tick: u64) {
        self.tick.store(tick, Ordering::Relaxed);
    }
    pub fn frame(&self, frame: u64) {
        self.frame.store(frame, Ordering::Relaxed);
    }
    pub fn next_command(&self, run_id: &str) -> String {
        format!(
            "{run_id}:command:{}",
            self.command.fetch_add(1, Ordering::Relaxed)
        )
    }
    pub fn envelope(&self, run_id: &str) -> Envelope {
        Envelope {
            schema_version: 1,
            run_id: run_id.into(),
            actor_id: None,
            tick: self.tick.load(Ordering::Relaxed),
            boundary: "last_completed_tick".into(),
            command_id: None,
            correlation_id: None,
            render_frame: self.frame.load(Ordering::Relaxed),
            wall_timestamp: crate::diagnostics::now(),
            elapsed_ns: self.start.elapsed().as_nanos(),
            span: SpanContext::default(),
        }
    }
}
