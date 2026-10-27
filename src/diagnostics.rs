//! Persisted run evidence: manifest, log, events, failure snapshots, physics/input traces.
//!
//! Every run writes to `artifacts/runs/<id>/`. The directory is append-only from the
//! caller's perspective; a run never rewrites another run's evidence.
mod automatic;
pub(crate) use automatic::observe_clocks;
pub mod decision;
mod health;
mod history;
mod incident;
mod logging;
mod recording;
mod windows;
pub(crate) use recording::bounded_json;
mod settings;
mod shutdown;
pub use shutdown::install_termination_handler;
pub(crate) use shutdown::poll_lifecycle;
mod writer;
pub use health::RecorderError;
pub use recording::EvidenceStream;
pub use settings::{RecorderConfig, RecorderFault, RecorderFaultStage};

use bevy::{
    app::AppExit, log::BoxedLayer, prelude::*, render::renderer::RenderAdapterInfo,
    window::PrimaryWindow,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

static RUN_COUNTER: AtomicU64 = AtomicU64::new(1);
static RUN_RECORDER: OnceLock<Arc<RunRecorder>> = OnceLock::new();

pub fn set_run_recorder(run: Arc<RunRecorder>) {
    let _ = RUN_RECORDER.set(run);
}

pub fn run_recorder() -> Option<Arc<RunRecorder>> {
    RUN_RECORDER.get().cloned()
}

/// Logging uses the same checked, bounded writer as the numeric evidence streams.
pub fn file_log_layer(app: &mut App) -> Option<BoxedLayer> {
    let run = app
        .world()
        .get_resource::<RunHandle>()
        .map(|run| run.0.clone())
        .or_else(run_recorder)?;
    Some(logging::layer(run))
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(run) = RUN_RECORDER.get() {
            run.health.request_panic(info);
        }
        previous(info);
    }));
}

#[derive(Serialize, Clone)]
pub struct GitInfo {
    pub sha: String,
    pub short: String,
    pub branch: String,
    pub dirty: bool,
}

#[derive(Serialize, Clone)]
pub struct EnvInfo {
    pub os: String,
    pub arch: String,
    pub rustc: String,
    pub tool_versions_role: &'static str,
    pub cargo: String,
    pub bevy: String,
    pub rapier: String,
    pub rust_log: String,
    pub cwd: String,
    pub assets_root: String,
    pub source_root: String,
}

#[derive(Serialize, Clone)]
pub struct ExitInfo {
    pub code: i32,
    pub reason: String,
}

#[derive(Serialize, Clone)]
pub struct Manifest {
    pub provenance: Option<Value>,
    pub recorder_health: Value,
    pub schema: u32,
    pub run_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub git: GitInfo,
    pub binary_sha256: String,
    pub args: Vec<String>,
    pub mode: String,
    pub car: String,
    pub env: EnvInfo,
    pub gpu: Option<Value>,
    pub window: Option<Value>,
    pub exit: Option<ExitInfo>,
}

impl Manifest {
    fn new(run_id: String, started_at: String, git: GitInfo, mode: &str, car: &str) -> Self {
        Self {
            provenance: None,
            recorder_health: json!({"schema":1,"path":"recorder-health.json","required":true}),
            schema: 1,
            run_id,
            started_at,
            ended_at: None,
            git,
            binary_sha256: env::current_exe()
                .ok()
                .as_deref()
                .and_then(sha256_file)
                .unwrap_or_else(|| "unknown".to_string()),
            args: env::args().skip(1).collect(),
            mode: mode.to_string(),
            car: car.to_string(),
            env: EnvInfo {
                os: env::consts::OS.to_string(),
                arch: env::consts::ARCH.to_string(),
                rustc: command_version("rustc", &["-V"]),
                tool_versions_role: "runtime_observation_not_binary_build_identity",
                cargo: command_version("cargo", &["-V"]),
                bevy: "0.20".to_string(),
                rapier: "0.37".to_string(),
                rust_log: env::var("RUST_LOG").unwrap_or_default(),
                cwd: env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                assets_root: env::current_dir()
                    .map(|p| p.join("assets").display().to_string())
                    .unwrap_or_default(),
                source_root: env::current_dir()
                    .map(|p| p.join("../carmodels").display().to_string())
                    .unwrap_or_default(),
            },
            gpu: None,
            window: None,
            exit: None,
        }
    }
}

