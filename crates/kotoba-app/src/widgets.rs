//! The shared drawing: the button (a pill with an icon), keys drawn on
//! caps, hover labels, a page fading in, the floating card's shadow, the
//! motion's lengths, and the click that is swallowed without leaving a
//! text drag open.

use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{h_flex, Icon, Sizable as _};

/// A list or a sheet comes in over this long, and goes over the next.
pub const MENU_IN: Duration = Duration::from_millis(140);
pub const MENU_OUT: Duration = Duration::from_millis(110);
/// What opens under a row, and what fades as a choice changes.
pub const FOLD_ANIM: Duration = Duration::from_millis(200);

/// Swallow a click without leaving the toolkit's text selection mid-drag.
/// gpui-component's window selection layer begins a drag on every left
/// mouse-down and ends it on the bubble-phase mouse-up; a click handler that
/// only stops propagation eats that mouse-up, and every later mouse move
/// then extends a selection nobody is making.
pub fn swallow_click(window: &mut Window, cx: &mut App) {
    gpui_base::TextSelection::end(window, cx);
    cx.stop_propagation();
}

/// The shadow under a floating card (a sheet, a list): a wide soft drop
/// in the ink's own hue and a hairline of contact under it, so the card
/// lifts off the page rather than sitting in a gray halo.
pub fn float_shadow(theme: &gpui_component::Theme) -> Vec<BoxShadow> {
    let dark = theme.mode.is_dark();
    let ink = if dark { gpui::black() } else { theme.foreground };
    vec![
        BoxShadow { color: ink.opacity(if dark { 0.45 } else { 0.08 }), offset: point(px(0.), px(12.)), blur_radius: px(32.), spread_radius: px(-8.), inset: false },
        BoxShadow { color: ink.opacity(if dark { 0.3 } else { 0.05 }), offset: point(px(0.), px(1.)), blur_radius: px(3.), spread_radius: px(0.), inset: false },
    ]
}

/// A page of the settings coming in: it fades in, rising a little.
pub fn page_in(id: &'static str, el: Div) -> AnyElement {
    el.with_animation(ElementId::Name(id.into()), Animation::new(Duration::from_millis(240)).with_easing(ease_out_quint()), |d, t| d.opacity(t).pt(px(8. * (1. - t)))).into_any_element()
}

/// The months, as the dates in the history name them in English.
pub const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// "Today", "Yesterday", or the date, for dictation's history, in the
/// language in use: the weekday and the month by their names in it, and
/// the order its dates are written in.
pub fn said_day(d: chrono::NaiveDate) -> String {
    use chrono::Datelike;
    use crate::i18n::{t, tf};
    let today = chrono::Local::now().date_naive();
    if d == today {
        return t("Today").into();
    }
    if Some(d) == today.pred_opt() {
        return t("Yesterday").into();
    }
    let month = t(MONTHS[d.month0() as usize]);
    if d.year() == today.year() {
        let weekday = match d.weekday() {
            chrono::Weekday::Mon => t("Monday"),
            chrono::Weekday::Tue => t("Tuesday"),
            chrono::Weekday::Wed => t("Wednesday"),
            chrono::Weekday::Thu => t("Thursday"),
            chrono::Weekday::Fri => t("Friday"),
            chrono::Weekday::Sat => t("Saturday"),
            chrono::Weekday::Sun => t("Sunday"),
        };
        tf("{weekday}, {month} {day}", &[("weekday", &weekday), ("month", &month), ("day", &d.day())])
    } else {
        tf("{month} {day}, {year}", &[("month", &month), ("day", &d.day()), ("year", &d.year())])
    }
}

/// What a button is, which is how its pill is drawn.
#[derive(Clone, Copy, PartialEq)]
pub enum Press {
    /// One of several, or the way out: an outline.
    Quiet,
    /// What cannot be taken back: the danger's ink and outline.
    Danger,
    /// The answer the page is for: filled with the accent.
    Primary,
}

