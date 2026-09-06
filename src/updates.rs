use crate::archive::{ResourceArchive, pack_project};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    sha256: String,
    length: u64,
    changed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    project_id: String,
    base_sha256: String,
    target_sha256: String,
    entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedManifest {
    manifest: Manifest,
    signature: Vec<u8>,
}

/// Computes the digest of a distribution without loading it into memory.
/// # Errors
/// Returns filesystem errors.
pub fn file_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// Creates a signed resource delta. Existing destinations are never replaced.
/// # Errors
/// Rejects project mismatches, invalid resources, keys and filesystem failures.
pub fn create(old: &Path, new: &Path, destination: &Path, secret: &[u8; 32]) -> Result<usize> {
    if destination.exists() {
        return Err("patch destination exists".into());
    }
    let base = ResourceArchive::open(old)?;
    let target = ResourceArchive::open(new)?;
    let compile = |path: &Path| -> Result<crate::Program> {
        crate::ProjectSource::open(path)?
            .compile()
            .map_err(|errors| {
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .into()
            })
    };
    let old_program: crate::Program = compile(old)?;
    let new_program: crate::Program = compile(new)?;
    if old_program.project_id != new_program.project_id {
        return Err("update belongs to another project".into());
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let temp = tempfile::tempdir_in(parent)?;
    let mut entries = Vec::new();
    for entry in target.entries() {
        if !crate::resources::visible_path(&entry.path) {
            return Err("excluded update resource".into());
        }
        let changed = !base.entries().iter().any(|old| {
            old.path == entry.path && old.sha256 == entry.sha256 && old.length == entry.length
        });
        if changed {
            let path = temp.path().join("files").join(&entry.path);
            fs::create_dir_all(path.parent().ok_or("invalid update path")?)?;
            fs::write(path, target.read(&entry.path)?)?;
        }
        entries.push(Entry {
            path: entry.path.clone(),
            sha256: entry.sha256.clone(),
            length: entry.length,
            changed,
        });
    }
    let count = entries.iter().filter(|entry| entry.changed).count();
    let manifest = Manifest {
        version: 1,
        project_id: new_program.project_id,
        base_sha256: file_hash(old)?,
        target_sha256: file_hash(new)?,
        entries,
    };
    let signature = SigningKey::from_bytes(secret)
        .sign(&serde_json::to_vec(&manifest)?)
        .to_bytes()
        .to_vec();
    fs::write(
        temp.path().join("update.json"),
        serde_json::to_vec(&SignedManifest {
            manifest,
            signature,
        })?,
    )?;
    fs::rename(temp.path(), destination)?;
    Ok(count)
}

/// Verifies a publisher signature, base archive and every target resource, then writes a new archive.
/// The trusted public key must come from the installed game or publisher, not the patch itself.
/// # Errors
/// Rejects tampered manifests/resources, incorrect base versions and unsafe paths.
pub fn apply(old: &Path, patch: &Path, destination: &Path, public: &[u8; 32]) -> Result<()> {
    if destination.exists() {
        return Err("update destination exists; use a new path".into());
    }
    let bytes = fs::read(patch.join("update.json"))?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("update manifest too large".into());
    }
    let envelope: SignedManifest = serde_json::from_slice(&bytes)?;
    VerifyingKey::from_bytes(public)?.verify_strict(
        &serde_json::to_vec(&envelope.manifest)?,
        &Signature::from_slice(&envelope.signature)?,
    )?;
    let manifest = envelope.manifest;
    if manifest.version != 1 || file_hash(old)? != manifest.base_sha256 {
        return Err("update base version mismatch".into());
    }
    let base = ResourceArchive::open(old)?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let temporary = tempfile::tempdir_in(parent)?;
    let root = temporary.path().join("project");
    fs::create_dir(&root)?;
    let mut names = std::collections::HashSet::new();
    for entry in manifest.entries {
        if !crate::resources::visible_path(&entry.path) || !names.insert(entry.path.clone()) {
            return Err("unsafe or duplicate update path".into());
        }
        let bytes = if entry.changed {
            fs::read(crate::resources::resource_path(
                &patch.join("files"),
                &entry.path,
            )?)?
        } else {
            base.read(&entry.path)?
        };
        if bytes.len() as u64 != entry.length
            || format!("{:x}", Sha256::digest(&bytes)) != entry.sha256
        {
            return Err(format!("update checksum mismatch: {}", entry.path).into());
        }
        let output_file = root.join(entry.path);
        fs::create_dir_all(output_file.parent().ok_or("invalid update path")?)?;
        fs::write(output_file, bytes)?;
    }
    let output = temporary.path().join("game.renrs");
    pack_project(&root, &output)?;
    if file_hash(&output)? != manifest.target_sha256 {
        return Err("assembled update checksum mismatch".into());
    }
    let source = crate::ProjectSource::open(&output)?;
    let program = source.compile().map_err(|errors| {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    if program.project_id != manifest.project_id {
        return Err("assembled project ID mismatch".into());
    }
    fs::rename(output, destination)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_delta_reconstructs_target_and_rejects_tampering() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("script.rns"),
            "config id \"test.update\"\nlabel start:\n    \"Before\"",
        )
        .unwrap();
        fs::write(root.join("unchanged.txt"), "same").unwrap();
        let old = temp.path().join("old.renrs");
        pack_project(&root, &old).unwrap();
        fs::write(
            root.join("script.rns"),
            "config id \"test.update\"\nlabel start:\n    \"After\"",
        )
        .unwrap();
        let new = temp.path().join("new.renrs");
        pack_project(&root, &new).unwrap();
        let secret = [7; 32];
        let public = SigningKey::from_bytes(&secret).verifying_key().to_bytes();
        let patch = temp.path().join("patch");
        assert_eq!(create(&old, &new, &patch, &secret).unwrap(), 1);
        let result = temp.path().join("result.renrs");
        apply(&old, &patch, &result, &public).unwrap();
        assert_eq!(file_hash(&new).unwrap(), file_hash(&result).unwrap());
        fs::write(patch.join("files/script.rns"), "tampered").unwrap();
        assert!(apply(&old, &patch, &temp.path().join("bad.renrs"), &public).is_err());
        assert!(!temp.path().join("bad.renrs").exists());
        assert!(
            apply(
                &old,
                &patch,
                &temp.path().join("wrong-key.renrs"),
                &SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes()
            )
            .is_err()
        );
    }
}