#[derive(Resource, Clone)]
pub struct RunHandle(pub Arc<RunRecorder>);

impl std::ops::Deref for RunHandle {
    type Target = RunRecorder;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Resource)]
pub struct RunRecorder {
    monitor: Mutex<std::collections::BTreeMap<String, automatic::Monitor>>,
    windows: Mutex<windows::Windows>,
    run_id: String,
    root: PathBuf,
    dir: PathBuf,
    manifest: Mutex<Manifest>,
    health: std::sync::Arc<health::HealthState>,
    recording: recording::Recording,
    finished: AtomicBool,
    final_exit: AtomicI32,
    failure_written: AtomicBool,
    seq: AtomicU64,
    pub evidence_clock: crate::telemetry::EvidenceClock,
}

impl RunRecorder {
    pub fn create(mode: &str, car: &str) -> Self {
        Self::create_in(Path::new("artifacts/runs"), mode, car)
    }

    pub fn create_in(root: &Path, mode: &str, car: &str) -> Self {
        Self::try_create_in(root, mode, car).expect("create run recorder")
    }

    pub fn try_create_in(root: &Path, mode: &str, car: &str) -> Result<Self, RecorderError> {
        Self::try_create_with_config(root, mode, car, RecorderConfig::default())
    }

    pub fn try_create_with_config(
        root: &Path,
        mode: &str,
        car: &str,
        config: RecorderConfig,
    ) -> Result<Self, RecorderError> {
        config.validate()?;
        let started_at = now();
        let git = git_info();
        let short = git.short.clone();
        let compact = compact_timestamp(&started_at);
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            RUN_COUNTER.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let run_id = format!("{}-{}-{}-{}", compact, short, sanitize(mode), suffix);
        let dir = root.join(&run_id);
        fs::create_dir_all(dir.join("captures"))
            .map_err(|e| RecorderError::new("open", "captures", e.kind()))?;
        fs::create_dir_all(dir.join("cars"))
            .map_err(|e| RecorderError::new("open", "cars", e.kind()))?;
        File::create(dir.join("run.log"))
            .map_err(|e| RecorderError::new("open", "log", e.kind()))?;

        let manifest = Manifest::new(run_id.clone(), started_at, git.clone(), mode, car);
        let health = std::sync::Arc::new(health::HealthState::new(dir.clone(), config));
        let recording = recording::Recording::open(health.clone())?;
        let recorder = Self {
            monitor: Mutex::new(std::collections::BTreeMap::new()),
            windows: Mutex::new(windows::Windows::default()),
            run_id: run_id.clone(),
            root: root.to_path_buf(),
            dir: dir.clone(),
            manifest: Mutex::new(manifest),
            health,
            recording,
            finished: AtomicBool::new(false),
            final_exit: AtomicI32::new(1),
            failure_written: AtomicBool::new(false),
            seq: AtomicU64::new(1),
            evidence_clock: crate::telemetry::EvidenceClock::default(),
        };
        recorder.write_manifest();
        recorder.append_index(json!({
            "event": "start",
            "run_id": run_id,
            "started_at": recorder.manifest.lock().map(|m| m.started_at.clone()).unwrap_or_default(),
            "mode": mode,
            "car": car,
            "git_sha": git.sha,
            "dirty": git.dirty,
            "dir": dir.display().to_string(),
        }));
        if root == Path::new("artifacts/runs")
            && let Err(error) = fs::write("artifacts/latest", recorder.dir.display().to_string())
        {
            recorder.health.error("publish", "latest", error.kind());
        }
        recorder.event(
            "run.start",
            json!({"mode": mode, "car": car, "dir": recorder.dir.display().to_string()}),
        );
        recorder.health.persist();
        if recorder.health.failed() {
            return Err(RecorderError::new(
                "publish",
                "startup",
                "initial evidence unavailable",
            ));
        }
        Ok(recorder)
    }

