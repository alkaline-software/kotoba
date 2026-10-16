//! Kotoba (言葉): dictation anywhere on this Mac. A key pressed in any
//! app starts it, a local model writes down what was said, and the words
//! are pasted where the cursor is. A menu bar app: a click on its item
//! opens the settings window, Kotoba's only window, and the other click
//! its menu.
//!
//! Dictation is a Mac's alone for now; elsewhere the window builds and
//! says so, and what only dictation uses is left unused there.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

mod assets;
mod controls;
mod fonts;
mod i18n;
mod kotoba;
mod look;
#[cfg(target_os = "macos")]
mod pill;
mod settings;
mod sys;
mod ui;

use gpui::*;
use kotoba::{CloseWindow, Dictate, Escape, Kotoba, KEY_CONTEXT};

actions!(kotoba_app, [Quit, OpenSettings, Hide, HideOthers, ShowAll]);

/// The app's one `Kotoba`, for an action that reached no view.
struct App_(Entity<Kotoba>);

impl Global for App_ {}

fn key_bindings() -> Vec<KeyBinding> {
    let mut keys = vec![
        KeyBinding::new("secondary-q", Quit, None),
        KeyBinding::new("secondary-,", OpenSettings, None),
        KeyBinding::new("secondary-h", Hide, None),
        KeyBinding::new("secondary-alt-h", HideOthers, None),
        KeyBinding::new("secondary-w", CloseWindow, None),
        KeyBinding::new("escape", Escape, Some(KEY_CONTEXT)),
    ];
    // Dictation starts and stops on the key the settings give it, one
    // global binding, while Kotoba is in front; the event tap has it in
    // every other app (`sys::outside_keys`). A tap of modifiers alone is
    // no binding: `Kotoba` watches for it.
    keys.extend(dictate_key(&kotoba_core::config::Config::load().app.dictate_key).map(|k| KeyBinding::new(&k, Dictate, None)));
    keys
}

/// The dictation's key as a binding gpui takes: the one asked for when
/// it is one, none for modifiers tapped alone (`dictate::tap`), else the
/// default.
fn dictate_key(asked: &str) -> Option<String> {
    if kotoba_core::dictate::tap(asked).is_some() {
        return None;
    }
    Some(if Keystroke::parse(asked).is_ok() { asked.to_string() } else { kotoba_core::config::AppConfig::default().dictate_key })
}

/// Dictation moved from the key `old` to `new`, at once: the old key's
/// binding is put out by a later one to nothing, which gpui ranks above
/// it, and the new key is bound after that.
pub fn move_dictate_key(old: &str, new: &str, cx: &mut App) {
    let old = dictate_key(old).map(|k| KeyBinding::new(&k, NoAction, None));
    let new = dictate_key(new).map(|k| KeyBinding::new(&k, Dictate, None));
    cx.bind_keys(old.into_iter().chain(new));
}

/// The action a key already has in the app, by its name, when it has
/// one: a key for dictation must be one nothing else has.
pub fn key_taken(binding: &str) -> Option<&'static str> {
    let asked = Keystroke::parse(binding).ok()?.unparse();
    key_bindings().into_iter().filter(|b| b.action().name() != Dictate.name()).find(|b| b.keystrokes().len() == 1 && b.keystrokes()[0].unparse() == asked).map(|b| b.action().name())
}

/// The menu bar item's menu: each section of the settings, with the
/// rail's icon, then Quit.
/// The app's menu at the screen's top, there while the settings window
/// is open and Kotoba is in the Dock (`sys::in_dock`): the Mac's own
/// choices for an app, in the language in use.
pub fn app_menus() -> Vec<Menu> {
    let t = i18n::t;
    vec![Menu {
        name: "Kotoba".into(),
        disabled: false,
        items: vec![
            MenuItem::action(t("Settings…"), OpenSettings),
            MenuItem::separator(),
            MenuItem::action(t("Hide Kotoba"), Hide),
            MenuItem::action(t("Hide Others"), HideOthers),
            MenuItem::action(t("Show All"), ShowAll),
            MenuItem::separator(),
            MenuItem::action(t("Quit Kotoba"), Quit),
        ],
    }]
}

