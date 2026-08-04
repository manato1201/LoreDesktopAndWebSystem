//! Content hashing for the content-addressed VCS blob store (see
//! `blob_store.rs`). A single thin wrapper around `sha2` so every call site
//! that needs a content hash (upload, stage, commit — stage/commit still
//! land in a later phase) goes through one function rather than each
//! reaching for `sha2` and formatting the digest independently.
//!
//! Called from `handlers::upload_file` as of the content-addressed upload
//! path.

use sha2::{Digest, Sha256};

/// Returns the lowercase hex-encoded SHA-256 digest of `bytes` (64
/// characters). This is the `content_hash` stored in `file_blobs` and used
/// to build blob paths (see `blob_store::blob_path`).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Well-known standard value: SHA-256 of the empty byte string is
    /// `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
    /// (see e.g. FIPS 180-4 / any SHA-256 reference implementation's test
    /// vectors).
    #[test]
    fn sha256_hex_of_empty_input_matches_known_digest() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
