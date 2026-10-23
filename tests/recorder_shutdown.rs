#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

struct RecordingProcess {
    child: Child,
    root: PathBuf,
    run: Option<PathBuf>,
}
impl RecordingProcess {
    fn spawn(paused: bool, extra: &[&str]) -> Self {
        Self::spawn_until(paused, extra, 9999)
    }
    fn spawn_until(paused: bool, extra: &[&str], tick: u64) -> Self {
        let root = std::env::temp_dir().join(format!(
            "grid-shutdown-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let commands = if paused {
            json!([{"tick":10,"command":{"action":"set_paused","paused":true}}])
        } else {
            json!([])
        };
        fs::write(root.join("scenario.json"),json!({"schema_version":1,"name":"shutdown","scene":"driving","initial":{"car":"350z"},"commands":commands,"assertions":[{"tick":tick,"kind":"selected_car","car":"350z"}],"max_ticks":10000,"max_wall_ms":15000,"readiness_timeout_ms":5000}).to_string()).unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_car-game"))
            .args(["--render-disabled", "--scenario"])
            .arg(root.join("scenario.json"))
            .arg("--artifact-dir")
            .arg(root.join("runs"))
            .args(extra)
            .env("RUST_LOG", "warn")
            .env("CAR_GAME_MUTE", "1")
            .stdout(Stdio::from(
                fs::File::create(root.join("stdout.log")).unwrap(),
            ))
            .stderr(Stdio::from(
                fs::File::create(root.join("stderr.log")).unwrap(),
            ))
            .spawn()
            .unwrap();
        Self {
            child,
            root,
            run: None,
        }
    }
    fn health(&mut self) -> Option<Value> {
        if self.run.is_none() {
            self.run = fs::read_dir(self.root.join("runs"))
                .ok()?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .find(|p| p.is_dir());
        }
        serde_json::from_slice(&fs::read(self.run.as_ref()?.join("recorder-health.json")).ok()?)
            .ok()
    }
    fn wait_until(&mut self, predicate: impl Fn(&Value) -> bool) {
        let start = Instant::now();
        loop {
            if self.health().as_ref().is_some_and(&predicate) {
                return;
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "{}",
                fs::read_to_string(self.root.join("stderr.log"))
                    .unwrap_or_else(|error| format!("stderr unavailable: {error}"))
            );
            assert!(
                start.elapsed() < Duration::from_secs(8),
                "recording failed to reach readiness"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn signal(&mut self, signal: &str) {
        assert!(
            Command::new("/bin/kill")
                .args([signal, &self.child.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
    }
    fn wait(&mut self) -> std::process::ExitStatus {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "shutdown exceeded bounded deadline"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for RecordingProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if std::thread::panicking() {
            eprintln!("retained failed shutdown evidence: {}", self.root.display());
        } else {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

#[test]
fn sigterm_drains_buffered_recording_even_while_simulation_is_paused() {
    for paused in [false, true] {
        let mut process = RecordingProcess::spawn(paused, &[]);
        process.wait_until(|health| health["completed_tick"].as_u64().unwrap_or(0) >= 5);
        process.signal("-TERM");
        let status = process.wait();
        assert_eq!(
            status.code(),
            Some(0),
            "{}",
            fs::read_to_string(process.root.join("stderr.log")).unwrap()
        );
        let health = process.health().unwrap();
        assert_eq!(health["phase"], "finalized");
        assert_eq!(health["failed"], false);
        assert_eq!(health["shutdown_reason"], "termination_requested");
        assert_eq!(health["last_durable_tick"], health["completed_tick"]);
        assert!(
            process
                .run
                .as_ref()
                .unwrap()
                .join("incident-history.jsonl")
                .is_file()
        );
        assert!(
            !process
                .run
                .as_ref()
                .unwrap()
                .join("scenario-summary.json")
                .exists(),
            "termination must not invent passed assertions"
        );
    }
}

#[test]
fn forced_kill_is_reported_as_unknown_and_never_graceful() {
    let mut process = RecordingProcess::spawn(false, &[]);
    process.wait_until(|health| health["last_durable_tick"].as_u64().unwrap_or(0) >= 120);
    process.signal("-KILL");
    assert!(!process.wait().success());
    let result = Command::new("python3")
        .args(["tools/verify_run.py", "--health-only", "--json"])
        .arg(process.run.as_ref().unwrap())
        .output()
        .unwrap();
    assert!(!result.status.success());
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["status"], "unknown");
    assert!(
        report["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["code"] == "recorder_unclean_exit" && e["termination_cause"] == "unknown")
    );
}

#[test]
fn panic_with_a_busy_writer_exports_history_without_locking_the_panicking_producer() {
    let mut process = RecordingProcess::spawn(
        false,
        &[
            "--recorder-fault",
            "panic:200",
            "--recorder-writer-delay-ms",
            "2",
        ],
    );
    assert!(!process.wait().success());
    let health = process
        .health()
        .expect("panic must retain structured health");
    assert_eq!(health["failed"], true);
    assert_eq!(health["shutdown_reason"], "panic");
    let run = process.run.as_ref().unwrap();
    assert!(run.join("panic.json").is_file());
    assert!(
        fs::metadata(run.join("incident-history.jsonl"))
            .unwrap()
            .len()
            > 0
    );
    assert!(
        health["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error["stage"] == "panic")
    );
}

#[test]
fn interrupted_inventory_export_does_not_turn_writer_durability_into_run_success() {
    let mut process = RecordingProcess::spawn_until(false, &[], 240);
    process.wait_until(|health| health["completed_tick"].as_u64().unwrap_or(0) >= 5);
    let run = process.run.as_ref().unwrap().clone();
    let line = format!("{{\"payload\":\"{}\"}}\n", "x".repeat(900));
    fs::write(run.join("export-padding.jsonl"), line.repeat(32768)).unwrap();
    process.wait_until(|health| health["phase"] == "finalized");
    assert!(!run.join("evidence-manifest.json").exists());
    process.signal("-KILL");
    assert!(!process.wait().success());
    let result = Command::new("python3")
        .args(["tools/verify_run.py", "--health-only", "--json"])
        .arg(&run)
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(!result.status.success(), "{report}");
    assert_eq!(report["status"], "unknown");
    assert_eq!(
        report["errors"][0]["code"],
        "recorder_finalization_uncommitted"
    );
}