pub fn menu_lines() -> Vec<sys::MenuLine> {
    let icon = |path: &str| match assets::Assets.load(path) {
        Ok(Some(std::borrow::Cow::Borrowed(bytes))) => bytes,
        _ => &[],
    };
    let mut lines: Vec<sys::MenuLine> = kotoba::SECTIONS.iter().map(|&(key, title, path)| sys::MenuLine { key, title: i18n::t(title), icon: icon(path), keys: "", gap: key == "updates" }).collect();
    lines.push(sys::MenuLine { key: "quit", title: i18n::t("Quit Kotoba"), icon: icon("icons/logout.svg"), keys: "q", gap: true });
    lines
}

fn main() {
    // A first launch is one with no settings written yet: it opens the
    // settings on Permissions when something is still to be given.
    let first = !kotoba_core::paths::config_file().exists();
    let app = gpui_platform::application().with_assets(assets::Assets);
    // Opened again from Finder or Spotlight while running: the settings.
    app.on_reopen(|cx| {
        if let Some(k) = cx.try_global::<App_>().map(|a| a.0.clone()) {
            k.update(cx, |k, cx| k.show_settings(None, cx));
        }
    });
    app.run(move |cx: &mut App| {
        gpui_component::init(cx);
        i18n::set(&i18n::chosen(&kotoba_core::config::Config::load().app.language));
        sys::stay_in_menu_bar();
        let cfg = kotoba_core::config::Config::load();
        // Written once, so the next launch is not a first one.
        if first {
            let _ = kotoba_core::config::Config::edit(|_| {});
        }
        fonts::install(cx);
        look::install(cx);
        look::apply(&cfg.app.appearance, None, cx);

        let kotoba = cx.new(Kotoba::new);
        cx.set_global(App_(kotoba.clone()));

        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &Hide, cx| cx.hide());
        cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
        cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
        cx.set_menus(app_menus());
        cx.on_action(|_: &OpenSettings, cx| {
            if let Some(k) = cx.try_global::<App_>().map(|a| a.0.clone()) {
                k.update(cx, |k, cx| k.show_settings(None, cx));
            }
        });
        // ⌘W closes the settings; a sheet over them is the window's own
        // to close first, by Escape.
        cx.on_action(|_: &CloseWindow, cx| {
            if let Some(w) = cx.active_window() {
                cx.defer(move |cx| {
                    let _ = w.update(cx, |_, window, _| window.remove_window());
                });
            }
        });
        // The key, pressed while no view heard it.
        cx.on_action(|_: &Dictate, cx| {
            if let Some(k) = cx.try_global::<App_>().map(|a| a.0.clone()) {
                k.update(cx, |k, cx| k.toggle_dictation(cx));
            }
        });
        // ggml aborts the process at exit while a model holds the GPU's
        // memory, so each is let go first.
        cx.on_app_quit(|_| {
            kotoba_core::dictate::unload();
            kotoba_core::clean::unload();
            async {}
        })
        .detach();
        cx.bind_keys(key_bindings());

        // The menu bar item: one click opens the settings, the other
        // shows its menu, of the settings' sections and Quit.
        let mut picks = sys::menu_bar(&menu_lines(), cfg.app.bar_swap, cfg.app.bar_icon);
        let k = kotoba.clone();
        cx.spawn(async move |cx| {
            use futures::StreamExt as _;
            while let Some(pick) = picks.next().await {
                let _ = k.update(cx, |k, cx| match pick {
                    sys::MenuPick::Settings => k.show_settings(None, cx),
                    sys::MenuPick::Line("quit") => cx.quit(),
                    sys::MenuPick::Line(section) => k.show_settings(Some(section), cx),
                });
            }
        })
        .detach();

        // `KOTOBA_SETTINGS=<section>` opens the settings on that section;
        // the first launch opens them where the permissions are given.
        let asked = std::env::var("KOTOBA_SETTINGS").ok();
        kotoba.update(cx, |k, cx| {
            if let Some(section) = asked {
                let section = kotoba::SECTIONS.iter().find(|(key, _, _)| *key == section).map(|(key, _, _)| *key);
                k.show_settings(section.or(Some("general")), cx);
            } else if first && kotoba_core::dictate::AVAILABLE && !(crate::sys::trusted() && crate::sys::mic() == sys::Mic::Allowed) {
                k.show_settings(Some("general"), cx);
            }
        });
    });
}
