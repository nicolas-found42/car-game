//! Capture-time content identity for the existing recorder; no simulation/save state.
mod binding;
mod policy;
mod source;
mod store;

pub(crate) use store::hash_file;

use serde_json::{Value, json};
use std::{env, fs, io, path::Path};
use store::Entry;

pub fn capture(
    run: &Path,
    run_id: &str,
    assets: &Path,
    root_kind: &str,
    config: Value,
) -> io::Result<Value> {
    let capture_path = run.join("provenance/capture.json");
    if capture_path.exists() {
        return Err(io::Error::other("provenance_already_captured"));
    }
    let mut entries = Vec::new();
    let mut omissions = Vec::new();
    let executable = env::current_exe()?;
    let binary = store::capture_file(run, &executable, "executable", "bin/car-game")?;
    let build = source::build_snapshot(run, &executable, &binary.sha256, &mut entries);
    entries.push(binary);
    entries.push(store::capture_bytes(
        run,
        include_bytes!("../assets/cars/roster.json"),
        "embedded_catalog",
        "catalog/roster.json",
    )?);
    capture_assets(run, assets, &mut entries, &mut omissions);
    required_assets(&entries, &mut omissions);
    let embedded = entries
        .iter()
        .find(|e| e.role == "embedded_catalog")
        .map(|e| &e.sha256);
    let runtime_catalog = entries
        .iter()
        .find(|e| e.logical_path == "assets/cars/roster.json")
        .map(|e| &e.sha256);
    let catalog_matches = embedded == runtime_catalog;
    if !catalog_matches {
        omissions.push(json!({"role": "catalog", "reason": "embedded_runtime_catalog_mismatch", "replay_required": true}));
    }
    let (args, inputs, omitted) = policy::arguments();
    for (role, path) in inputs {
        match store::capture_file(run, &path, &role, &format!("inputs/{role}")) {
            Ok(entry) => entries.push(entry),
            Err(_) => omissions.push(
                json!({"role": role, "replay_required": true, "reason": "input_unavailable"}),
            ),
        }
    }
    if omitted > 0 {
        omissions.push(json!({"role": "arguments", "reason": "unknown_arguments_redacted", "count": omitted, "replay_required": true}));
    }
    let effective = json!({"settings": policy::config(&config), "arguments": args,
        "environment": policy::environment(), "asset_root_kind": root_kind});
    entries.push(store::capture_bytes(
        run,
        &serde_json::to_vec_pretty(&effective)?,
        "effective_config",
        "config/effective.json",
    )?);
    if effective["settings"].to_string().contains("redacted")
        || effective["environment"].to_string().contains("redacted")
    {
        omissions.push(json!({"role": "effective_config", "reason": "required_configuration_redacted", "replay_required": true}));
    }
    let workspace = source::runtime_snapshot(run, &mut entries);
    let capture = json!({"schema": "car-game.provenance", "version": 1, "policy": policy::ID,
        "run_id": run_id, "captured_at": crate::diagnostics::now(),
        "platform": {"os": env::consts::OS, "arch": env::consts::ARCH},
        "compiled_source_fingerprint": option_env!("CAR_GAME_BUILD_SOURCE_SHA256"),
        "asset_root_kind": root_kind, "catalog_matches_embedded": catalog_matches, "inventory": entries, "omissions": omissions,
        "binary_build": build, "runtime_workspace": workspace, "source_comparison": source::compare_sources(&entries),
        "replay_prerequisites": prerequisites(),
        "completeness": {"inventory_integrity": true,
            "runtime_material_complete": omissions.is_empty(), "runtime_replay_complete": false,
            "rebuild_complete": build["rebuild_complete"].as_bool().unwrap_or(false)}});
    store::freeze(&capture_path, &capture)?;
    Ok(
        json!({"path": "provenance/capture.json", "sha256": store::hash_file(&capture_path)?.0,
        "version": 1, "policy": policy::ID}),
    )
}

fn prerequisites() -> Value {
    json!([
        {"id": "compatible_os_arch_dynamic_libraries", "availability": "external_required", "replay_required": true},
        {"id": "gpu_renderer_audio_drivers", "availability": "external_required", "replay_required": true},
        {"id": "deterministic_cross_platform_replay", "availability": "unavailable", "replay_required": false,
            "reason": "content_integrity_does_not_prove_determinism"},
        {"id": "original_authoring_assets", "availability": "unavailable", "replay_required": false,
            "reason": "prepared_assets_included_originals_only_needed_for_regeneration"}
    ])
}