/// The window's button: a pill with an icon before its word. Every
/// button is one. `off` is one that cannot be pressed now: paler, and
/// the caller gives it no click (`.when(!off, ..)`).
pub fn choice_pill(id: impl Into<ElementId>, icon: &'static str, label: impl Into<SharedString>, kind: Press, off: bool, theme: &gpui_component::Theme) -> Stateful<Div> {
    let ink = match kind {
        Press::Danger => theme.danger,
        Press::Primary => theme.primary_foreground,
        Press::Quiet => theme.foreground,
    };
    let (ground, edge, hover) = match kind {
        Press::Danger => (None, theme.danger.opacity(0.35), theme.danger.opacity(0.1)),
        Press::Primary => (Some(theme.primary), theme.primary, theme.primary.opacity(0.85)),
        // An edge of the ink, not the border's color: on a card the
        // border is the card's own shade and the pill had no outline.
        Press::Quiet => (None, theme.foreground.opacity(0.16), theme.foreground.opacity(0.06)),
    };
    h_flex()
        .id(id)
        .h(px(28.))
        .px(px(12.))
        .gap(px(6.))
        .flex_shrink_0()
        .items_center()
        .rounded_full()
        .text_size(px(12.5))
        .text_color(ink)
        .border_1()
        .border_color(edge)
        .when_some(ground, |d, bg| d.bg(bg))
        .when(!icon.is_empty(), |d| d.child(Icon::default().path(icon).with_size(px(14.)).text_color(ink)))
        .child(label.into())
        .map(|d| if off { d.opacity(0.4) } else { d.cursor_pointer().hover(move |s| s.bg(hover)) })
}

/// The button (`choice_pill`) in the size a row of the settings takes.
pub fn small_pill(id: impl Into<ElementId>, icon: &'static str, label: impl Into<SharedString>, kind: Press, off: bool, theme: &gpui_component::Theme) -> Stateful<Div> {
    choice_pill(id, icon, label, kind, off, theme).h(px(24.)).px(px(10.)).gap(px(5.)).text_size(px(12.))
}

/// The icon a key's sign is drawn with: none for a character that is
/// not one.
fn key_sign(c: char) -> Option<&'static str> {
    Some(match c {
        '⌘' => "icons/key-command.svg",
        '⌥' => "icons/key-option.svg",
        '⇧' => "icons/key-shift.svg",
        '⌃' => "icons/chevron-up.svg",
        '🌐' => "icons/globe.svg",
        '↩' => "icons/key-return.svg",
        '⇥' => "icons/key-tab.svg",
        '↑' => "icons/arrow-up.svg",
        '↓' => "icons/arrow-down.svg",
        _ => return None,
    })
}

/// A piece of a line that names keys: words, or one shortcut's keys.
enum KeyPiece {
    Words(String),
    Keys(Vec<String>),
}

/// A word a sign can stand before as one key: "⇧Tab".
const KEY_WORDS: [&str; 5] = ["Tab", "Esc", "Enter", "Space", "Return"];

/// The signs a key is written with.
const KEY_SIGNS: [char; 9] = ['⌘', '⌥', '⇧', '⌃', '🌐', '↩', '⇥', '↑', '↓'];

/// How long an F key's name (F1 to F20) is at the head of `s`, when one
/// is there and is a word of its own.
fn f_word(s: &str) -> Option<usize> {
    let digits: String = s.strip_prefix('F')?.chars().take_while(char::is_ascii_digit).collect();
    let after = s.chars().nth(1 + digits.len());
    (kotoba_core::dictate::f_key(&format!("f{digits}")).is_some() && after.is_none_or(|c| !c.is_alphanumeric())).then_some(1 + digits.len())
}

/// A key's cap is this tall, and no narrower.
const KEY_CAP: f32 = 17.;

