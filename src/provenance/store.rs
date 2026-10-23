use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub role: String,
    pub logical_path: String,
    pub availability: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub object: String,
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn hash_file(path: &Path) -> io::Result<(String, u64)> {
    let mut source = File::open(path)?;
    let mut digest = Sha256::new();
    let mut size = 0;
    let mut buffer = [0; 65536];
    loop {
        let count = match source.read(&mut buffer) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        size += count as u64;
    }
    Ok((format!("{:x}", digest.finalize()), size))
}

pub fn capture_file(root: &Path, source: &Path, role: &str, logical: &str) -> io::Result<Entry> {
    if fs::symlink_metadata(source)?.file_type().is_symlink() {
        return Err(io::Error::other("symlink_capture_disallowed"));
    }
    let mut input = File::open(source)?;
    let objects = root.join("objects/sha256");
    fs::create_dir_all(&objects)?;
    let temporary = objects.join(format!(".capture-{}", std::process::id()));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = copy_hashed(&mut input, &mut output);
    drop(output);
    let result = result.and_then(|(sha, size)| {
        let destination = objects.join(&sha);
        if destination.exists() {
            if hash_file(&destination)? != (sha.clone(), size) {
                return Err(io::Error::other("existing_object_hash_mismatch"));
            }
        } else {
            fs::rename(&temporary, &destination)?;
        }
        Ok(entry(role, logical, sha, size))
    });
    let _ = fs::remove_file(temporary);
    result
}

fn copy_hashed(input: &mut File, output: &mut File) -> io::Result<(String, u64)> {
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    let mut size = 0;
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        output.write_all(&buffer[..count])?;
        digest.update(&buffer[..count]);
        size += count as u64;
    }
    output.sync_all()?;
    Ok((format!("{:x}", digest.finalize()), size))
}

fn entry(role: &str, logical: &str, sha: String, size: u64) -> Entry {
    Entry {
        role: role.into(),
        logical_path: logical.into(),
        availability: "included".into(),
        object: format!("objects/sha256/{sha}"),
        sha256: sha,
        size_bytes: size,
    }
}

pub fn capture_bytes(root: &Path, bytes: &[u8], role: &str, logical: &str) -> io::Result<Entry> {
    let sha = digest(bytes);
    let target = root.join(format!("objects/sha256/{sha}"));
    fs::create_dir_all(target.parent().expect("object parent"))?;
    if target.exists() {
        if hash_file(&target)? != (sha.clone(), bytes.len() as u64) {
            return Err(io::Error::other("existing_object_hash_mismatch"));
        }
    } else {
        publish(&target, bytes)?;
    }
    Ok(entry(role, logical, sha, bytes.len() as u64))
}

fn publish(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension(format!("{}-tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let result = result.and_then(|()| fs::hard_link(&temporary, path));
    let _ = fs::remove_file(temporary);
    result
}

pub fn files(root: &Path) -> io::Result<Vec<PathBuf>> {
    if fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err(io::Error::other("symlink_capture_disallowed"));
    }
    let mut result = Vec::new();
    for item in fs::read_dir(root)? {
        let path = item?.path();
        let kind = fs::symlink_metadata(&path)?.file_type();
        if kind.is_symlink() {
            return Err(io::Error::other("symlink_capture_disallowed"));
        }
        if kind.is_dir() {
            result.extend(files(&path)?);
        } else if kind.is_file() {
            result.push(path);
        }
    }
    result.sort();
    Ok(result)
}

pub fn freeze(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::create_dir_all(path.parent().expect("capture parent"))?;
    publish(path, &bytes)
}

/// Publish immutable non-JSON metadata before its referring index.
pub fn freeze_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::create_dir_all(path.parent().expect("metadata parent"))?;
    if path.exists() {
        return if fs::read(path)? == bytes {
            Ok(())
        } else {
            Err(io::Error::other("existing_metadata_mismatch"))
        };
    }
    publish(path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_sha256_vectors_cover_all_provenance_hash_paths() {
        let root = std::env::temp_dir().join(format!("grid-sha-vectors-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let source = root.join("input");
        for (bytes, expected) in [
            (
                vec![],
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc".to_vec(),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                vec![b'a'; 1_000_000],
                "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
            ),
            (
                (0..65_537).map(|i| (i % 251) as u8).collect(),
                "237356e18b503616912abb8ffaed3a72591e397d4ac294c4637917d48a3f529d",
            ),
        ] {
            fs::write(&source, &bytes).unwrap();
            assert_eq!(digest(&bytes), expected);
            assert_eq!(
                hash_file(&source).unwrap(),
                (expected.into(), bytes.len() as u64)
            );
            assert_eq!(
                crate::diagnostics::sha256_file(&source).as_deref(),
                Some(expected)
            );
            let captured = capture_file(&root, &source, "probe", "input").unwrap();
            assert_eq!(captured.sha256, expected);
            assert_eq!(captured.size_bytes, bytes.len() as u64);
            assert_eq!(fs::read(root.join(captured.object)).unwrap(), bytes);
        }
        fs::remove_dir_all(root).unwrap();
    }
}
