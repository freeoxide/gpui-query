//! Reference disk persistence adapter for [`gpui_query`]: [`FilePersister`],
//! an atomic, durable [`Persister`] over one JSON or bincode file, plus a
//! re-exported [`NoopPersister`] for tests and disabled modes.
//!
//! # Atomic write
//!
//! Each save serializes the snapshot to a sibling [`tempfile::NamedTempFile`]
//! (random name, created `O_EXCL` in the target's own directory, so a shared
//! `/tmp` is never involved and symlink planting fails), fsyncs it
//! (`F_FULLFSYNC` on macOS, plain `fsync` elsewhere), then renames it over
//! the target. On POSIX the parent directory is fsynced after the replace.
//! A crash mid-write therefore leaves the previous file intact plus, at
//! worst, one stray `<name>.<random>.tmp` sibling. The file is created with
//! owner-only permissions (`0o600` on Unix), since a query cache is app- and
//! user-private data.
//!
//! # Tolerant load
//!
//! A missing file yields an empty snapshot; a corrupt or unparseable one is
//! logged and treated as empty; a version mismatch returns
//! [`PersistError::VersionMismatch`] so callers can tell "corrupt" from
//! "wrong format".
//!
//! # Concurrency
//!
//! Saves on one persister are serialized by a `std::sync::Mutex`. Loads skip
//! the lock: the rename is atomic, so a load concurrent with a save sees
//! either the old or the new complete file. Two persister instances on the
//! same path likewise cannot corrupt each other; each save replaces the
//! whole file and the last writer wins, matching the [`Persister`] contract.
//!
//! The async methods do synchronous `std::fs` I/O with no await points, which
//! is what GPUI's blocking-friendly `background_executor` is for. On a tokio
//! runtime, wrap `load`/`save` in `spawn_blocking` to avoid stalling worker
//! threads.
//!
//! On Windows the atomic replace can fail with `ERROR_ACCESS_DENIED` while an
//! antivirus scanner or concurrent reader holds the destination; that is
//! surfaced as the retryable [`PersistError::Permission`] rather than
//! [`PersistError::Io`], which still carries the original error (kind and
//! source chain intact) for every other failure.

#![deny(missing_docs)]

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use gpui_query::client::{
    PERSIST_VERSION, PersistError, PersistSnapshot, PersistedEntry, Persister,
};
use gpui_query::core::CachePolicy;

/// On-disk serialization format for [`FilePersister`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistFormat {
    /// Human-readable JSON (`serde_json`). Default; easy to inspect/debug.
    Json,
    /// Compact binary (`bincode`). Smaller and faster; not human-readable.
    Bincode,
}

/// Atomic, durable disk-backed [`Persister`].
///
/// Saves write a sibling [`tempfile::NamedTempFile`], fsync it, and rename it
/// over the target, so a crash never leaves a truncated file. See the
/// [crate docs](crate) for the durability and concurrency story.
pub struct FilePersister {
    path: PathBuf,
    format: PersistFormat,
    write_lock: Mutex<()>,
}

impl FilePersister {
    /// Construct a persister writing to `path` in the given `format`.
    pub fn new(path: impl Into<PathBuf>, format: PersistFormat) -> Self {
        Self {
            path: path.into(),
            format,
            write_lock: Mutex::new(()),
        }
    }

    /// Construct a JSON persister at `path`.
    pub fn json(path: impl Into<PathBuf>) -> Self {
        Self::new(path, PersistFormat::Json)
    }

    /// Construct a bincode persister at `path`.
    pub fn bincode(path: impl Into<PathBuf>) -> Self {
        Self::new(path, PersistFormat::Bincode)
    }

    /// Construct a JSON persister at `<cache_dir>/<app_name>/gpui-query-cache.json`.
    ///
    /// Returns [`PersistError::BadPath`] when the OS reports no cache dir.
    /// The cache dir (rather than Roaming config) is deliberate: the file is
    /// a regenerable offline cache, not state worth syncing. `app_name` is
    /// joined as-is, so treat it as trusted configuration.
    pub fn in_cache_dir(app_name: impl AsRef<str>) -> Result<Self, PersistError> {
        let app_name = app_name.as_ref();
        let dir = dirs::cache_dir().ok_or_else(|| {
            PersistError::BadPath(format!("no OS cache dir available for app {app_name:?}"))
        })?;
        Ok(Self::json(dir.join(app_name).join("gpui-query-cache.json")))
    }

