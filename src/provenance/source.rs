use super::{
    binding, policy,
    store::{self, Entry},
};
use serde_json::{Value, json};
use std::{fs, io, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().into())
}

pub fn runtime_snapshot(run: &Path, entries: &mut Vec<Entry>) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let result = source_files(root).and_then(|files| {
        for logical in files {
            entries.push(store::capture_file(
                run,
                &root.join(&logical),
                "runtime_workspace_snapshot",
                &format!("runtime_source/{logical}"),
            )?);
        }
        if let Some(patch) = git(
            root,
            &[
                "diff",
                "--binary",
                "HEAD",
                "--",
                "src",
                "Cargo.toml",
                "Cargo.lock",
                "build.rs",
                "assets/cars/roster.json",
                "assets/cars/physics.json",
            ],
        ) {
            entries.push(store::capture_bytes(
                run,
                patch.as_bytes(),
                "runtime_workspace_patch",
                "runtime_source/dirty.patch",
            )?);
        }
        Ok::<_, io::Error>(())
    });
    json!({"availability": if result.is_ok() { "included" } else { "unavailable" },
        "binding": "runtime_observation_only_not_binary_build_source",
        "commit": git(root, &["rev-parse", "HEAD"]),
        "dirty": git(root, &["status", "--porcelain"]).map(|text| !text.is_empty()),
        "deleted_paths": git(root, &["diff", "--name-only", "--diff-filter=D", "HEAD", "--", "src", "Cargo.toml", "Cargo.lock", "build.rs", "assets"]).map(|text| text.lines().map(String::from).collect::<Vec<_>>()),
        "reason": result.err().map(|_| "runtime_source_unavailable")})
}

