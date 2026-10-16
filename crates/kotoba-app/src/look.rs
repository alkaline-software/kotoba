//! The window's look: which appearance to draw, as `config.json` asks
//! (the system's, or light or dark pinned), and the accent, green.
//!
//! The palette is Emaki's, in `themes/kotoba.json`, one config per
//! appearance. An accent is a substitution over the handful of keys that
//! carry the accent there (the primary button, the focus ring, links,
//! the active row's border, the selection tint), applied to both configs
//! before they are handed to the toolkit, so `Theme::change` keeps
//! re-applying the accent on every appearance switch with nothing to
//! patch afterwards. Kotoba's accent is the icon's green, as Emaki's is
//! terracotta.

use gpui::{App, Window};
use gpui_component::theme::{Theme, ThemeConfig, ThemeMode, ThemeSet};

/// The accent: the colour it paints in each appearance (base, hover,
/// active) and the link colour, darker in the light palette and lighter
/// in the dark one so text stays readable. The icon's green. A menu bar
/// app's one window has no need of a choice of accents: there is one.
struct Accent {
    light: [&'static str; 3],
    dark: [&'static str; 3],
    link: [&'static str; 2],
}

const GREEN: Accent = Accent { light: ["#519F5C", "#469151", "#3D8348"], dark: ["#519F5C", "#61A96A", "#3D8348"], link: ["#3D8348", "#76B57D"] };

/// The inks the palette names by name ("@l-blue", "@d-rose"), one set an
/// appearance: Emaki's, for code, which the theme file still carries.
struct Inks {
    blue: u32,
    violet: u32,
    gold: u32,
    rose: u32,
    teal: u32,
    green: u32,
    red: u32,
    grey: u32,
}

const INKS_LIGHT: Inks = Inks { blue: 0x286983, violet: 0x7A6398, gold: 0x8A6A1E, rose: 0xA3566F, teal: 0x357783, green: 0x4C7B58, red: 0xB0505F, grey: 0x85818F };
const INKS_DARK: Inks = Inks { blue: 0x7FB4D0, violet: 0xC4A7E7, gold: 0xE9C083, rose: 0xE8B0AE, teal: 0x9CCFD8, green: 0xA3CCA4, red: 0xEB8FA6, grey: 0x8F8BA0 };

/// The theme file with each ink's name replaced by its colour.
fn inked(json: &str) -> String {
    let mut out = json.to_string();
    for (p, inks) in [("l", &INKS_LIGHT), ("d", &INKS_DARK)] {
        for (name, c) in [("blue", inks.blue), ("violet", inks.violet), ("gold", inks.gold), ("rose", inks.rose), ("teal", inks.teal), ("green", inks.green), ("red", inks.red), ("grey", inks.grey)] {
            out = out.replace(&format!("\"@{p}-{name}\""), &format!("\"#{c:06X}\""));
        }
    }
    out
}

fn paint(cfg: &mut ThemeConfig, a: &Accent) {
    let dark = cfg.mode.is_dark();
    let [base, hover, active] = if dark { a.dark } else { a.light };
    let link = if dark { a.link[1] } else { a.link[0] };
    let c = &mut cfg.colors;
    c.primary = Some(base.into());
    c.primary_hover = Some(hover.into());
    c.primary_active = Some(active.into());
    c.ring = Some(base.into());
    c.link = Some(link.into());
    c.link_hover = Some(active.into());
    c.link_active = Some(active.into());
    c.sidebar_primary = Some(base.into());
    c.list_active_border = Some(base.into());
    c.drag_border = Some(base.into());
    c.progress_bar = Some(base.into());
    c.slider_bar = Some(base.into());
    c.drop_target = Some(format!("{base}33").into());
    c.selection = Some(format!("{base}{}", if dark { "66" } else { "59" }).into());
}

/// Hand the toolkit both palettes with the accent painted in, once at
/// start; `apply` then draws it.
pub fn install(cx: &mut App) {
    let set: ThemeSet = match serde_json::from_str(&inked(include_str!("../themes/kotoba.json"))) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("kotoba: theme file is invalid, using the default look: {e}");
            return;
        }
    };
    let a = &GREEN;
    let theme = Theme::global_mut(cx);
    for mut cfg in set.themes {
        paint(&mut cfg, a);
        let cfg = std::rc::Rc::new(cfg);
        if cfg.mode.is_dark() {
            theme.dark_theme = cfg;
        } else {
            theme.light_theme = cfg;
        }
    }
}

/// Draw the appearance `config.json` asks for: the system's when
/// `appearance` is `system`, otherwise the one named. Called at start, on
/// every system appearance change, and from the settings.
pub fn apply(appearance: &str, window: Option<&mut Window>, cx: &mut App) {
    match appearance {
        "light" => Theme::change(ThemeMode::Light, window, cx),
        "dark" => Theme::change(ThemeMode::Dark, window, cx),
        _ => Theme::sync_system_appearance(window, cx),
    }
}
