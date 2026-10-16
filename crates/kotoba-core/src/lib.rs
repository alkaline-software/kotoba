//! kotoba-core: everything that does not need a window.
//!
//! The speech models and the microphone (`dictate`), the post-processing
//! that rewrites what they wrote (`clean`), the settings and the folders
//! under `~/.kotoba` (`config`, `paths`), and the newest release
//! (`update`).

pub mod clean;
pub mod config;
pub mod dictate;
pub mod paths;
pub mod update;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