/// A line cut into its words and its shortcuts. A shortcut is a run of
/// signs with the one key after them (a letter, a digit, a mark, or one
/// of `KEY_WORDS`), or signs alone ("⇧↩"). Brackets that hold nothing
/// but shortcuts are dropped: the caps set them apart already.
fn key_pieces(text: &str) -> Vec<KeyPiece> {
    // An F key alone, as dictation's key is shown: one cap.
    if f_word(text) == Some(text.chars().count()) {
        return vec![KeyPiece::Keys(vec![text.to_string()])];
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<KeyPiece> = Vec::new();
    let mut words = String::new();
    let mut i = 0;
    while i < chars.len() {
        if !KEY_SIGNS.contains(&chars[i]) {
            words.push(chars[i]);
            i += 1;
            continue;
        }
        let mut keys: Vec<String> = Vec::new();
        while i < chars.len() && KEY_SIGNS.contains(&chars[i]) {
            keys.push(chars[i].to_string());
            i += 1;
        }
        // The key the signs go with, when the next thing is one.
        let rest: String = chars[i..].iter().collect();
        if let Some(w) = KEY_WORDS.iter().find(|w| rest.starts_with(**w)) {
            keys.push(w.to_string());
            i += w.chars().count();
        } else if let Some(n) = f_word(&rest) {
            keys.push(rest.chars().take(n).collect());
            i += n;
        } else if let Some(c) = chars.get(i).filter(|c| !c.is_whitespace() && !matches!(c, ')' | '(')) {
            let alone = chars.get(i + 1).is_none_or(|n| !n.is_alphanumeric());
            if alone && (c.is_alphanumeric() || matches!(c, ',' | '.' | '/' | '[' | ']' | '-' | '=')) && keys.iter().any(|k| k == "⌘" || k == "⌥") {
                keys.push(c.to_uppercase().to_string());
                i += 1;
            }
        }
        if !words.is_empty() {
            out.push(KeyPiece::Words(std::mem::take(&mut words)));
        }
        out.push(KeyPiece::Keys(keys));
    }
    if !words.is_empty() {
        out.push(KeyPiece::Words(words));
    }
    // "(⌘K)" and "(⇧↩, ⌘⇧G)": the brackets go.
    let mut at = 0;
    while at < out.len() {
        let opens = matches!(&out[at], KeyPiece::Words(w) if w.ends_with('('));
        if opens {
            let mut end = at + 1;
            while matches!(out.get(end), Some(KeyPiece::Keys(_))) && matches!(out.get(end + 1), Some(KeyPiece::Words(w)) if w == ", ") {
                end += 2;
            }
            if matches!(out.get(end), Some(KeyPiece::Keys(_))) && matches!(out.get(end + 1), Some(KeyPiece::Words(w)) if w.starts_with(')')) {
                if let KeyPiece::Words(w) = &mut out[at] {
                    w.pop();
                }
                if let KeyPiece::Words(w) = &mut out[end + 1] {
                    w.remove(0);
                }
            }
        }
        at += 1;
    }
    out.retain(|p| !matches!(p, KeyPiece::Words(w) if w.is_empty()));
    out
}

/// One key on its cap: a small plate, every one the same height and no
/// narrower than that, with the key in its middle. A sign is an icon and
/// a letter is a letter, both of one size; set in the middle of equal
/// plates they are level and read as one size.
fn key_cap(key: &str) -> Div {
    let sign = key.chars().next().filter(|_| key.chars().count() == 1).and_then(key_sign);
    div()
        .flex_shrink_0()
        .h(px(KEY_CAP))
        .min_w(px(KEY_CAP))
        .px(px(3.5))
        .rounded(px(4.5))
        .bg(gpui::hsla(0., 0., 0.5, 0.17))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(10.5))
        .line_height(px(KEY_CAP))
        .font_weight(FontWeight::MEDIUM)
        .map(|d| match sign {
            Some(path) => d.child(Icon::default().path(path).with_size(px(10.5))),
            None => d.child(key.to_string()),
        })
}

