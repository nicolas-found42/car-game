use super::{EvidenceStream, RunRecorder};
use bevy::log::{BoxedLayer, tracing_subscriber};
use std::{
    io::{self, Write},
    sync::Arc,
    time::Instant,
};

pub fn layer(run: Arc<RunRecorder>) -> BoxedLayer {
    Box::new(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(move || LogRecord {
                run: run.clone(),
                bytes: Vec::new(),
                failed: false,
            }),
    )
}
struct LogRecord {
    run: Arc<RunRecorder>,
    bytes: Vec<u8>,
    failed: bool,
}
impl Write for LogRecord {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.bytes.len() + bytes.len() > self.run.recorder_config().max_record_bytes {
            self.failed = true;
            self.run
                .health
                .error("oversize", "log", "log_record_too_large");
            return Err(io::Error::other("log_record_too_large"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for LogRecord {
    fn drop(&mut self) {
        if self.failed || self.bytes.is_empty() || self.run.recording.closed() {
            return;
        }
        let start = Instant::now();
        self.run.recording.bytes(
            EvidenceStream::Log,
            self.run.envelope().tick,
            std::mem::take(&mut self.bytes),
        );
        self.run.health.observe("producer_log", start);
    }
}
