//! The window's words in the person's language: the one they chose, or
//! the first of the system's preferred languages that Kotoba speaks,
//! else English.
//!
//! English is the key. `locales/words.json` holds, for each English
//! phrase, its every other language; a phrase not there is drawn in
//! English. The languages are a few of the most used (`LANGS`), so the
//! table stays one file a person can read through.
//!
//! What dictation hears and writes is not touched: this is the window's
//! own words, the menu bar's menu and the toolkit's (`set_locale`). The
//! person may set another in General (`CHOICES`), at once.

use std::collections::HashMap;
use std::sync::RwLock;

/// The languages Kotoba speaks besides English: the code in the table,
/// the toolkit's own code for it, and the name the bundle declares
/// (`CFBundleLocalizations`, so System Settings offers each per app).
pub const LANGS: &[(&str, &str, &str)] = &[
    ("zh-Hans", "zh-CN", "zh-Hans"),
    ("zh-Hant", "zh-TW", "zh-Hant"),
    ("ja", "ja", "ja"),
    ("ko", "ko", "ko"),
    ("fr", "fr", "fr"),
    ("de", "de", "de"),
    ("es", "es", "es"),
];

/// What the window's language may be set to, after the system's: each
/// language by its code, its flag and its letters. Traditional Chinese
/// wears Hong Kong's.
pub const CHOICES: &[(&str, &str, &str)] = &[
    ("en", "🇺🇸", "EN"),
    ("zh-Hans", "🇨🇳", "ZH-CN"),
    ("zh-Hant", "🇭🇰", "ZH-HK"),
    ("ja", "🇯🇵", "JA"),
    ("ko", "🇰🇷", "KO"),
    ("fr", "🇫🇷", "FR"),
    ("de", "🇩🇪", "DE"),
    ("es", "🇪🇸", "ES"),
];

const TABLE: &str = include_str!("../locales/words.json");

/// The language in use and its words. It changes when the person picks
/// another, so it is read at every phrase; the words are `'static`, kept
/// for the process's life.
static WORDS: RwLock<Option<(&'static str, HashMap<&'static str, &'static str>)>> = RwLock::new(None);

/// The language for a system's preferred languages (BCP 47, as macOS
/// gives them: `zh-Hans-CN`, `zh-HK`, `fr-CA`), in their order: the
/// first Kotoba speaks, else English. Chinese is told apart by its
/// script, or where none is given, by its region.
pub fn pick<'a>(preferred: impl IntoIterator<Item = &'a str>) -> &'static str {
    for tag in preferred {
        let tag = tag.replace('_', "-").to_ascii_lowercase();
        let mut parts = tag.split('-');
        let lang = parts.next().unwrap_or("");
        let rest: Vec<&str> = parts.collect();
        let code = match lang {
            "en" => "en",
            "zh" if rest.iter().any(|p| *p == "hant" || *p == "tw" || *p == "hk" || *p == "mo") => "zh-Hant",
            "zh" => "zh-Hans",
            other => match LANGS.iter().find(|(c, _, _)| *c == other) {
                Some((c, _, _)) => c,
                None => continue,
            },
        };
        return code;
    }
    "en"
}

/// The language the system would have: its preferred languages, or
/// `KOTOBA_LANG` for a probe.
fn system_lang() -> &'static str {
    match std::env::var("KOTOBA_LANG") {
        Ok(code) => pick([code.as_str()]),
        Err(_) => pick(crate::sys::system_languages().iter().map(String::as_str)),
    }
}

/// What the language is set to: `system`, or a code of `CHOICES`. On a
/// Mac the app's own `AppleLanguages` says, which System Settings may
/// have changed since; elsewhere the settings do.
pub fn chosen(saved: &str) -> String {
    if cfg!(target_os = "macos") {
        return crate::sys::own_language().map_or("system".into(), |code| pick([code.as_str()]).to_string());
    }
    if saved == "system" || CHOICES.iter().any(|(c, _, _)| *c == saved) { saved.to_string() } else { "system".into() }
}

/// The window's language set to `choice` (`system` or a code), and the
/// toolkit's with it.
pub fn set(choice: &str) {
    let lang = if choice == "system" { system_lang() } else { pick([choice]) };
    let toolkit = LANGS.iter().find(|(c, _, _)| *c == lang).map_or("en", |(_, t, _)| *t);
    gpui_component::set_locale(toolkit);
    if let Ok(mut words) = WORDS.write() {
        *words = Some((lang, words_for(lang)));
    }
}

/// The language in use, by its code in the table.
pub fn lang() -> &'static str {
    WORDS.read().ok().and_then(|w| w.as_ref().map(|(l, _)| *l)).unwrap_or("en")
}

fn table() -> HashMap<&'static str, HashMap<&'static str, &'static str>> {
    serde_json::from_str(TABLE).expect("locales/words.json is the table")
}