/// Words that name keys, written as the Mac has them ("Add (↩)"), with
/// each shortcut drawn as its keys on caps (`key_cap`) after the words.
/// **Every place the window shows a key goes through this** (`tip`, or
/// this itself for a line of words). The words take the size and color
/// of where they stand. It does not wrap.
pub fn key_text(text: &str) -> Div {
    let mut row = h_flex().flex_shrink_0().items_center();
    for piece in key_pieces(text) {
        row = match piece {
            KeyPiece::Words(w) => row.child(div().flex_shrink_0().child(w.replace(' ', "\u{a0}"))),
            KeyPiece::Keys(keys) => row.child(h_flex().flex_shrink_0().gap(px(2.)).mx(px(1.)).children(keys.iter().map(|k| key_cap(k)))),
        };
    }
    row
}

/// `key_text` that wraps, for a line of prose that names a key: each word
/// and each shortcut is a piece the line can break between.
pub fn key_text_wrap(text: &str) -> Div {
    let mut row = h_flex().flex_wrap().items_center().gap_x(px(3.5)).gap_y(px(2.));
    for piece in key_pieces(text) {
        row = match piece {
            KeyPiece::Words(w) => row.children(w.split_whitespace().map(|word| div().flex_shrink_0().child(word.to_string()))),
            KeyPiece::Keys(keys) => row.child(h_flex().flex_shrink_0().gap(px(2.)).children(keys.iter().map(|k| key_cap(k)))),
        };
    }
    row
}

/// Keys held together, as a tap of two is written ("⌥ + ⌘"): each on its
/// caps, with a quiet "+" between.
pub fn key_combo(text: &str, quiet: Hsla) -> Div {
    let mut row = h_flex().flex_shrink_0().items_center().gap(px(5.));
    for (ix, part) in text.split(" + ").enumerate() {
        if ix > 0 {
            row = row.child(div().text_color(quiet).font_weight(FontWeight::NORMAL).child("+"));
        }
        row = row.child(key_text(part));
    }
    row
}

/// A binding as gpui writes it (`secondary-shift-d`) in the Mac's signs
/// ("⌘⇧D"), for `key_text`.
pub fn binding_signs(binding: &str) -> String {
    let parts: Vec<&str> = binding.split('-').collect();
    let (mods, key) = parts.split_at(parts.len().saturating_sub(1));
    let mut out = String::new();
    for sign in [("secondary", "⌘"), ("cmd", "⌘"), ("alt", "⌥"), ("shift", "⇧")] {
        if mods.contains(&sign.0) && !out.contains(sign.1) {
            out.push_str(sign.1);
        }
    }
    let key = key.first().copied().unwrap_or("");
    out.push_str(&match key {
        "space" => "Space".to_string(),
        "enter" => "↩".to_string(),
        "tab" => "⇥".to_string(),
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        k if k.chars().count() == 1 => k.to_uppercase(),
        k => k.chars().next().map(|c| c.to_uppercase().collect::<String>() + &k[c.len_utf8()..]).unwrap_or_default(),
    });
    out
}

/// Dictation's key in the Mac's signs, for `key_text`: a binding's
/// ("⌘⇧D") or a tap's ("Right ⌘").
pub fn dictate_key_signs(key: &str) -> String {
    kotoba_core::dictate::tap(key).map(tap_signs).unwrap_or_else(|| binding_signs(key))
}

/// Modifiers tapped alone as the window shows them: their signs, with
/// the word for the right-hand one in the language in use.
pub fn tap_signs(tap: &kotoba_core::dictate::Tap) -> String {
    tap.signs.replace("Right", crate::i18n::t("Right"))
}

/// A hover's label, with the keys it names drawn (`key_text`): what
/// every `managed_tooltip` in the window builds.
pub fn tip(text: impl Into<SharedString>, window: &mut Window, cx: &mut App) -> AnyView {
    let text: SharedString = text.into();
    gpui_component::tooltip::Tooltip::element(move |_, _| key_text(&text)).build(window, cx)
}
