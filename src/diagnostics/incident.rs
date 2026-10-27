use super::{health::HealthState, history::HistorySnapshot};
use serde_json::json;
use std::{
    fs::{self, File},
    io::{self, BufWriter, Write},
    sync::Arc,
    time::Instant,
};

pub fn publish(health: &Arc<HealthState>, reason: &str, snapshot: Option<HistorySnapshot>) -> bool {
    let start = Instant::now();
    let result = write(health, reason, snapshot);
    health.observe("incident_export", start);
    if let Err(error) = result {
        health.error("incident_export", "history", error.kind());
        return false;
    }
    true
}
fn write(health: &HealthState, reason: &str, snapshot: Option<HistorySnapshot>) -> io::Result<()> {
    let Some(snapshot) = snapshot else {
        return Err(io::Error::other("history_unavailable"));
    };
    write_snapshot(&health.dir, &health.dir, &snapshot)?;
    super::write_json_atomic(
        &health.dir.join("incident.json"),
        &json!({"schema":1,"run_id":health.dir.file_name().expect("run directory").to_string_lossy(),
        "reason":reason,"at":super::now(),"history":snapshot.status,"health":health.snapshot(),
        "records":snapshot.records.len(),"path":"incident-history.jsonl"}),
    )
}
pub(super) fn write_snapshot(
    dir: &std::path::Path,
    run: &std::path::Path,
    snapshot: &HistorySnapshot,
) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = dir.join("incident-history.jsonl");
    let temporary = path.with_extension("tmp");
    let mut file = BufWriter::with_capacity(65536, File::create(&temporary)?);
    let run_id = run.file_name().expect("run directory").to_string_lossy();
    let id = serde_json::to_string(&run_id)?;
    for record in &snapshot.records {
        write!(
            file,
            "{{\"schema\":1,\"run_id\":{id},\"stream\":\"{}\",\"recorder_seq\":{},\"tick\":{},\"record\":",
            record.stream.name(),
            record.seq,
            record.tick
        )?;
        file.write_all(record.bytes.strip_suffix(b"\n").unwrap_or(&record.bytes))?;
        file.write_all(b"}\n")?;
    }
    file.flush()?;
    file.get_ref().sync_all()?;
    drop(file);
    fs::rename(temporary, &path)?;
    Ok(())
}