fn capture_assets(run: &Path, assets: &Path, entries: &mut Vec<Entry>, omissions: &mut Vec<Value>) {
    let result = store::files(assets).and_then(|paths| {
        for path in paths {
            let relative = path.strip_prefix(assets).expect("asset prefix");
            let logical = format!("assets/{}", relative.to_string_lossy());
            if !allowed_assets().contains(&logical) {
                omissions.push(json!({"role": "runtime_asset", "logical_path": logical,
                    "reason": "asset_path_not_allowlisted", "replay_required": true}));
                continue;
            }
            let role = match path.extension().and_then(|s| s.to_str()) {
                Some("glb") => "prepared_asset",
                Some("json") if relative.starts_with("cars") => "receipt_or_catalog",
                _ => "runtime_asset",
            };
            entries.push(store::capture_file(run, &path, role, &logical)?);
        }
        Ok(())
    });
    if result.is_err() {
        omissions.push(json!({"role": "runtime_assets", "reason": "asset_capture_failed_or_symlink_disallowed", "replay_required": true}));
    }
}

pub fn finalize(run: &Path) -> io::Result<()> {
    finalize_bounded(run, 512 * 1024 * 1024)
}

pub fn finalize_bounded(run: &Path, max_bytes: u64) -> io::Result<()> {
    let capture_path = run.join("provenance/capture.json");
    if !capture_path.exists() {
        return Ok(());
    }
    let index_path = run.join("evidence-manifest.json");
    if index_path.exists() {
        let expected = fs::read_to_string(run.join("evidence-manifest.sha256"))?;
        return if index_path.is_file() && expected.trim() == store::hash_file(&index_path)?.0 {
            Ok(())
        } else {
            Err(io::Error::other("invalid_existing_evidence_index"))
        };
    }
    let output_paths = check_output_budget(run, max_bytes)?;
    let capture: Value = serde_json::from_slice(&fs::read(&capture_path)?)?;
    let mut entries: Vec<Entry> = serde_json::from_value(capture["inventory"].clone())?;
    entries.push(store::capture_file(
        run,
        &capture_path,
        "capture_index",
        "provenance/capture.json",
    )?);
    capture_outputs(run, &mut entries, max_bytes, &output_paths)?;
    let manifest: Value = serde_json::from_slice(&fs::read(run.join("manifest.json"))?)?;
    let report = run.join("report.json");
    let report_sha = report
        .is_file()
        .then(|| store::hash_file(&report).ok().map(|p| p.0))
        .flatten();
    let evidence = json!({"schema": 2, "provenance_version": 1, "policy": policy::ID,
        "run_id": capture["run_id"], "started_at": manifest["started_at"], "ended_at": manifest["ended_at"],
        "finalized_at": crate::diagnostics::now(), "capture_sha256": store::hash_file(&capture_path)?.0,
        "inventory": entries, "completeness": capture["completeness"],
        "omissions": capture["omissions"], "replay_prerequisites": capture["replay_prerequisites"],
        "behavioral_run_result": {"exit": manifest["exit"], "failure_recorded": run.join("failure.json").exists(), "recorder_health":recording_health(run,&manifest)?},
        "report_sha256": report_sha, "cars": fleet_evidence(run, &entries),
        "diagnostic_export": "allowlisted_derived_json_and_numeric_traces_raw_logs_excluded"});
    let digest = store::digest(&serde_json::to_vec_pretty(&evidence)?);
    store::freeze_bytes(&run.join("evidence-manifest.sha256"), digest.as_bytes())?;
    store::freeze(&index_path, &evidence)
}

fn recording_health(run: &Path, manifest: &Value) -> io::Result<Value> {
    if manifest["recorder_health"]["required"] != true {
        return Ok(json!({"status":"legacy_unknown"}));
    }
    let health: Value = serde_json::from_slice(&fs::read(run.join("recorder-health.json"))?)?;
    Ok(policy::redact(&health))
}

fn capture_outputs(
    run: &Path,
    entries: &mut Vec<Entry>,
    max_bytes: u64,
    output_paths: &[std::path::PathBuf],
) -> io::Result<()> {
    for path in output_paths {
        let relative = path.strip_prefix(run).expect("run prefix");
        let logical = relative.to_string_lossy();
        if logical.starts_with("objects/") || logical.starts_with("provenance/") {
            continue;
        }
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        match extension {
            "json" => {
                if fs::metadata(path)?.len() > 2 * 1024 * 1024 {
                    return Err(io::Error::other("metadata_capture_limit_exceeded"));
                }
                let value: Value = serde_json::from_slice(&fs::read(path)?)?;
                entries.push(store::capture_bytes(
                    run,
                    &serde_json::to_vec_pretty(&policy::redact(&value))?,
                    "run_metadata",
                    &logical,
                )?);
            }
            "jsonl" => capture_jsonl(run, path, &logical, entries, max_bytes)?,
            "png" | "jpg" if relative.starts_with("captures") => {
                entries.push(store::capture_file(run, path, "capture", &logical)?)
            }
            _ => (),
        }
    }
    Ok(())
}