fn words_for(lang: &str) -> HashMap<&'static str, &'static str> {
    if lang == "en" {
        return HashMap::new();
    }
    table().into_iter().filter_map(|(en, all)| all.get(lang).map(|w| (en, *w))).collect()
}

/// A phrase in the language in use.
pub fn t(en: &'static str) -> &'static str {
    WORDS.read().ok().and_then(|w| w.as_ref().and_then(|(_, words)| words.get(en).copied())).unwrap_or(en)
}

/// A phrase with its blanks filled: `{name}` in it, in either language,
/// takes its value.
pub fn tf(en: &'static str, values: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut s = t(en).to_string();
    for (name, value) in values {
        s = s.replace(&format!("{{{name}}}"), &value.to_string());
    }
    s
}

/// A count with its noun: `one` or `many` in English (`"{n} word"`,
/// `"{n} words"`), by the language's own rule for which, then put in
/// that language with `{n}` filled.
pub fn count(n: usize, one: &'static str, many: &'static str) -> String {
    // French takes the singular for none and one; the others here use
    // one for one alone, or have no plural.
    let single = if lang() == "fr" { n <= 1 } else { n == 1 };
    tf(if single { one } else { many }, &[("n", &n)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_are_picked_from_the_systems_order() {
        assert_eq!(pick(["zh-Hans-CN"]), "zh-Hans");
        assert_eq!(pick(["zh-Hant-TW"]), "zh-Hant");
        assert_eq!(pick(["zh-HK"]), "zh-Hant");
        assert_eq!(pick(["zh-CN"]), "zh-Hans");
        assert_eq!(pick(["fr-CA"]), "fr");
        assert_eq!(pick(["it-IT", "de-AT"]), "de");
        assert_eq!(pick(["it-IT", "pt-BR"]), "en");
        assert_eq!(pick(["en-GB", "ja-JP"]), "en");
        assert_eq!(pick(["ja_JP"]), "ja");
    }

    /// Every phrase in the table is there in every language, with the
    /// same blanks as its English.
    #[test]
    fn every_phrase_is_in_every_language() {
        for (en, all) in table() {
            let blanks = |s: &str| {
                let mut b: Vec<String> = s.split('{').skip(1).filter_map(|p| p.split_once('}').map(|(n, _)| n.to_string())).collect();
                b.sort();
                b
            };
            for (lang, _, _) in LANGS {
                let w = all.get(lang).unwrap_or_else(|| panic!("{en:?} has no {lang}"));
                assert_eq!(blanks(w), blanks(en), "{en:?} in {lang}: {w:?}");
            }
            assert_eq!(all.len(), LANGS.len(), "{en:?} has a language not in LANGS");
        }
    }

    /// Every phrase the app asks for is in the table: each `t("…")`,
    /// `tf("…"` and `count(…, "…", "…")` in its sources, and the words
    /// the core hands the window (the models', the history's lengths).
    #[test]
    fn every_phrase_asked_for_is_in_the_table() {
        let table = table();
        let sources = [
            include_str!("main.rs"),
            include_str!("kotoba.rs"),
            include_str!("settings.rs"),
            include_str!("controls.rs"),
            include_str!("ui.rs"),
            include_str!("sys.rs"),
            include_str!("pill.rs"),
        ];
        let mut asked: Vec<String> = Vec::new();
        for src in sources {
            for call in ["t", "tf", "count"] {
                let opener = format!("{call}(");
                let mut from = 0;
                while let Some(at) = src[from..].find(&opener).map(|i| i + from) {
                    from = at + opener.len();
                    // A call of its own, not the end of another name.
                    if src[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                        continue;
                    }
                    let rest = &src[from..];
                    // The quoted phrases among the call's first arguments.
                    let quoted: Vec<&str> = rest.split('"').collect();
                    if call == "count" {
                        if quoted.len() > 3 && !quoted[0].contains(')') {
                            asked.push(quoted[1].to_string());
                            asked.push(quoted[3].to_string());
                        }
                    } else if rest.starts_with('"') && quoted.len() > 1 {
                        asked.push(quoted[1].to_string());
                    }
                }
            }
        }
        for m in kotoba_core::dictate::MODELS.iter().chain(kotoba_core::clean::MODELS.iter()) {
            asked.push(m.about.to_string());
            asked.push(m.languages.to_string());
        }
        for (_, label, _) in kotoba_core::dictate::KEEPS {
            asked.push(label.to_string());
        }
        asked.extend(crate::ui::MONTHS.iter().map(|m| m.to_string()));
        asked.extend(crate::kotoba::SECTIONS.iter().map(|(_, name, _)| name.to_string()));
        let missing: Vec<&String> = asked.iter().filter(|p| !p.is_empty() && !table.contains_key(p.as_str())).collect();
        assert!(missing.is_empty(), "not in locales/words.json: {missing:#?}");
        let unused: Vec<&&str> = table.keys().filter(|k| !asked.iter().any(|a| a == *k)).collect();
        assert!(unused.is_empty(), "in locales/words.json but never asked for: {unused:#?}");
    }
}