    /// Freeze identities before the app can load or mutate its effective inputs.
    pub fn capture_provenance(&self, assets: &Path, root_kind: &str, config: Value) {
        match crate::provenance::capture(&self.dir, &self.run_id, assets, root_kind, config) {
            Ok(reference) => {
                if let Ok(mut manifest) = self.manifest.lock() {
                    manifest.env.assets_root = assets.display().to_string();
                    manifest.provenance = Some(reference);
                }
                self.write_manifest();
            }
            Err(error) => {
                self.health.error("capture", "provenance", error.kind());
                if let Ok(mut manifest) = self.manifest.lock()
                    && manifest.provenance.is_none()
                {
                    manifest.provenance =
                        Some(json!({"availability": "unavailable", "reason": "capture_failed"}));
                }
                self.write_manifest();
                self.event(
                    "provenance.capture_failed",
                    json!({"reason": "capture_unavailable"}),
                );
            }
        }
    }

    /// Checked serialization and nonblocking bounded recording boundary.
    pub fn record_evidence(
        &self,
        stream: EvidenceStream,
        tick: u64,
        payload: &impl Serialize,
    ) -> bool {
        self.recording.record(stream, tick, payload)
    }
    pub fn flush(&self) -> bool {
        self.recording.flush(false)
    }
    pub(crate) fn recording_failed(&self) -> bool {
        self.health.failed()
    }
    pub(crate) fn signal_support(&self) {
        self.health.update(|data| data.signal_support = true);
    }
    pub(crate) fn terminate(&self) {
        let first = self.health.update(|data| {
            if data.shutdown_reason == "termination_requested" {
                return false;
            }
            data.shutdown_reason = "termination_requested".into();
            true
        });
        if first {
            self.event(
                "run.termination_requested",
                json!({"signal_identity":"not_available"}),
            );
            self.recording.incident("termination_requested");
        }
    }
    pub fn recorder_config(&self) -> &RecorderConfig {
        &self.health.config
    }
    pub fn health(&self) -> Value {
        self.health.snapshot()
    }
    pub fn completed_tick(&self, tick: u64) {
        self.evidence_clock.completed(tick);
        self.recording.completed(tick);
        self.poll_window(tick, false);
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn event(&self, kind: &str, fields: Value) {
        self.event_envelope(kind, fields, self.envelope());
    }

    pub fn actor_event(&self, actor_id: &str, kind: &str, fields: Value) {
        let mut envelope = self.envelope();
        envelope.actor_id = Some(actor_id.into());
        self.event_envelope(kind, fields, envelope);
    }

    pub fn correlated_event(
        &self,
        actor_id: &str,
        correlation_id: &str,
        kind: &str,
        fields: Value,
    ) {
        let mut envelope = self.envelope();
        envelope.actor_id = Some(actor_id.into());
        envelope.correlation_id = Some(correlation_id.into());
        self.event_envelope(kind, fields, envelope);
    }

    pub fn envelope(&self) -> crate::telemetry::Envelope {
        self.evidence_clock.envelope(&self.run_id)
    }

    pub fn command(&self, envelope: &crate::telemetry::Envelope, command: &Value, source: &str) {
        let mut record = serde_json::to_value(envelope).expect("serializable envelope");
        record["command"] = command.clone();
        record["source"] = source.into();
        record["phase"] = "ingress".into();
        self.record_evidence(EvidenceStream::Commands, envelope.tick, &record);
    }

    pub fn event_envelope(&self, kind: &str, fields: Value, envelope: crate::telemetry::Envelope) {
        let mut record = serde_json::to_value(&envelope).expect("serializable envelope");
        let legacy = json!({
            "ts": now(),
            "run_id": self.run_id,
            "seq": self.seq.fetch_add(1, Ordering::Relaxed),
            "kind": kind,
            "fields": fields,
        });
        record
            .as_object_mut()
            .expect("envelope object")
            .extend(legacy.as_object().expect("legacy object").clone());
        self.record_evidence(EvidenceStream::Events, envelope.tick, &record);
    }

    pub fn set_gpu(&self, gpu: Value) {
        if let Ok(mut manifest) = self.manifest.lock() {
            manifest.gpu = Some(gpu.clone());
        }
        self.write_manifest();
        self.event("environment.gpu", gpu);
    }

    pub fn set_window(&self, window: Value) {
        if let Ok(mut manifest) = self.manifest.lock() {
            manifest.window = Some(window.clone());
        }
        self.write_manifest();
        self.event("environment.window", window);
    }

    pub fn record_report(&self, report: &Value) {
        if self
            .health
            .write_json(&self.dir.join("report.json"), report)
        {
            self.event("report.write", json!({"path": "report.json"}));
        }
    }

    pub fn record_car_report(&self, id: &str, report: &Value) {
        if !self
            .health
            .write_json(&self.dir.join("cars").join(format!("{id}.json")), report)
        {
            return;
        }
        self.actor_event(
            "player-vehicle",
            "car.report",
            json!({"car": id, "path": format!("cars/{id}.json")}),
        );
    }

    pub fn record_capture_metrics(&self, id: &str, view: &str, metrics: &Value) {
        self.health.write_json(
            &self.dir.join("captures").join(format!("{id}-{view}.json")),
            metrics,
        );
    }

    /// Write a failure snapshot exactly once. Later failures do not overwrite the first.
    pub fn fail(&self, reason: &str, details: Value) {
        if self.failure_written.swap(true, Ordering::SeqCst) {
            return;
        }
        self.event("run.failure", json!({"reason":reason}));
        let recent = self
            .health
            .history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .recent_events();
        let details = match recording::bounded_json(&details, 256 * 1024, false) {
            Ok(bytes) => serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null),
            Err(_) => {
                self.health.error("oversize", "incident", "details_omitted");
                json!({"availability":"omitted","reason":"details_exceed_256KiB"})
            }
        };
        let manifest = self.manifest.lock().ok().map(|m| m.clone());
        let payload = json!({
            "schema": 1,
            "run_id": self.run_id,
            "at": now(),
            "reason": reason,
            "details": details,
            "manifest": manifest,
            "recent_events": recent,
            "log_tail": tail_lines(&self.dir.join("run.log"), 80),
            "physics_tail": tail_lines(&self.dir.join("physics.jsonl"), 120),
        });
        if !self
            .health
            .write_json(&self.dir.join("failure.json"), &payload)
        {
            self.failure_written.store(false, Ordering::Release);
        }
        self.recording.incident(reason);
    }