fn capture_jsonl(
    run: &Path,
    path: &Path,
    logical: &str,
    entries: &mut Vec<Entry>,
    max_bytes: u64,
) -> io::Result<()> {
    let temporary = run.join("provenance/.sanitized-trace.tmp");
    let result = sanitize_trace(path, &temporary, max_bytes)
        .and_then(|()| store::capture_file(run, &temporary, "run_trace", logical));
    let _ = fs::remove_file(temporary);
    entries.push(result?);
    Ok(())
}
fn sanitize_trace(source: &Path, destination: &Path, max_bytes: u64) -> io::Result<()> {
    use std::io::{BufRead, BufReader, BufWriter, Read, Write};
    let mut input = BufReader::new(fs::File::open(source)?);
    let mut output = BufWriter::new(fs::File::create(destination)?);
    let mut line = Vec::new();
    let mut total = 0u64;
    loop {
        line.clear();
        if input
            .by_ref()
            .take(2 * 1024 * 1024 + 1)
            .read_until(b'\n', &mut line)?
            == 0
        {
            break;
        }
        if line.len() > 2 * 1024 * 1024 {
            return Err(io::Error::other("trace_record_exceeds_capture_limit"));
        }
        let value: Value = serde_json::from_slice(&line)?;
        let bytes =
            crate::diagnostics::bounded_json(&policy::redact(&value), 2 * 1024 * 1024, false)
                .map_err(io::Error::other)?;
        total = total.saturating_add(bytes.len() as u64 + 1);
        if total > max_bytes {
            return Err(io::Error::other("sanitized_trace_budget_exceeded"));
        }
        output.write_all(&bytes)?;
        output.write_all(b"\n")?;
    }
    output.flush()?;
    output.get_ref().sync_all()
}
fn check_output_budget(run: &Path, max_bytes: u64) -> io::Result<Vec<std::path::PathBuf>> {
    let mut paths = Vec::new();
    let mut pending = vec![run.to_path_buf()];
    let mut count = 0;
    let mut bytes = 0u64;
    while let Some(directory) = pending.pop() {
        for item in fs::read_dir(&directory)? {
            let item = item?;
            if directory == run
                && ["objects", "provenance"]
                    .iter()
                    .any(|name| item.file_name() == *name)
            {
                continue;
            }
            count += 1;
            if count > 8192 {
                return Err(io::Error::other("output_file_budget_exceeded"));
            }
            let metadata = fs::symlink_metadata(item.path())?;
            if metadata.file_type().is_symlink() {
                return Err(io::Error::other("symlink_capture_disallowed"));
            }
            if metadata.is_dir() {
                pending.push(item.path());
            } else {
                paths.push(item.path());
                bytes = bytes.saturating_add(metadata.len());
            }
            if bytes > max_bytes {
                return Err(io::Error::other("output_byte_budget_exceeded"));
            }
        }
    }
    paths.sort();
    Ok(paths)
}

fn fleet_evidence(run: &Path, entries: &[Entry]) -> Value {
    let Ok(reports) = fs::read(run.join("report.json")) else {
        return json!([]);
    };
    let Ok(Value::Array(reports)) = serde_json::from_slice::<Value>(&reports) else {
        return json!([]);
    };
    let mut cars = Vec::new();
    for report in reports {
        let asset_path = report["asset_path"].as_str().unwrap_or_default();
        let asset = entries
            .iter()
            .find(|entry| entry.logical_path == asset_path);
        let receipt_path = Path::new(asset_path)
            .with_extension("json")
            .to_string_lossy()
            .into_owned();
        let receipt = entries
            .iter()
            .find(|entry| entry.logical_path == receipt_path);
        cars.push(json!({"id": report["id"], "asset_path": asset_path,
            "asset_sha256": asset.map(|e| &e.sha256), "receipt_sha256": receipt.map(|e| &e.sha256),
            "captures": policy::redact(&report["captures"])}));
    }
    json!(cars)
}

fn allowed_assets() -> Vec<String> {
    let catalog: Vec<Value> =
        serde_json::from_slice(include_bytes!("../assets/cars/roster.json")).unwrap_or_default();
    let mut required = vec!["assets/cars/roster.json".to_string()];
    for car in catalog {
        let Some(relative) = car["path"].as_str() else {
            continue;
        };
        let asset = format!("assets/{relative}");
        required.push(
            Path::new(&asset)
                .with_extension("json")
                .to_string_lossy()
                .into_owned(),
        );
        required.push(asset);
    }
    for audio in ["engine", "tires", "impact"] {
        required.push(format!("assets/audio/{audio}.wav"));
    }
    for index in 0..4 {
        required.push(format!("assets/textures/facade-{index}.png"));
    }
    required
}

fn required_assets(entries: &[Entry], omissions: &mut Vec<Value>) {
    for logical in allowed_assets() {
        if !entries.iter().any(|entry| entry.logical_path == logical) {
            omissions.push(json!({"role": "catalog_content", "logical_path": logical,
                "reason": "required_catalog_content_unavailable", "replay_required": true}));
        }
    }
}
