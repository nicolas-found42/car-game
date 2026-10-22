use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecorderConfig {
    pub queue_records: usize,
    pub queue_bytes: usize,
    pub max_record_bytes: usize,
    pub flush_ticks: u64,
    pub flush_interval_ms: u64,
    pub drain_timeout_ms: u64,
    pub writer_delay_ms: u64,
    pub fault: Option<RecorderFault>,
    pub history_ticks: u64,
    pub history_bytes: usize,
    pub history_records: usize,
    pub finalize_max_bytes: u64,
}
impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            queue_records: 8192,
            queue_bytes: 8 * 1024 * 1024,
            max_record_bytes: 1024 * 1024,
            flush_ticks: 120,
            flush_interval_ms: 500,
            drain_timeout_ms: 2000,
            writer_delay_ms: 0,
            fault: None,
            history_ticks: 3600,
            history_bytes: 64 * 1024 * 1024,
            history_records: 32768,
            finalize_max_bytes: 512 * 1024 * 1024,
        }
    }
}
impl RecorderConfig {
    pub fn from_args(args: &[String]) -> Result<Self, super::RecorderError> {
        let mut value = serde_json::to_value(Self::default()).expect("recorder configuration");
        for field in [
            "queue_records",
            "queue_bytes",
            "max_record_bytes",
            "flush_ticks",
            "flush_interval_ms",
            "drain_timeout_ms",
            "writer_delay_ms",
            "history_ticks",
            "history_bytes",
            "history_records",
            "finalize_max_bytes",
        ] {
            let flag = format!("--recorder-{}", field.replace('_', "-"));
            if let Some(index) = args.iter().position(|arg| arg == &flag) {
                let number = args
                    .get(index + 1)
                    .and_then(|v| v.parse::<u64>().ok())
                    .ok_or_else(|| {
                        super::RecorderError::new(
                            "configuration",
                            "recorder",
                            "expected_unsigned_integer",
                        )
                    })?;
                value[field] = number.into();
            }
        }
        if let Some(index) = args.iter().position(|arg| arg == "--recorder-fault") {
            let (stage, after) = args
                .get(index + 1)
                .and_then(|v| v.split_once(':'))
                .ok_or_else(|| {
                    super::RecorderError::new(
                        "configuration",
                        "recorder",
                        "expected_stage:after_records",
                    )
                })?;
            let stage: RecorderFaultStage = serde_json::from_value(stage.into()).map_err(|_| {
                super::RecorderError::new("configuration", "recorder", "unknown_fault_stage")
            })?;
            let after_records = after.parse().map_err(|_| {
                super::RecorderError::new("configuration", "recorder", "invalid_fault_threshold")
            })?;
            value["fault"] = serde_json::to_value(RecorderFault {
                stage,
                after_records,
            })
            .expect("fault configuration");
        }
        let config: Self = serde_json::from_value(value).map_err(|_| {
            super::RecorderError::new("configuration", "recorder", "invalid_configuration")
        })?;
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<(), super::RecorderError> {
        if !(1..=32768).contains(&self.queue_records)
            || !(128..=64 * 1024 * 1024).contains(&self.queue_bytes)
            || !(64..=1024 * 1024).contains(&self.max_record_bytes)
            || self.max_record_bytes > self.queue_bytes
            || !(1..=3600).contains(&self.flush_ticks)
            || !(10..=5000).contains(&self.flush_interval_ms)
            || !(10..=5000).contains(&self.drain_timeout_ms)
            || self.writer_delay_ms > 1000
            || !(1..=7200).contains(&self.history_ticks)
            || !(1024..=128 * 1024 * 1024).contains(&self.history_bytes)
            || !(1..=65536).contains(&self.history_records)
            || !(1024..=1024 * 1024 * 1024).contains(&self.finalize_max_bytes)
            || self.fault.as_ref().is_some_and(|fault| {
                fault.stage == RecorderFaultStage::Panic && fault.after_records == 0
            })
        {
            return Err(super::RecorderError::new(
                "configuration",
                "recorder",
                "invalid_recorder_bounds",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecorderFaultStage {
    Write,
    Flush,
    Sync,
    Drop,
    Panic,
}
impl RecorderFaultStage {
    pub fn name(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Flush => "flush",
            Self::Sync => "sync",
            Self::Drop => "drop",
            Self::Panic => "panic",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecorderFault {
    pub stage: RecorderFaultStage,
    pub after_records: u64,
}
