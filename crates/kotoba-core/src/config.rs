//! Configuration: defaults and `~/.kotoba/config.json`, the settings under
//! `app`. Loading never fails; a malformed file falls back to the defaults.

use serde::{Deserialize, Serialize};

use crate::paths;

/// What the settings hold. Every field has a spelled-out default so a
/// hand-edited `config.json` with one of them missing still loads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// `system` follows the OS; `light` and `dark` pin one look.
    pub appearance: String,
    /// Ask GitHub for the newest release once a day and say so in the
    /// settings window when there is one. Nothing is installed unasked.
    pub check_updates: bool,
    /// The dictation's model, by `dictate::MODELS`' keys: one at a time.
    pub dictate_model: String,
    /// The key that starts and stops dictation, as gpui writes a binding
    /// (`secondary-shift-d`: ⌘⇧D, Ctrl off a Mac), or modifiers alone as
    /// `tap:<key>` (`dictate::TAPS`).
    pub dictate_key: String,
    /// How long the dictation's history keeps what it wrote, by
    /// `dictate::KEEPS`' keys.
    pub dictate_keep: String,
    /// Whether what dictation writes is cleaned up by a model after
    /// (`clean`), and by which, by `clean::MODELS`' keys.
    pub clean_on: bool,
    pub clean_model: String,
    /// The person's own instructions to the clean-up's model, put first
    /// in its system turn; empty for none.
    pub clean_prompt: String,
    /// Words the person wants written their way, capitals and all: told
    /// to the speech model and the clean-up's, and spelled so after
    /// (`dictate::spell`).
    pub dictate_terms: Vec<String>,
    /// Words the person never wants written, left out of what was heard
    /// and of what post-processing wrote (`dictate::avoid`).
    pub dictate_avoid: Vec<String>,
    /// A left click on the menu bar's icon shows its menu, and a right
    /// click opens the settings; off, the other way round.
    pub bar_swap: bool,
    /// Whether the icon is in the menu bar. Hidden, Kotoba is reached by
    /// opening it again, from Finder or Spotlight.
    pub bar_icon: bool,
    /// The window's language: `system` follows the system's preferred
    /// languages, or a code of one Kotoba speaks (`ja`, `zh-Hant`). On a
    /// Mac the app's own `AppleLanguages` is what is read, the setting
    /// System Settings gives each app; this is kept beside it.
    pub language: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            appearance: "system".into(),
            check_updates: true,
            dictate_model: crate::dictate::DEFAULT_MODEL.into(),
            dictate_key: "secondary-shift-d".into(),
            dictate_keep: crate::dictate::DEFAULT_KEEP.into(),
            clean_on: false,
            clean_model: crate::clean::DEFAULT_MODEL.into(),
            clean_prompt: String::new(),
            dictate_terms: Vec::new(),
            dictate_avoid: Vec::new(),
            bar_swap: false,
            bar_icon: true,
            language: "system".into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub app: AppConfig,
}

impl Config {
    pub fn load() -> Config {
        paths::read_json(&paths::config_file()).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        paths::ensure_dirs()?;
        paths::write_json(&paths::config_file(), &serde_json::to_value(self)?)
    }

    /// Change the file on disk: what is there now is read, changed and
    /// written back through a temp file and a rename, so a reader never
    /// sees half of it.
    pub fn edit(change: impl FnOnce(&mut Config)) -> std::io::Result<()> {
        let mut on_disk = Config::load();
        change(&mut on_disk);
        on_disk.save()
    }
}
