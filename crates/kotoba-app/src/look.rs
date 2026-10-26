//! The window's look: which appearance to draw, as `config.json` asks
//! (the system's, or light or dark pinned), and the accent, green.
//!
//! The palette is in `themes/kotoba.json`, one config per appearance. An
//! accent is a substitution over the handful of keys that carry it there
//! (the primary button, the focus ring, links, the active row's border,
//! the selection tint), painted into both configs before they are handed
//! to the toolkit, so `Theme::change` keeps it on every appearance switch
//! with nothing to patch afterwards.

use gpui::{App, Window};
use gpui_component::theme::{Theme, ThemeConfig, ThemeMode, ThemeSet};

/// The accent: its base, hover and active shades in each appearance, and
/// the link's shade in each. A menu bar app's one window has no need of a
/// choice of accents: there is one.
struct Accent {
    light: [&'static str; 3],
    dark: [&'static str; 3],
    link: [&'static str; 2],
}

/// The icon's green, at the leaf's hue (147 in OKLCH) and the studio's
/// lightness for an accent: 0.52 in the light appearance and 0.74 in the
/// dark, its chroma eased to 0.084 as the studio's greens are. Hover is
/// 0.02 darker in the light appearance and 0.04 lighter in the dark;
/// active 0.06 and 0.04 darker. The link is the hover shade in the light
/// appearance and the base in the dark.
const GREEN: Accent = Accent { light: ["#46764C", "#417046", "#35653B"], dark: ["#88B98C", "#94C699", "#7CAD80"], link: ["#417046", "#88B98C"] };

/// What is drawn on the accent: white on the light appearance's shade,
/// the ink on the dark one's, 5.3 and 7.4 to 1.
const ON_LIGHT: &str = "#FFFFFF";
const ON_DARK: &str = "#1F1E1D";

/// The inks the palette names by name ("@l-blue", "@d-rose"), one set an
/// appearance, for the code styles the theme file carries.
struct Inks {
    blue: u32,
    violet: u32,
    gold: u32,
    rose: u32,
    teal: u32,
    green: u32,
    red: u32,
    gray: u32,
}

const INKS_LIGHT: Inks = Inks { blue: 0x286983, violet: 0x7A6398, gold: 0x8A6A1E, rose: 0xA3566F, teal: 0x357783, green: 0x4C7B58, red: 0xB0505F, gray: 0x85818F };
const INKS_DARK: Inks = Inks { blue: 0x7FB4D0, violet: 0xC4A7E7, gold: 0xE9C083, rose: 0xE8B0AE, teal: 0x9CCFD8, green: 0xA3CCA4, red: 0xEB8FA6, gray: 0x8F8BA0 };

/// The theme file with each ink's name replaced by its color.
fn inked(json: &str) -> String {
    let mut out = json.to_string();
    for (p, inks) in [("l", &INKS_LIGHT), ("d", &INKS_DARK)] {
        for (name, c) in [("blue", inks.blue), ("violet", inks.violet), ("gold", inks.gold), ("rose", inks.rose), ("teal", inks.teal), ("green", inks.green), ("red", inks.red), ("gray", inks.gray)] {
            out = out.replace(&format!("\"@{p}-{name}\""), &format!("\"#{c:06X}\""));
        }
    }
    out
}

fn paint(cfg: &mut ThemeConfig, a: &Accent) {
    let dark = cfg.mode.is_dark();
    let [base, hover, active] = if dark { a.dark } else { a.light };
    let link = if dark { a.link[1] } else { a.link[0] };
    let on = if dark { ON_DARK } else { ON_LIGHT };
    let c = &mut cfg.colors;
    c.primary = Some(base.into());
    c.primary_foreground = Some(on.into());
    c.sidebar_primary_foreground = Some(on.into());
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
