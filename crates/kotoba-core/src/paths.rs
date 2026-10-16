//! Filesystem layout for Kotoba.
//!
//! Everything Kotoba owns lives under `~/.kotoba` (mode 0700). Kotoba
//! stands alone: it never reads Emaki's `~/.emaki`, even where the files
//! are the same.
//!
//! ```text
//! ~/.kotoba/
//!   config.json                   the settings, under `app`
//!   models/<file>                 speech and post-processing models
//!   dictation/history.jsonl       what dictation wrote, a line each
//!   state/update.json             when the newest release was last asked for
//!   cache/updates/                an installer while it is fetched
//! ```

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

pub fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

fn env_path(name: &str) -> Option<PathBuf> {
    let raw = std::env::var_os(name)?;
    if raw.is_empty() {
        return None;
    }
    let s = raw.to_string_lossy();
    Some(expand_tilde(&s))
}

pub fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        home().join(rest)
    } else if s == "~" {
        home()
    } else {
        PathBuf::from(s)
    }
}

/// The Kotoba data directory. `KOTOBA_HOME` overrides (tests use it).
pub fn root() -> PathBuf {
    env_path("KOTOBA_HOME").unwrap_or_else(|| home().join(".kotoba"))
}

pub fn config_file() -> PathBuf {
    root().join("config.json")
}
pub fn state_dir() -> PathBuf {
    root().join("state")
}
pub fn cache_dir() -> PathBuf {
    root().join("cache")
}
/// The downloaded models, speech and post-processing alike.
pub fn models_dir() -> PathBuf {
    root().join("models")
}
/// Where the dictation's history is kept.
pub fn dictation_dir() -> PathBuf {
    root().join("dictation")
}

/// Create the whole tree with private permissions. Idempotent.
pub fn ensure_dirs() -> std::io::Result<()> {
    let r = root();
    fs::create_dir_all(&r)?;
    chmod700(&r);
    for d in [state_dir(), cache_dir(), models_dir(), dictation_dir()] {
        fs::create_dir_all(&d)?;
        chmod700(&d);
    }
    Ok(())
}

#[cfg(unix)]
fn chmod700(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
}
#[cfg(not(unix))]
fn chmod700(_path: &Path) {}

pub fn read_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn write_json(path: &Path, data: &Value) -> std::io::Result<()> {
    let mut text = serde_json::to_string_pretty(data)?;
    text.push('\n');
    write_atomic(path, text.as_bytes())
}

/// Write via a sibling temp file and rename, so a reader sees either the old
/// file or the new one and never a half-written one.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let tmp = path.with_file_name(format!("{name}.tmp{}", std::process::id()));
    let result = (|| {
        let mut fh = fs::File::create(&tmp)?;
        fh.write_all(bytes)?;
        fh.sync_all().ok();
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}
