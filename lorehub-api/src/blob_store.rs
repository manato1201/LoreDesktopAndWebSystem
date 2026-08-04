//! Filesystem-backed content-addressed blob storage.
//!
//! File content bytes for the VCS schema (see `migrations/0001_vcs_schema.
//! sql`) live on disk rather than in SQLite — `file_blobs` only stores
//! metadata (hash/size). This module is the read/write boundary for that
//! filesystem layout: `{base_dir}/blobs/{repo_slug}/{content_hash[..2]}/
//! {content_hash}`, so a single directory never accumulates more entries
//! than fit comfortably (256-way fan-out on the hash's first byte) and blobs
//! are naturally partitioned per repository.
//!
//! Wired into the upload/read handlers (`handlers::upload_file`/
//! `handlers::resolve_uploaded_content`) as of the content-addressed upload
//! path — see `BASE_DIR` for what `base_dir` is at those call sites.

use std::path::{Path, PathBuf};

use rand::Rng;
use tokio::fs;

/// The filesystem root every `base_dir` argument in this module resolves
/// against at the handlers' call sites — the same relative-to-process-CWD
/// basis `db::connect("lorehub.db")` uses for its bare filename (see that
/// function's doc comment). In Docker, `WORKDIR /data` (see `Dockerfile`)
/// means both `lorehub.db` and `./blobs/...` land inside the same mounted
/// volume with no extra code needed to keep them together.
pub const BASE_DIR: &str = ".";

/// Returns the on-disk path for `content_hash` within `repo_slug`, rooted at
/// `base_dir`: `{base_dir}/blobs/{repo_slug}/{content_hash[..2]}/{content_hash}`.
/// Does not touch the filesystem — pure path construction.
pub fn blob_path(base_dir: &Path, repo_slug: &str, content_hash: &str) -> PathBuf {
    let prefix = &content_hash[..2.min(content_hash.len())];
    base_dir
        .join("blobs")
        .join(repo_slug)
        .join(prefix)
        .join(content_hash)
}

/// Writes `bytes` to the blob path for `content_hash`, creating parent
/// directories as needed. Content-addressed, so if a file already exists at
/// the target path this is a no-op (identical hash implies identical bytes
/// — no need to rewrite, and no read-back verification is done).
///
/// Otherwise writes to a temp file in the same directory and atomically
/// renames it into place, so a crash or concurrent reader never observes a
/// partially-written blob.
///
/// Retries up to [`MAX_SAVE_ATTEMPTS`] times if the parent directory
/// disappears mid-write (`NotFound` from the write or rename step) rather
/// than surfacing it as a hard error. This can genuinely happen: a
/// repository's blob directory can be removed out from under an in-flight
/// write by a concurrent `DELETE /api/repositories/{slug}` (see
/// `handlers::delete_repository`'s filesystem-cleanup step) landing between
/// this function's own `create_dir_all` and its `write`/`rename` — recreating
/// the directory and retrying is simpler and more robust than trying to
/// serialize upload and delete against each other.
async fn save_blob_attempt(
    parent: &Path,
    target: &Path,
    content_hash: &str,
    bytes: &[u8],
) -> std::io::Result<()> {
    fs::create_dir_all(parent).await?;

    let suffix: u64 = rand::thread_rng().r#gen();
    let tmp_path = parent.join(format!("{content_hash}.tmp-{suffix:016x}"));

    fs::write(&tmp_path, bytes).await?;
    fs::rename(&tmp_path, target).await?;

    Ok(())
}

const MAX_SAVE_ATTEMPTS: u32 = 3;

pub async fn save_blob(
    base_dir: &Path,
    repo_slug: &str,
    content_hash: &str,
    bytes: &[u8],
) -> std::io::Result<()> {
    let target = blob_path(base_dir, repo_slug, content_hash);

    // Content-addressed: identical hash means identical bytes, so an
    // existing file at this path never needs to be rewritten.
    if fs::metadata(&target).await.is_ok() {
        return Ok(());
    }

    let parent = target
        .parent()
        .expect("blob_path always has a parent directory");

    let mut last_err = None;
    for _ in 0..MAX_SAVE_ATTEMPTS {
        match save_blob_attempt(parent, &target, content_hash, bytes).await {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                last_err = Some(err);
            }
            Err(err) => return Err(err),
        }
    }
    Err(last_err.expect("loop runs at least once"))
}

/// Reads back the bytes previously written by [`save_blob`] for
/// `content_hash`. Returns an `Err` (not a panic) if no blob exists at that
/// path.
pub async fn load_blob(
    base_dir: &Path,
    repo_slug: &str,
    content_hash: &str,
) -> std::io::Result<Vec<u8>> {
    let target = blob_path(base_dir, repo_slug, content_hash);
    fs::read(&target).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_path_matches_documented_layout() {
        let base = Path::new("/data");
        let hash = "abcd1234";
        let path = blob_path(base, "my-repo", hash);
        assert_eq!(path, Path::new("/data/blobs/my-repo/ab/abcd1234"));
    }

    #[tokio::test]
    async fn save_then_load_round_trips_exact_bytes() {
        let base = tempfile::tempdir().expect("create temp dir");
        let content_hash = "roundtriphash";
        let bytes = b"hello content-addressed world".to_vec();

        save_blob(base.path(), "repo-a", content_hash, &bytes)
            .await
            .expect("save_blob should succeed");
        let loaded = load_blob(base.path(), "repo-a", content_hash)
            .await
            .expect("load_blob should succeed");

        assert_eq!(loaded, bytes);
    }

    #[tokio::test]
    async fn saving_the_same_content_twice_does_not_error() {
        let base = tempfile::tempdir().expect("create temp dir");
        let content_hash = "idempotenthash";
        let bytes = b"same bytes every time".to_vec();

        save_blob(base.path(), "repo-b", content_hash, &bytes)
            .await
            .expect("first save_blob should succeed");
        save_blob(base.path(), "repo-b", content_hash, &bytes)
            .await
            .expect("second save_blob with identical hash should also succeed");

        let loaded = load_blob(base.path(), "repo-b", content_hash)
            .await
            .expect("load_blob should succeed");
        assert_eq!(loaded, bytes);
    }

    #[tokio::test]
    async fn loading_a_nonexistent_blob_returns_err() {
        let base = tempfile::tempdir().expect("create temp dir");

        let result = load_blob(base.path(), "repo-c", "does-not-exist").await;

        assert!(result.is_err());
    }
}
