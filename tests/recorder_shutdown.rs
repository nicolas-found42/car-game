#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::{Duration, Instant},
};

// Give the panic/export control an uncontended CPU/IO budget. The other
// independent process controls can overlap; all original deadlines remain.
static PROCESS_LEASE: RwLock<()> = RwLock::new(());
enum ProcessLease {
    Shared {
        _guard: RwLockReadGuard<'static, ()>,
    },
    Exclusive {
        _guard: RwLockWriteGuard<'static, ()>,
    },
}

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

struct RecordingProcess {
    _process_lease: ProcessLease,
    child: Child,
    root: PathBuf,
    run: Option<PathBuf>,
}
impl RecordingProcess {
    fn spawn(paused: bool, extra: &[&str]) -> Self {
        Self::spawn_until(paused, extra, 9999)
    }
    fn spawn_until(paused: bool, extra: &[&str], tick: u64) -> Self {
        Self::spawn_guarded(paused, extra, tick, false)
    }
    fn spawn_guarded(paused: bool, extra: &[&str], tick: u64, exclusive: bool) -> Self {
        let process_lease = if exclusive {
            ProcessLease::Exclusive {
                _guard: PROCESS_LEASE
                    .write()
                    .unwrap_or_else(|error| error.into_inner()),
            }
        } else {
            ProcessLease::Shared {
                _guard: PROCESS_LEASE
                    .read()
                    .unwrap_or_else(|error| error.into_inner()),
            }
        };
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
            _process_lease: process_lease,
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
            retain_failed_evidence(&self.root, self.run.as_deref());
        } else {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

// CI uploads artifacts/logs, while temporary child directories disappear with
// the runner. Keep bounded raw diagnostics; missing/oversized files stay explicit.
fn retain_failed_evidence(root: &Path, run: Option<&Path>) {
    let destination = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("artifacts/logs")
        .join(root.file_name().unwrap_or_default());
    if fs::create_dir_all(&destination).is_err() {
        return;
    }
    let run = run.map(Path::to_path_buf).or_else(|| {
        fs::read_dir(root.join("runs"))
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.is_dir())
    });
    let mut inventory = Vec::new();
    for (group, source, names) in [
        (
            "process",
            Some(root),
            &["stdout.log", "stderr.log", "scenario.json"][..],
        ),
        (
            "run",
            run.as_deref(),
            &[
                "manifest.json",
                "recorder-health.json",
                "failure.json",
                "panic.json",
                "run.log",
                "scenario-summary.json",
                "incident-history.jsonl",
            ][..],
        ),
    ] {
        let _ = fs::create_dir_all(destination.join(group));
        for name in names {
            let file = format!("{group}/{name}");
            let metadata = source.and_then(|source| fs::metadata(source.join(name)).ok());
            let Some(metadata) = metadata else {
                inventory.push(json!({"file":file,"unavailable":true}));
                continue;
            };
            let copied = metadata.is_file()
                && metadata.len() <= 2 * 1024 * 1024
                && source.is_some_and(|source| {
                    fs::copy(source.join(name), destination.join(&file)).is_ok()
                });
            inventory.push(json!({"file":file,"bytes":metadata.len(),"copied":copied}));
        }
    }
    let _ = fs::write(
        destination.join("inventory.json"),
        json!(inventory).to_string(),
    );
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
    let mut process = RecordingProcess::spawn_guarded(
        false,
        &[
            "--recorder-fault",
            "panic:200",
            "--recorder-writer-delay-ms",
            "2",
        ],
        9999,
        true,
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