    /// The on-disk path this persister writes to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Serialize + atomically write `snapshot` to disk.
    fn write_atomic(&self, snapshot: &PersistSnapshot) -> Result<(), PersistError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| PersistError::Permission("write lock poisoned".to_string()))?;

        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        let bytes: Vec<u8> = match self.format {
            PersistFormat::Json => serde_json::to_vec(snapshot)?,
            // bincode's format is not self-describing, so it cannot drive
            // serde_json::Value's deserialize_any; the adapter carries each
            // value as a JSON String instead.
            PersistFormat::Bincode => {
                let adapter = BincodeSnapshot::from_snapshot(snapshot)?;
                bincode::serialize(&adapter).map_err(|e| {
                    use serde::ser::Error as _;
                    PersistError::Serialize(serde_json::Error::custom(e.to_string()))
                })?
            }
        };

        // Sibling temp file, fsync, rename over the target. The .tmp suffix
        // keeps crash orphans identifiable for cleanup.
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        let mut tmp = tempfile::Builder::new()
            .prefix(
                self.path
                    .file_name()
                    .map(Path::new)
                    .unwrap_or_else(|| Path::new("cache")),
            )
            .suffix(".tmp")
            .tempfile_in(parent)?;
        tmp.write_all(&bytes)?;
        tmp.as_file().sync_all()?;
        #[cfg(target_os = "macos")]
        try_fullfsync(tmp.as_file());
        // A Windows AV scanner or concurrent reader holding the destination
        // makes the replace fail with ERROR_ACCESS_DENIED; that case is
        // retryable, so it maps to Permission instead of Io.
        tmp.persist(&self.path).map_err(|persist_err| {
            let io_err = persist_err.error;
            let denied = io_err.kind() == std::io::ErrorKind::PermissionDenied
                || is_windows_access_denied(io_err.raw_os_error());
            if denied {
                PersistError::Permission(format!(
                    "atomic persist of cache file was denied (retryable): {io_err}"
                ))
            } else {
                PersistError::Io(io_err)
            }
        })?;

        #[cfg(unix)]
        fsync_parent(parent);

        Ok(())
    }

    /// Tolerantly read + deserialize the snapshot from disk. Lock-free: the
    /// atomic rename means a concurrent save can only swap in another
    /// complete file, never expose a partial one.
    fn read_tolerant(&self) -> Result<PersistSnapshot, PersistError> {
        let mut file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(PersistSnapshot::new());
            }
            Err(e) => return Err(e.into()),
        };

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        let (label, parsed): (&str, Result<PersistSnapshot, String>) = match self.format {
            PersistFormat::Json => {
                ("JSON", serde_json::from_slice(&buf).map_err(|e| e.to_string()))
            }
            PersistFormat::Bincode => ("bincode", bincode_load(&buf)),
        };
        let snapshot = match parsed {
            Ok(s) => s,
            Err(detail) => {
                eprintln!(
                    "FilePersister: corrupt {label} cache at {}: {detail}; treating as empty",
                    self.path.display()
                );
                return Ok(PersistSnapshot::new());
            }
        };

        if snapshot.version != PERSIST_VERSION {
            return Err(PersistError::VersionMismatch {
                expected: PERSIST_VERSION,
                found: snapshot.version,
            });
        }
        Ok(snapshot)
    }
}

impl Persister for FilePersister {
    async fn load(&self) -> Result<PersistSnapshot, PersistError> {
        self.read_tolerant()
    }

    async fn save(&self, snapshot: &PersistSnapshot) -> Result<(), PersistError> {
        self.write_atomic(snapshot)
    }
}

/// A [`Persister`] that persists nothing and loads an empty snapshot, for
/// tests or disabled modes. Re-exported from `gpui_query::client` so this
/// crate is a one-stop import.
pub use gpui_query::client::NoopPersister;

// ── helpers ─────────────────────────────────────────────────────────────