pub fn source_files(root: &Path) -> io::Result<Vec<String>> {
    let mut paths = store::files(&root.join("src"))?
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .map(|p| {
            p.strip_prefix(root)
                .expect("source prefix")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    for logical in [
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "rust-toolchain",
        "rust-toolchain.toml",
        "assets/cars/roster.json",
        "assets/cars/physics.json",
    ] {
        if root.join(logical).is_file() {
            paths.push(logical.into());
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn build_snapshot(
    run: &Path,
    executable: &Path,
    binary_sha: &str,
    entries: &mut Vec<Entry>,
) -> Value {
    let sidecar = executable.with_file_name(format!(
        "{}.provenance",
        executable.file_name().unwrap_or_default().to_string_lossy()
    ));
    match import_build(run, &sidecar, binary_sha, entries) {
        Ok(metadata) => metadata,
        Err(error) => json!({"availability": "unavailable", "rebuild_complete": false,
            "reason": build_error(&error),
            "compiler": "unavailable", "dependency_features": "unavailable", "flags": "unavailable"}),
    }
}

fn import_build(
    run: &Path,
    sidecar: &Path,
    binary_sha: &str,
    entries: &mut Vec<Entry>,
) -> io::Result<Value> {
    let metadata = read_build_metadata(sidecar, binary_sha)?;
    let sources: Vec<Entry> = serde_json::from_value(metadata["inventory"].clone())?;
    validate_sources(&sources)?;
    for source in sources {
        let suffix = source
            .object
            .strip_prefix("objects/sha256/")
            .filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()))
            .ok_or_else(|| io::Error::other("unsafe_bundle_path"))?;
        let path = sidecar_file(sidecar, &format!("objects/sha256/{suffix}"))?;
        if suffix != source.sha256
            || source.role != "binary_build_snapshot"
            || !safe_logical(&source.logical_path)
        {
            return Err(io::Error::other("unsafe_build_entry"));
        }
        let captured = store::capture_file(run, &path, &source.role, &source.logical_path)?;
        if captured.sha256 != source.sha256 || captured.size_bytes != source.size_bytes {
            return Err(io::Error::other("source_build_mismatch"));
        }
        entries.push(captured);
    }
    entries.push(store::capture_bytes(
        run,
        &serde_json::to_vec(&policy::redact(&metadata))?,
        "build_metadata",
        "build/build.json",
    )?);
    Ok(policy::redact(&metadata))
}

fn read_build_metadata(sidecar: &Path, binary_sha: &str) -> io::Result<Value> {
    let metadata: Value = serde_json::from_slice(&fs::read(sidecar_file(sidecar, "build.json")?)?)?;
    let compiled_fingerprint = option_env!("CAR_GAME_BUILD_SOURCE_SHA256")
        .ok_or_else(|| io::Error::other("compiler_source_binding_unavailable"))?;
    if metadata["schema"] != 1
        || metadata["binary_sha256"].as_str() != Some(binary_sha)
        || metadata["source_fingerprint"].as_str() != Some(compiled_fingerprint)
    {
        return Err(io::Error::other("source_build_mismatch"));
    }
    binding::verify(&metadata, compiled_fingerprint)?;
    Ok(metadata)
}

fn build_error(error: &io::Error) -> &'static str {
    match error.to_string().as_str() {
        "source_build_mismatch" => "source_build_mismatch",
        "invalid_build_completeness_claim" => "invalid_build_completeness_claim",
        "unsupported_build_binding_format" => "unsupported_build_binding_format",
        _ => "matching_build_sidecar_unavailable_use_tools_record_build_py",
    }
}

fn safe_logical(path: &str) -> bool {
    !path.starts_with('/')
        && !path.contains('\\')
        && !path.split('/').any(|p| p == ".." || p.is_empty())
}

pub fn compare_sources(entries: &[Entry]) -> Value {
    let build = entries
        .iter()
        .filter(|entry| {
            entry.role == "binary_build_snapshot"
                && entry.logical_path != "build_source/dirty.patch"
        })
        .collect::<Vec<_>>();
    if build.is_empty() {
        return json!({"availability": "unavailable", "reason": "binary_build_snapshot_unavailable"});
    }
    let mut differences = Vec::new();
    for entry in entries
        .iter()
        .filter(|entry| entry.role == "runtime_workspace_snapshot")
    {
        let logical = entry.logical_path.trim_start_matches("runtime_source/");
        if !build.iter().any(|source| {
            source.logical_path.trim_start_matches("build_source/") == logical
                && source.sha256 == entry.sha256
        }) {
            differences.push(logical);
        }
    }
    for entry in &build {
        let logical = entry.logical_path.trim_start_matches("build_source/");
        if !entries.iter().any(|source| {
            source.role == "runtime_workspace_snapshot"
                && source.logical_path.trim_start_matches("runtime_source/") == logical
        }) {
            differences.push(logical);
        }
    }
    json!({"availability": "included", "matches": differences.is_empty(),
        "code": if differences.is_empty() { "source_build_match" } else { "source_build_mismatch" },
        "logical_paths": differences,
        "meaning": "runtime_workspace_observation_does_not_change_recorded_executable_identity"})
}

fn sidecar_file(root: &Path, logical: &str) -> io::Result<std::path::PathBuf> {
    let mut path = root.to_path_buf();
    if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err(io::Error::other("unsafe_bundle_path"));
    }
    for part in Path::new(logical).components() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return Err(io::Error::other("unsafe_bundle_path"));
        }
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(io::Error::other("unsafe_bundle_path"));
        }
    }
    Ok(path)
}

fn validate_sources(sources: &[Entry]) -> io::Result<()> {
    let mut paths = std::collections::HashSet::new();
    for entry in sources {
        if entry.role != "binary_build_snapshot"
            || !entry.logical_path.starts_with("build_source/")
            || !paths.insert(entry.logical_path.as_str())
        {
            return Err(io::Error::other("source_build_mismatch"));
        }
    }
    for required in [
        "Cargo.toml",
        "Cargo.lock",
        "src/main.rs",
        "assets/cars/roster.json",
        "dirty.patch",
    ] {
        if !paths.contains(format!("build_source/{required}").as_str()) {
            return Err(io::Error::other("source_build_mismatch"));
        }
    }
    if paths.contains("build_source/src/vehicle/profile.rs")
        && !paths.contains("build_source/assets/cars/physics.json")
    {
        return Err(io::Error::other("source_build_mismatch"));
    }
    Ok(())
}