    pub fn finish(&self, exit: AppExit) -> AppExit {
        if self.finished.swap(true, Ordering::AcqRel) {
            return exit_from_code(if self.health.failed() {
                1
            } else {
                self.final_exit.load(Ordering::Acquire)
            });
        }
        self.flush();
        let requested_code = match exit {
            AppExit::Success => 0,
            AppExit::Error(code) => i32::from(code.get()),
        };
        self.event(
            "run.exit",
            json!({"code":requested_code,"reason":if requested_code==0 {"success"} else {"error"}}),
        );
        self.poll_window(self.envelope().tick, true);
        let drained = self.recording.flush(true);
        self.health.write_json(
            &self.dir.join("diagnostic-windows.json"),
            &self.window_status(),
        );
        let mut code = if self.health.failed() || self.failure_written.load(Ordering::Acquire) {
            1
        } else {
            requested_code
        };
        if let Ok(mut manifest) = self.manifest.lock() {
            manifest.ended_at = Some(now());
            manifest.exit = Some(ExitInfo {
                code,
                reason: if code == 0 {
                    "success".into()
                } else {
                    "error".into()
                },
            });
        }
        self.write_manifest();
        self.append_index(json!({"event":"finish","run_id":self.run_id,"exit_code":code}));
        self.health.update(|data| {
            data.phase = if self.health.failed() || !drained {
                "incomplete".into()
            } else {
                "finalized".into()
            }
        });
        self.health.persist();
        if self.health.failed() {
            code = 1;
            self.update_exit_error();
        }
        if drained
            && let Err(error) = crate::provenance::finalize_bounded(
                &self.dir,
                self.health.config.finalize_max_bytes,
            )
        {
            self.health.error("finalize", "provenance", error);
            self.health.update(|data| data.phase = "incomplete".into());
            self.health.persist();
            self.update_exit_error();
            code = 1;
        }
        self.final_exit.store(code, Ordering::Release);
        exit_from_code(code)
    }
    fn update_exit_error(&self) {
        if let Ok(mut manifest) = self.manifest.lock() {
            manifest.exit = Some(ExitInfo {
                code: 1,
                reason: "error".into(),
            });
        }
        self.write_manifest();
    }