/// Bincode-safe adapter for [`PersistSnapshot`].
///
/// `serde_json::Value` deserializes via `deserialize_any`, which bincode's
/// non-self-describing format cannot drive. The adapter stores each entry's
/// `value` (and `meta`) as a JSON `String`, which bincode carries natively.
/// The conversion is lossless.
#[derive(serde::Serialize, serde::Deserialize)]
struct BincodeSnapshot {
    entries: HashMap<String, BincodeEntry>,
    version: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct BincodeEntry {
    /// The entry's value, JSON-encoded to a String so bincode can carry it.
    value_json: String,
    cached_at: u64,
    cache_policy: CachePolicy,
    meta_json: Option<String>,
}

impl BincodeSnapshot {
    fn from_snapshot(s: &PersistSnapshot) -> Result<Self, PersistError> {
        let mut entries = HashMap::with_capacity(s.entries.len());
        for (k, e) in &s.entries {
            entries.insert(
                k.clone(),
                BincodeEntry {
                    value_json: serde_json::to_string(&e.value)?,
                    cached_at: e.cached_at,
                    cache_policy: e.cache_policy,
                    meta_json: e
                        .meta
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()?,
                },
            );
        }
        Ok(Self {
            entries,
            version: s.version,
        })
    }

    fn into_snapshot(self) -> Result<PersistSnapshot, PersistError> {
        let mut entries = HashMap::with_capacity(self.entries.len());
        for (k, e) in self.entries {
            let value: serde_json::Value = serde_json::from_str(&e.value_json)?;
            let meta = e.meta_json.map(|m| serde_json::from_str(&m)).transpose()?;
            entries.insert(
                k,
                PersistedEntry {
                    value,
                    cached_at: e.cached_at,
                    cache_policy: e.cache_policy,
                    meta,
                },
            );
        }
        Ok(PersistSnapshot {
            entries,
            version: self.version,
        })
    }
}

/// Decode a bincode-format snapshot, flattening both the bincode step and the
/// inner JSON step into one String error (the tolerant path only logs it).
fn bincode_load(buf: &[u8]) -> Result<PersistSnapshot, String> {
    let adapter: BincodeSnapshot = bincode::deserialize(buf).map_err(|e| e.to_string())?;
    adapter.into_snapshot().map_err(|e| e.to_string())
}

/// Also match raw Windows `ERROR_ACCESS_DENIED` (5); std maps it to
/// `PermissionDenied`, but errors built via `from_raw_os_error` on older
/// toolchains may not be normalized.
#[cfg(windows)]
const ERROR_ACCESS_DENIED: i32 = 5;
fn is_windows_access_denied(raw: Option<i32>) -> bool {
    #[cfg(windows)]
    {
        raw == Some(ERROR_ACCESS_DENIED)
    }
    #[cfg(not(windows))]
    {
        let _ = raw;
        false
    }
}

/// macOS `F_FULLFSYNC`: unlike `fsync`, it also flushes the drive's write
/// cache. Best-effort; failure is logged and the save still succeeds on the
/// strength of the preceding `sync_all`.
#[cfg(target_os = "macos")]
fn try_fullfsync(file: &File) {
    // F_FULLFSYNC = 0x00008027 (fcntl.h on Darwin); extern declared here to
    // avoid a libc dependency.
    unsafe extern "C" {
        fn fcntl(fd: std::os::fd::RawFd, cmd: std::ffi::c_int, ...) -> std::ffi::c_int;
    }
    const F_FULLFSYNC: std::ffi::c_int = 0x00008027;
    use std::os::fd::AsRawFd;
    // SAFETY: F_FULLFSYNC takes no argument (the variadic tail is unused) and
    // the fd is the temp file we just wrote.
    let rc = unsafe { fcntl(file.as_raw_fd(), F_FULLFSYNC) };
    if rc != 0 {
        eprintln!("FilePersister: F_FULLFSYNC failed (rc={rc}); relying on fsync");
    }
}

/// fsync the parent directory so the rename is durable across power loss.
#[cfg(unix)]
fn fsync_parent(parent: &Path) {
    use std::os::fd::AsRawFd;
    match OpenOptions::new().read(true).open(parent) {
        Ok(dir) => {
            unsafe extern "C" {
                fn fsync(fd: std::ffi::c_int) -> std::ffi::c_int;
            }
            // SAFETY: fd is a valid open directory file descriptor.
            let rc = unsafe { fsync(dir.as_raw_fd()) };
            if rc != 0 {
                eprintln!("FilePersister: parent-dir fsync failed");
            }
        }
        Err(e) => {
            eprintln!("FilePersister: could not open parent dir for fsync: {e}");
        }
    }
}
