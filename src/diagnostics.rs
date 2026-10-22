//! Persisted run evidence: manifest, log, events, failure snapshots, physics/input traces.
//!
//! Every run writes to `artifacts/runs/<id>/`. The directory is append-only from the
//! caller's perspective; a run never rewrites another run's evidence.
use bevy::{
    app::AppExit,
    log::{BoxedLayer, tracing_subscriber},
    prelude::*,
    render::renderer::RenderAdapterInfo,
    window::PrimaryWindow,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashSet, VecDeque},
    env,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

static LOG_FILE: OnceLock<PathBuf> = OnceLock::new();
static RUN_DIR: OnceLock<PathBuf> = OnceLock::new();
static RUN_RECORDER: OnceLock<Arc<RunRecorder>> = OnceLock::new();

pub fn set_run_recorder(run: Arc<RunRecorder>) {
    let _ = RUN_RECORDER.set(run);
}

pub fn run_recorder() -> Option<Arc<RunRecorder>> {
    RUN_RECORDER.get().cloned()
}

pub fn set_log_file(path: PathBuf) {
    let _ = LOG_FILE.set(path);
}

/// Extra tracing layer that writes every log event to the run's `run.log`.
pub fn file_log_layer(_app: &mut App) -> Option<BoxedLayer> {
    let path = LOG_FILE.get()?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()?;
    Some(Box::new(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(Mutex::new(file)),
    ))
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(dir) = RUN_DIR.get() {
            let message = info.to_string();
            let location = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_default();
            let backtrace = std::backtrace::Backtrace::force_capture().to_string();
            let payload = json!({
                "schema": 1,
                "at": now(),
                "reason": "panic",
                "message": message,
                "location": location,
                "backtrace": backtrace,
            });
            write_json_atomic(&dir.join("panic.json"), &payload);
            let failure = dir.join("failure.json");
            if !failure.exists() {
                write_json_atomic(&failure, &payload);
            }
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
    run_id: String,
    root: PathBuf,
    dir: PathBuf,
    manifest: Mutex<Manifest>,
    events: Mutex<BufWriter<File>>,
    recent: Mutex<VecDeque<Value>>,
    failure_written: AtomicBool,
    seq: AtomicU64,
}

impl RunRecorder {
    pub fn create(mode: &str, car: &str) -> Self {
        Self::create_in(Path::new("artifacts/runs"), mode, car)
    }

    pub fn create_in(root: &Path, mode: &str, car: &str) -> Self {
        let started_at = now();
        let git = git_info();
        let short = git.short.clone();
        let compact = compact_timestamp(&started_at);
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let run_id = format!("{}-{}-{}-{}", compact, short, sanitize(mode), suffix);
        let dir = root.join(&run_id);
        fs::create_dir_all(dir.join("captures")).expect("create run captures directory");
        fs::create_dir_all(dir.join("cars")).expect("create run cars directory");
        let _ = File::create(dir.join("run.log"));

        let manifest = Manifest {
            schema: 1,
            run_id: run_id.clone(),
            started_at,
            ended_at: None,
            git: git.clone(),
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
        };
        let events = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("events.jsonl"))
            .expect("create events file");
        let recorder = Self {
            run_id: run_id.clone(),
            root: root.to_path_buf(),
            dir: dir.clone(),
            manifest: Mutex::new(manifest),
            events: Mutex::new(BufWriter::new(events)),
            recent: Mutex::new(VecDeque::new()),
            failure_written: AtomicBool::new(false),
            seq: AtomicU64::new(1),
        };
        let _ = RUN_DIR.set(dir.clone());
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
        if root == Path::new("artifacts/runs") {
            let _ = fs::write("artifacts/latest", recorder.dir.display().to_string());
        }
        recorder.event(
            "run.start",
            json!({"mode": mode, "car": car, "dir": recorder.dir.display().to_string()}),
        );
        recorder
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn event(&self, kind: &str, fields: Value) {
        let record = json!({
            "ts": now(),
            "run_id": self.run_id,
            "seq": self.seq.fetch_add(1, Ordering::Relaxed),
            "kind": kind,
            "fields": fields,
        });
        if let Ok(mut events) = self.events.lock() {
            let _ = writeln!(events, "{}", record);
            let _ = events.flush();
        }
        if let Ok(mut recent) = self.recent.lock() {
            if recent.len() >= 200 {
                recent.pop_front();
            }
            recent.push_back(record);
        }
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
        write_json_atomic(&self.dir.join("report.json"), report);
        self.event("report.write", json!({"path": "report.json"}));
    }

    pub fn record_car_report(&self, id: &str, report: &Value) {
        write_json_atomic(&self.dir.join("cars").join(format!("{id}.json")), report);
        self.event(
            "car.report",
            json!({"car": id, "path": format!("cars/{id}.json")}),
        );
    }

    pub fn record_capture_metrics(&self, id: &str, view: &str, metrics: &Value) {
        write_json_atomic(
            &self.dir.join("captures").join(format!("{id}-{view}.json")),
            metrics,
        );
    }

    /// Write a failure snapshot exactly once. Later failures do not overwrite the first.
    pub fn fail(&self, reason: &str, details: Value) {
        if self.failure_written.swap(true, Ordering::SeqCst) {
            return;
        }
        let recent = self
            .recent
            .lock()
            .map(|events| events.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
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
        write_json_atomic(&self.dir.join("failure.json"), &payload);
        self.event("run.failure", json!({"reason": reason}));
    }

    pub fn finish(&self, exit: AppExit) {
        let (code, reason) = match exit {
            AppExit::Success => (0, "success"),
            AppExit::Error(code) => (code.get() as i32, "error"),
        };
        if let Ok(mut manifest) = self.manifest.lock() {
            manifest.ended_at = Some(now());
            manifest.exit = Some(ExitInfo {
                code,
                reason: reason.to_string(),
            });
        }
        self.write_manifest();
        self.event("run.exit", json!({"code": code, "reason": reason}));
        let manifest = self.manifest.lock().ok().map(|m| m.clone());
        self.append_index(json!({
            "event": "finish",
            "run_id": self.run_id,
            "ended_at": manifest.as_ref().and_then(|m| m.ended_at.clone()),
            "exit_code": code,
            "exit_reason": reason,
        }));
    }

    fn write_manifest(&self) {
        let manifest = self.manifest.lock().ok().map(|m| m.clone());
        if let Some(manifest) = manifest {
            write_json_atomic(&self.dir.join("manifest.json"), &manifest);
        }
    }

    fn append_index(&self, record: Value) {
        let path = self.root.join("index.jsonl");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{}", record);
        }
    }
}

#[derive(Resource)]
pub struct PhysicsTrace {
    physics: BufWriter<File>,
    input: BufWriter<File>,
    ring: VecDeque<PhysicsSample>,
    capacity: usize,
    step: u64,
}

impl PhysicsTrace {
    pub fn new(run_dir: &Path) -> Self {
        let physics = OpenOptions::new()
            .create(true)
            .append(true)
            .open(run_dir.join("physics.jsonl"))
            .expect("create physics trace");
        let input = OpenOptions::new()
            .create(true)
            .append(true)
            .open(run_dir.join("input.jsonl"))
            .expect("create input trace");
        Self {
            physics: BufWriter::new(physics),
            input: BufWriter::new(input),
            ring: VecDeque::with_capacity(1200),
            capacity: 1200,
            step: 0,
        }
    }

    pub fn record(&mut self, sample: PhysicsSample) {
        if let Ok(line) = serde_json::to_string(&sample) {
            let _ = writeln!(self.physics, "{line}");
        }
        let input = InputSample::from(&sample);
        if let Ok(line) = serde_json::to_string(&input) {
            let _ = writeln!(self.input, "{line}");
        }
        if self.ring.len() >= self.capacity {
            self.ring.pop_front();
        }
        self.ring.push_back(sample);
        self.step += 1;
        if self.step.is_multiple_of(60) {
            self.flush();
        }
    }

    pub fn flush(&mut self) {
        let _ = self.physics.flush();
        let _ = self.input.flush();
    }

    pub fn step(&self) -> u64 {
        self.step
    }

    #[allow(dead_code)]
    pub fn tail(&self, count: usize) -> Vec<PhysicsSample> {
        self.ring
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect::<Vec<_>>()
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
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
    pub handbrake: bool,
    pub reset: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct PhysicsSample {
    pub step: u64,
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

    pub fn next(&mut self) -> Option<InputSample> {
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

fn now() -> String {
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
    let bytes = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Some(format!("{:x}", hasher.finalize()))
}

fn write_json_atomic(path: &Path, value: &impl Serialize) {
    let Ok(bytes) = serde_json::to_vec_pretty(value) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let temp = path.with_extension("tmp");
    if fs::write(&temp, bytes).is_ok() {
        let _ = fs::rename(&temp, path);
    }
}

fn tail_lines(path: &Path, count: usize) -> Vec<String> {
    tail_lines_result(path, count).unwrap_or_default()
}

fn tail_lines_result(path: &Path, count: usize) -> std::io::Result<Vec<String>> {
    let text = fs::read_to_string(path)?;
    let lines: Vec<&str> = text.lines().collect();
    Ok(lines[lines.len().saturating_sub(count)..]
        .iter()
        .map(|line| (*line).to_string())
        .collect())
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
    gamepads: Query<(Entity, &Name, &Gamepad)>,
    mut recorded: Local<bool>,
    mut seen_pads: Local<HashSet<Entity>>,
) {
    for (entity, name, pad) in &gamepads {
        if seen_pads.insert(entity) {
            run.event(
                "gamepad.connected",
                json!({
                    "entity": entity.to_bits(),
                    "name": name.as_str(),
                    "vendor_id": pad.vendor_id(),
                    "product_id": pad.product_id(),
                }),
            );
        }
    }
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
        assert_eq!(replay.next().unwrap().throttle, 1.0);
        assert!(!replay.active);
        assert!(replay.next().is_none());
        let _ = fs::remove_file(path);
    }
}