    fn write_manifest(&self) {
        let manifest = self.manifest.lock().ok().map(|m| m.clone());
        if let Some(manifest) = manifest {
            self.health
                .write_json(&self.dir.join("manifest.json"), &manifest);
        }
    }

    fn append_index(&self, record: Value) {
        let path = self.root.join("index.jsonl");
        let result = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                writeln!(file, "{record}")?;
                file.sync_all()
            });
        if let Err(error) = result {
            self.health.error("publish", "run_index", error.kind());
        }
    }
}

fn exit_from_code(code: i32) -> AppExit {
    if code == 0 {
        AppExit::Success
    } else {
        AppExit::Error(std::num::NonZeroU8::new(code as u8).unwrap_or(std::num::NonZeroU8::MIN))
    }
}

#[derive(Resource)]
pub struct PhysicsTrace {
    run: Arc<RunRecorder>,
    step: u64,
}
impl PhysicsTrace {
    pub fn for_recorder(run: Arc<RunRecorder>) -> Self {
        Self { run, step: 0 }
    }
    pub fn record(&mut self, sample: PhysicsSample) {
        self.run
            .record_evidence(EvidenceStream::Physics, sample.tick, &sample);
        self.step += 1;
    }
    pub fn record_input(&mut self, input: InputSample) {
        self.run
            .record_evidence(EvidenceStream::Input, input.tick, &input);
    }
    pub fn flush(&mut self) {
        if !self.run.recording.closed() {
            self.run.flush();
        }
    }
    pub fn step(&self) -> u64 {
        self.step
    }
}
impl Drop for PhysicsTrace {
    fn drop(&mut self) {
        self.flush();
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct InputSample {
    pub step: u64,
    #[serde(default)]
    pub tick: u64,
    #[serde(default)]
    pub controller: Option<crate::runtime::ControllerOwner>,
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
    pub handbrake: bool,
    pub reset: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct PhysicsSample {
    pub decisions: Option<decision::VehicleDecisions>,
    pub drive_demands: [f32; 4],
    pub longitudinal_forces: [f32; 4],
    pub envelope: Option<crate::telemetry::Envelope>,
    pub vehicle_id: crate::assembly::VehicleId,
    pub body_part_id: Option<crate::assembly::PartId>,
    pub backend_applied: Option<crate::physics::ForceSample>,
    pub body_id: String,
    pub boundary: &'static str,
    pub query_mode: &'static str,
    pub integration_steps: u32,
    pub evidence: crate::physics::StepEvidence,
    pub phase_durations: crate::physics::PhaseDurations,
    pub step: u64,
    pub tick: u64,
    pub actor_id: String,
    pub time: f32,
    pub car: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub grounded: usize,
    pub wheel_lengths: [f32; 4],
    pub wheel_loads: [f32; 4],
    pub steering: f32,
    pub speed: f32,
    pub slip: f32,
    pub rpm: f32,
    pub gear: i32,
    pub reset_count: u32,
    pub input: InputSample,
    pub ray_misses: usize,
}

impl From<&PhysicsSample> for InputSample {
    fn from(sample: &PhysicsSample) -> Self {
        InputSample {
            step: sample.step,
            tick: sample.tick,
            controller: sample.input.controller,
            throttle: sample.input.throttle,
            brake: sample.input.brake,
            steer: sample.input.steer,
            handbrake: sample.input.handbrake,
            reset: sample.input.reset,
        }
    }
}

#[derive(Resource, Default)]
pub struct ReplayInput {
    pub samples: Vec<InputSample>,
    pub index: usize,
    pub active: bool,
    pub path: Option<PathBuf>,
}

impl ReplayInput {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("read replay {}: {error}", path.display()))?;
        let mut samples = Vec::new();
        for (line_no, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let sample = serde_json::from_str::<InputSample>(line).map_err(|error| {
                format!("parse replay {}:{}: {error}", path.display(), line_no + 1)
            })?;
            samples.push(sample);
        }
        Ok(Self {
            active: !samples.is_empty(),
            samples,
            index: 0,
            path: Some(path.to_path_buf()),
        })
    }

    pub fn next_sample(&mut self) -> Option<InputSample> {
        let sample = self.samples.get(self.index).copied()?;
        self.index += 1;
        if self.index >= self.samples.len() {
            self.active = false;
        }
        Some(sample)
    }
}

pub fn replay_from_args() -> Option<ReplayInput> {
    let args: Vec<String> = env::args().collect();
    let index = args.iter().position(|arg| arg == "--replay-input")?;
    let path = args.get(index + 1).unwrap_or_else(|| {
        eprintln!("--replay-input requires a path");
        std::process::exit(2);
    });
    match ReplayInput::load(Path::new(path)) {
        Ok(replay) => Some(replay),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

pub fn diagnose_target() -> Option<Option<String>> {
    let args: Vec<String> = env::args().collect();
    let index = args.iter().position(|arg| arg == "--diagnose")?;
    Some(
        args.get(index + 1)
            .filter(|arg| !arg.starts_with("--"))
            .cloned(),
    )
}

pub fn print_diagnosis(target: Option<&str>) {
    let dir = resolve_run_dir(target);
    let Some(dir) = dir else {
        eprintln!("no run directory found; pass --diagnose <run-dir|run-id>");
        return;
    };
    println!("run: {}", dir.display());
    for name in ["manifest.json", "failure.json", "panic.json"] {
        let path = dir.join(name);
        if let Ok(text) = fs::read_to_string(&path) {
            println!("\n== {name} ==\n{text}");
        }
    }
    let report = dir.join("report.json");
    if let Ok(text) = fs::read_to_string(&report) {
        println!("\n== report.json (summary) ==\n{}", summarize_report(&text));
    }
    if let Ok(lines) = tail_lines_result(&dir.join("events.jsonl"), 20) {
        println!("\n== events.jsonl (last 20) ==\n{}", lines.join("\n"));
    }
    if let Ok(lines) = tail_lines_result(&dir.join("run.log"), 60) {
        println!("\n== run.log (last 60) ==\n{}", lines.join("\n"));
    }
    if let Ok(lines) = tail_lines_result(&dir.join("physics.jsonl"), 40) {
        println!("\n== physics.jsonl (last 40) ==\n{}", lines.join("\n"));
    }
    if let Ok(lines) = tail_lines_result(&dir.join("input.jsonl"), 40) {
        println!("\n== input.jsonl (last 40) ==\n{}", lines.join("\n"));
    }
}

fn resolve_run_dir(target: Option<&str>) -> Option<PathBuf> {
    if let Some(target) = target {
        let path = PathBuf::from(target);
        if path.is_dir() {
            return Some(path);
        }
        if path.is_file() {
            let value = fs::read_to_string(&path).ok()?;
            let pointed = PathBuf::from(value.trim());
            if pointed.is_dir() {
                return Some(pointed);
            }
        }
        let candidate = PathBuf::from("artifacts/runs").join(target);
        if candidate.is_dir() {
            return Some(candidate);
        }
        return None;
    }
    let latest = fs::read_to_string("artifacts/latest").ok()?;
    let path = PathBuf::from(latest.trim());
    path.is_dir().then_some(path)
}

fn summarize_report(text: &str) -> String {
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Array(reports)) => reports
            .iter()
            .map(|report| {
                format!(
                    "{} passed={} captures={} speed={:.1} fps={:.1}",
                    report.get("id").and_then(Value::as_str).unwrap_or("?"),
                    report
                        .get("passed")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    report
                        .get("nonblank_captures")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    report
                        .get("max_speed_kmh")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                    report
                        .get("average_fps")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => text.to_string(),
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CaptureMetrics {
    pub width: u32,
    pub height: u32,
    pub unique_colors: usize,
    pub luma_mean: f32,
    pub luma_stddev: f32,
    pub edge_count: usize,
    pub nonblank: bool,
}

pub fn capture_metrics(image: &Image) -> Option<CaptureMetrics> {
    let data = image.data.as_ref()?;
    Some(metrics_from_rgba(image.width(), image.height(), data))
}

pub fn metrics_from_rgba(width: u32, height: u32, data: &[u8]) -> CaptureMetrics {
    let pixels = data.len() / 4;
    if pixels == 0 || width == 0 || height == 0 {
        return CaptureMetrics {
            width,
            height,
            unique_colors: 0,
            luma_mean: 0.0,
            luma_stddev: 0.0,
            edge_count: 0,
            nonblank: false,
        };
    }
    let mut colors = HashSet::new();
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    let mut edges = 0_usize;
    let mut previous_row: Vec<f32> = vec![0.0; width as usize];
    let mut current_row: Vec<f32> = Vec::with_capacity(width as usize);
    for (index, pixel) in data.as_chunks::<4>().0.iter().enumerate() {
        if colors.len() < 4096 {
            colors.insert(u32::from_le_bytes([pixel[0], pixel[1], pixel[2], 0]));
        }
        let luma = 0.2126 * pixel[0] as f32 + 0.7152 * pixel[1] as f32 + 0.0722 * pixel[2] as f32;
        sum += luma as f64;
        sum_sq += (luma as f64) * (luma as f64);
        let x = index % width as usize;
        let y = index / width as usize;
        if x > 0 {
            let left = (index - 1) * 4;
            let left_luma = 0.2126 * data[left] as f32
                + 0.7152 * data[left + 1] as f32
                + 0.0722 * data[left + 2] as f32;
            if (luma - left_luma).abs() > 12.0 {
                edges += 1;
            }
        }
        if y > 0 && (luma - previous_row[x]).abs() > 12.0 {
            edges += 1;
        }
        current_row.push(luma);
        if x + 1 == width as usize {
            previous_row = std::mem::take(&mut current_row);
        }
    }
    let count = pixels as f64;
    let mean = sum / count;
    let variance = (sum_sq / count - mean * mean).max(0.0);
    let stddev = variance.sqrt() as f32;
    let unique_colors = colors.len();
    CaptureMetrics {
        width,
        height,
        unique_colors,
        luma_mean: mean as f32,
        luma_stddev: stddev,
        edge_count: edges,
        nonblank: unique_colors > 1 && stddev > 1.0,
    }
}

pub(crate) fn now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}

fn compact_timestamp(timestamp: &str) -> String {
    timestamp
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == 'T' || *c == 'Z')
        .collect()
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn git_info() -> GitInfo {
    let sha = git_output(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let short =
        git_output(&["rev-parse", "--short=7", "HEAD"]).unwrap_or_else(|| "nogit".to_string());
    let branch =
        git_output(&["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let dirty = git_output(&["status", "--porcelain"])
        .map(|status| !status.is_empty())
        .unwrap_or(false);
    GitInfo {
        sha,
        short,
        branch,
        dirty,
    }
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn command_version(command: &str, args: &[&str]) -> String {
    Command::new(command)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn sha256_file(path: &Path) -> Option<String> {
    crate::provenance::hash_file(path).ok().map(|(sha, _)| sha)
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> std::io::Result<()> {
    let bytes = recording::bounded_json(value, 2 * 1024 * 1024, true)
        .map_err(|code| std::io::Error::new(std::io::ErrorKind::InvalidData, code))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("tmp");
    let mut file = File::create(&temp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    fs::rename(&temp, path)?;
    Ok(())
}

fn tail_lines(path: &Path, count: usize) -> Vec<String> {
    tail_lines_result(path, count).unwrap_or_default()
}

fn tail_lines_result(path: &Path, count: usize) -> std::io::Result<Vec<String>> {
    const MAX_BYTES: u64 = 256 * 1024;
    let mut file = File::open(path)?;
    let start = file.metadata()?.len().saturating_sub(MAX_BYTES);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES).read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let text = if start > 0 {
        text.split_once('\n').map_or("", |(_, tail)| tail)
    } else {
        &text
    };
    let mut lines: Vec<_> = text.lines().rev().take(count).map(str::to_owned).collect();
    lines.reverse();
    Ok(lines)
}

pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, record_environment);
    }
}

fn record_environment(
    run: Res<RunHandle>,
    adapter: Option<Res<RenderAdapterInfo>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut recorded: Local<bool>,
) {
    if *recorded {
        return;
    }
    let Some(adapter) = adapter else {
        return;
    };
    let info = &**adapter;
    run.set_gpu(json!({
        "name": info.name,
        "vendor": info.vendor,
        "device": info.device,
        "device_type": format!("{:?}", info.device_type),
        "driver": info.driver,
        "driver_info": info.driver_info,
        "backend": format!("{:?}", info.backend),
    }));
    if let Ok(window) = windows.single() {
        run.set_window(json!({
            "available": true,
            "width": window.resolution.width(),
            "height": window.resolution.height(),
            "scale_factor": window.resolution.scale_factor(),
            "present_mode": format!("{:?}", window.present_mode),
            "mode": format!("{:?}", window.mode),
            "focused": window.focused,
        }));
    } else {
        run.set_window(json!({"available": false, "offscreen": true}));
    }
    *recorded = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_captures_are_rejected_and_content_is_measured() {
        let blank = metrics_from_rgba(2, 1, &[0, 0, 0, 255, 0, 0, 0, 255]);
        assert!(!blank.nonblank);
        let flat_color = metrics_from_rgba(2, 1, &[255, 0, 0, 255, 255, 0, 0, 255]);
        assert!(!flat_color.nonblank);
        let content = metrics_from_rgba(2, 1, &[20, 30, 40, 255, 80, 90, 100, 255]);
        assert!(content.nonblank);
        assert_eq!(content.unique_colors, 2);
        assert!(content.luma_stddev > 1.0);
    }

    #[test]
    fn run_recorder_persists_failure_report_and_manifest() {
        let root = env::temp_dir().join(format!(
            "grid-diagnostics-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let run = RunRecorder::create_in(&root, "test", "350z");
        run.event("stage.enter", json!({"stage": 1}));
        run.record_report(&json!([{"id": "350z", "passed": true}]));
        run.fail("capture_timeout", json!({"view": "rear"}));
        run.finish(AppExit::error());
        assert!(run.dir().join("events.jsonl").is_file());
        assert!(run.dir().join("report.json").is_file());
        assert!(run.dir().join("failure.json").is_file());
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(run.dir().join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["exit"]["reason"], "error");
        assert_eq!(manifest["schema"], 1);
        let failure: Value =
            serde_json::from_str(&fs::read_to_string(run.dir().join("failure.json")).unwrap())
                .unwrap();
        assert_eq!(failure["reason"], "capture_timeout");
        assert!(failure["recent_events"].as_array().unwrap().len() >= 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn replay_reads_recorded_input_samples() {
        let path = env::temp_dir().join(format!(
            "grid-replay-test-{}-{}.jsonl",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            "{\"step\":0,\"throttle\":1.0,\"brake\":0.0,\"steer\":0.0,\"handbrake\":false,\"reset\":false}\n",
        )
        .unwrap();
        let mut replay = ReplayInput::load(&path).unwrap();
        assert!(replay.active);
        assert_eq!(replay.next_sample().unwrap().throttle, 1.0);
        assert!(!replay.active);
        assert!(replay.next_sample().is_none());
        let _ = fs::remove_file(path);
    }
}
