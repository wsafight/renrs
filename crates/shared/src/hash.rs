//! SHA-256 helpers.
//!
//! Only the shapes that were byte-identical across crates live here. The
//! compiler's domain-separated, length-prefixed ID hashing deliberately stays
//! local: changing it would change every generated statement and instruction
//! ID.

use sha2::{Digest, Sha256};
use std::io::{self, Read};
use std::path::Path;

/// Hex digest of an in-memory buffer.
#[must_use]
pub fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(bytes.as_ref()))
}

/// Hex digest of a reader, hashed in fixed-size chunks.
///
/// # Errors
///
/// Returns the reader's I/O error.
pub fn sha256_reader(mut reader: impl Read) -> io::Result<String> {
    let mut digest = Sha256::new();
    let mut buffer = vec![0; 65_536];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(hex::encode(digest.finalize()))
}

/// Hex digest of a file without loading it into memory.
///
/// # Errors
///
/// Returns filesystem errors.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    sha256_reader(std::fs::File::open(path)?)
}

/// Copies `input` to `output` while hashing, returning the byte count and digest.
///
/// # Errors
///
/// Returns read or write errors from either side.
pub fn copy_hashed(
    input: &mut impl Read,
    output: &mut impl std::io::Write,
    buffer: &mut [u8],
) -> io::Result<(u64, String)> {
    if buffer.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "hash copy buffer must not be empty",
        ));
    }
    let mut digest = Sha256::new();
    let mut length = 0_u64;
    loop {
        let count = input.read(buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        output.write_all(&buffer[..count])?;
        length += count as u64;
    }
    Ok((length, hex::encode(digest.finalize())))
}

/// Hashes `fingerprint` together with a serialized payload.
///
/// Used to fold derived tables (layered images, extensions) into the program
/// fingerprint without a second hashing convention.
///
/// # Errors
///
/// Returns the serialization error.
pub fn fold_fingerprint<T: serde::Serialize>(
    fingerprint: &str,
    payload: &T,
) -> Result<String, serde_json::Error> {
    let mut hash = Sha256::new();
    hash.update(fingerprint.as_bytes());
    hash.update(serde_json::to_vec(payload)?);
    Ok(hex::encode(hash.finalize()))
}
