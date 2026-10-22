//! Canonical JSON binding shared with tools/provenance_binding.py.
use super::store;
use serde_json::{Value, json};
use std::io;

pub const FORMAT: &str = "grid-build-inputs-v1";

pub fn verify(metadata: &Value, compiled: &str) -> io::Result<()> {
    if metadata["binding_format"] != FORMAT {
        return Err(io::Error::other("unsupported_build_binding_format"));
    }
    let identity = json!({"inventory": metadata["inventory"],
        "invocation": metadata["invocation"], "flags": metadata["rust_flags"],
        "dependencies": metadata["dependency_graph"],
        "compiler_sha256": metadata["compiler_sha256"],
        "declarations": metadata["binding_declarations"]});
    let mut canonical = Vec::new();
    encode(&identity, &mut canonical)?;
    let calculated = store::digest(&canonical);
    if calculated != compiled || metadata["source_fingerprint"].as_str() != Some(compiled) {
        return Err(io::Error::other("source_build_mismatch"));
    }
    verify_completeness(metadata)?;
    Ok(())
}

fn verify_completeness(metadata: &Value) -> io::Result<()> {
    let declarations = &metadata["binding_declarations"];
    for key in [
        "compiler_version",
        "cargo_version",
        "git_commit",
        "dirty",
        "source_binding",
        "omissions",
        "rebuild_prerequisites",
    ] {
        if metadata[key] != declarations[key] {
            return Err(io::Error::other("source_build_mismatch"));
        }
    }
    let omissions = declarations["omissions"]
        .as_array()
        .ok_or_else(|| io::Error::other("source_build_mismatch"))?;
    if metadata["rebuild_complete"].as_bool() != Some(omissions.is_empty()) {
        return Err(io::Error::other("invalid_build_completeness_claim"));
    }
    Ok(())
}

fn encode(value: &Value, output: &mut Vec<u8>) -> io::Result<()> {
    match value {
        Value::Object(values) => {
            output.push(b'{');
            let mut sorted = values.iter().collect::<Vec<_>>();
            sorted.sort_by_key(|(key, _)| *key);
            for (index, (key, item)) in sorted.into_iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                serde_json::to_writer(&mut *output, key)?;
                output.push(b':');
                encode(item, output)?;
            }
            output.push(b'}');
        }
        Value::Array(values) => {
            output.push(b'[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                encode(item, output)?;
            }
            output.push(b']');
        }
        _ => serde_json::to_writer(output, value)?,
    }
    Ok(())
}
