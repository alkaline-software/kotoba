//! The settings window, Kotoba's only window: a rail of sections at the
//! left, under the window's traffic lights, and the chosen section's rows
//! at the right, as Emaki's settings panel has them. Dictate is Emaki's
//! Settings, Dictate; Permissions asks for the microphone and
//! Accessibility ahead of the first dictation; Appearance, Updates and
//! About are Emaki's. History, Preferences and Terms open on sheets over
//! it (`over_settings`).

use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, Textarea};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::tooltip::ManagedTooltipExt as _;
use gpui_component::{h_flex, v_flex, ActiveTheme as _, Icon, IconName, Sizable as _};
use kotoba_core::{dictate, update};

use crate::kotoba::{Dictate, Escape, Inner, Kotoba, Lane, PickFor, WordList, KEY_CONTEXT, SECTIONS};
use crate::i18n::{count, t, tf};
use crate::sys::{Login, Mic};
use crate::ui::{choice_pill, dictate_key_signs, float_shadow, key_combo, key_text, key_text_wrap, page_in, said_day, small_pill, swallow_click, tip, Press, FOLD_ANIM, MENU_IN, MENU_OUT};

/// The rail's width, and the page's margin at either side.
const RAIL_W: Pixels = px(190.);
const PAGE_X: Pixels = px(36.);
/// What the widest controls at a row's right take (the key with More and
/// Reset, the history's), so those on a page start and end together.
const CONTROL_W: Pixels = px(336.);
/// The key's field and the history's list, and the key's More and Reset.
const FIELD_W: Pixels = px(160.);
const DICTATE_PILL_W: Pixels = px(80.);
/// A permission's state or button, and the update's button.
const PERMIT_W: Pixels = px(150.);
const UPDATE_W: Pixels = px(190.);
/// The language's list at General's top right.
const LANGUAGE_W: Pixels = px(136.);
/// The two slots at a model's right, one width, so every button down a
/// lane is one size and stands in one column.
const MODEL_SLOT_W: Pixels = px(96.);
/// The preferences' field: this tall, or in a short window what room
/// is left above it (the window's top, the field's name) and below it
/// (Terms and Avoid, the page's foot), and never shorter than the least.
/// Filling the window was tried: the field outgrew what is typed in it.
const PREFERENCES_H: Pixels = px(300.);
const PREFERENCES_ROOM: Pixels = px(244.);
const PREFERENCES_LEAST: Pixels = px(160.);
/// The window's top, clear for the traffic lights and dragged by.
const TOP_H: Pixels = px(52.);

impl Render for Kotoba {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.viewport_h = window.viewport_size().height;
        // The scrollers name themselves again as they are drawn; a kept
        // handle the last draw did not use goes.
        self.inner_live.borrow_mut().clear();
        self.inner_kept.borrow_mut().retain(|_, kept| std::mem::take(&mut kept.1));
        self.settings_glide_step(window);
        let theme = cx.theme().clone();
        div()
            .id("kotoba")
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .size_full()
            .relative()
            .bg(theme.popover)
            .text_color(theme.foreground)
            // Escape stops dictation before anything else it does, in
            // whatever field has the keys.
            .capture_action(cx.listener(|this, _: &Escape, _, cx| {
                if this.cancel_dictation(cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &gpui_component::input::Escape, _, cx| {
                if this.cancel_dictation(cx) {
                    cx.stop_propagation();
                }
            }))
            .on_action(cx.listener(|this, _: &Escape, window, cx| {
                if this.pick.is_some() {
                    this.close_pick(cx);
                } else if this.words_confirm.is_some() {
                    this.words_confirm = None;
                    cx.notify();
                } else if this.words_sheet.is_some() {
                    this.close_words(window, cx);
                } else if this.dictated.is_some() {
                    this.close_dictated(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Dictate, _, cx| this.toggle_dictation(cx)))
            // A raw wheel listener in the capture phase, registered at
            // paint (nothing is drawn): see `route_scroll`.
            .child({
                let this = cx.entity().downgrade();
                canvas(
                    |_, _, _| (),
                    move |_, _, window, _| {
                        let scroll = this.clone();
                        window.on_mouse_event(move |e: &ScrollWheelEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture {
                                let _ = scroll.update(cx, |k, cx| k.route_scroll(e, cx));
                            }
                        });
                    },
                )
                .absolute()
                .size_0()
            })
            // A new language: the window fades into it, once.
            .child({
                let page = div().size_full().child(self.render_settings(cx));
                match self.lang_serial {
                    0 => page.into_any_element(),
                    n => page.with_animation(ElementId::Name(format!("language-{n}").into()), Animation::new(std::time::Duration::from_millis(260)).with_easing(ease_out_quint()), |d, t| d.opacity(0.3 + 0.7 * t)).into_any_element(),
                }
            })
            .when_some(self.dictated_gone.clone().filter(|(_, _, at)| at.elapsed() < MENU_OUT), |d, (all, serial, _)| d.child(self.render_dictated(all, serial, true, cx)))
            .when_some(self.dictated.clone(), |d, (all, serial)| d.child(self.render_dictated(all, serial, false, cx)))
            .when_some(self.words_gone.filter(|(_, _, at)| at.elapsed() < MENU_OUT), |d, (list, serial, _)| d.child(self.render_words(list, serial, true, cx)))
            .when_some(self.words_sheet, |d, (list, serial)| d.child(self.render_words(list, serial, false, cx)))
            .when_some(self.pick_gone.clone().filter(|(_, at)| at.elapsed() < MENU_OUT), |d, (m, _)| d.child(self.render_pick(m, true, window, cx)))
            // Where its field is now, not where it was when the list opened.
            .when_some(self.pick.clone(), |d, mut m| {
                m.by = self.pill_at.borrow().get(&m.what).copied().unwrap_or(m.by);
                d.child(self.render_pick(m, false, window, cx))
            })
    }
}

impl Kotoba {
    fn render_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // The page's scroller is named before anything on it, so a field
        // on the page that scrolls itself (Preferences) is found over it.
        self.inner_scroller(&self.settings_scroll, Inner::Over);
        let theme = cx.theme().clone();
        let app = self.cfg.app.clone();

        let link_color = theme.muted_foreground;
        let link_hover = theme.primary;
        let said = |t: &'static str| Some(div().child(t).into_any_element());

        let section = self.section;
        let rows: Vec<AnyElement> = match section {
            "models" => self.models_rows(cx),
            "customize" => self.customize_rows(cx),
            // General: dictation's key and history, how the app stands on
            // this Mac, and what the system has to allow.
            "general" => {
                let appearance_key = match app.appearance.as_str() {
                    "light" => "light",
                    "dark" => "dark",
                    _ => "system",
                };
                let appearance = div().w(CONTROL_W).child(self.segmented("appearance", vec![("system", t("System").into()), ("light", t("Light").into()), ("dark", t("Dark").into())], appearance_key, Rc::new(|this, key, window, cx| this.set_appearance(key, window, cx)), cx)).into_any_element();
                // Opening at login is the system's to keep: while it waits
                // on the person in Login Items, the way there stands
                // before the switch.
                let waiting = self.login == Login::Waiting;
                let login = h_flex()
                    .gap(px(10.))
                    .items_center()
                    .when(waiting, |d| d.child(small_pill("login-items", "icons/external-link.svg", t("Login Items"), Press::Quiet, false, &theme).h(px(28.)).on_click(|_, _, _| crate::sys::open_login_items())))
                    .child(self.switch("login", self.login != Login::Off, Rc::new(|this, on, _, cx| this.set_login(on, cx)), cx))
                    .into_any_element();
                let shown = self.switch("bar-icon", app.bar_icon, Rc::new(|this, on, _, cx| this.set_bar_icon(on, cx)), cx);
                let swap = self.switch("bar-swap", app.bar_swap, Rc::new(|this, on, _, cx| this.set_bar_swap(on, cx)), cx);
                // Each permission: whether it is given, and the one thing
                // to do about it when it is not. Asked here ahead of the
                // first dictation, so it does not stop to ask then.
                let given = |id: &'static str| h_flex().id(id).h(px(28.)).w_full().gap(px(6.)).items_center().justify_center().rounded(px(8.)).bg(theme.success.opacity(0.1)).text_size(px(12.5)).text_color(theme.success).child(Icon::default().path("icons/check.svg").with_size(px(14.))).child(t("Allowed"));
                let press = |id: &'static str, icon: &'static str, label: &'static str, kind: Press| small_pill(id, icon, label, kind, false, &theme).h(px(28.)).w_full().justify_center();
                let ax = if self.outside_on {
                    given("ax-given").into_any_element()
                } else {
                    press("ax-allow", "icons/shield.svg", t("Allow"), Press::Primary).on_click(cx.listener(|this, _, window, cx| {
                        swallow_click(window, cx);
                        this.allow_accessibility(cx);
                    })).into_any_element()
                };
                let mic = match self.mic {
                    Mic::Allowed => given("mic-given").into_any_element(),
                    Mic::Unasked => press("mic-allow", "icons/microphone.svg", t("Allow"), Press::Primary)
                        .on_click(cx.listener(|this, _, window, cx| {
                            swallow_click(window, cx);
                            this.allow_microphone(cx);
                        }))
                        .into_any_element(),
                    Mic::Refused => press("mic-open", "icons/external-link.svg", t("Open Settings"), Press::Quiet)
                        .on_click(cx.listener(|this, _, window, cx| {
                            swallow_click(window, cx);
                            this.allow_microphone(cx);
                        }))
                        .into_any_element(),
                };
                let permit = |el: AnyElement| div().w(PERMIT_W).child(el).into_any_element();
                let mic_line = if self.mic == Mic::Refused { t("Turn it on in System Settings.") } else { t("To hear you.") };
                vec![
                    group(Some(t("Dictation")), None, self.shortcut_rows(cx), &theme),
                    group(
                        Some(t("App")),
                        None,
                        vec![
                            line(t("Theme"), None, appearance, &theme),
                            line(t("Launch at login"), waiting.then(|| div().child(t("Allow Kotoba in Login Items.")).into_any_element()), login, &theme),
                            line(t("Show in menu bar"), said(t("When hidden, open Kotoba again to come back here.")), shown, &theme),
                            line(t("Swap clicks"), said(t("Left click shows the menu, right click opens settings.")), swap, &theme),
                        ],
                        &theme,
                    ),
                    group(Some(t("Permissions")), None, vec![line(t("Microphone"), said(mic_line), permit(mic), &theme), line(t("Accessibility"), said(t("For the shortcut and paste.")), permit(ax), &theme)], &theme),
                ]
            }
            "updates" => {
                // As little as possible: the version, with what a check
                // found, and one button that is "Check for updates" until
                // a check finds something and "Update to x" after, in the
                // same place, with "Release notes" under the version then.
                let u = &self.update;
                let current = update::current_version();
                let update_line = if u.installing {
                    match u.progress {
                        Some((done, Some(total))) if total > 0 => tf("Downloading… {p}%", &[("p", &(done * 100 / total))]),
                        Some(_) => t("Downloading…").to_string(),
                        None => t("Installing… Kotoba restarts by itself.").to_string(),
                    }
                } else if !u.install_error.is_empty() {
                    tf("The update did not go through: {e}", &[("e", &u.install_error)])
                } else if !u.error.is_empty() {
                    t("Could not reach GitHub.").to_string()
                } else if u.available.is_none() && u.answered && !u.automatic {
                    t("Up to date.").to_string()
                } else {
                    String::new()
                };
                let can_install = u.available.is_some() && !u.installing && update::asset_name().is_some();
                let (checking, installing) = (u.checking, u.installing);
                let notes_url = u.available.as_deref().map(update::release_page);
                let press = |id: &'static str, label: SharedString, kind: Press, off: bool| small_pill(id, "icons/redo-2.svg", label, kind, off, &theme).h(px(28.)).w(UPDATE_W).justify_center();
                let button = if can_install {
                    press("update-now", tf("Update to {v}", &[("v", &u.available.as_deref().unwrap_or(""))]).into(), Press::Primary, false).on_click(cx.listener(|this, _, _, cx| this.install_update_now(cx))).into_any_element()
                } else if installing {
                    press("update-busy", t("Updating…").into(), Press::Quiet, true).into_any_element()
                } else {
                    press("update-check", if checking { t("Checking…") } else { t("Check for updates") }.into(), Press::Quiet, checking).on_click(cx.listener(|this, _, _, cx| this.check_updates_now(cx))).into_any_element()
                };
                let version_detail: SharedString = if update_line.is_empty() { format!("Kotoba {current}") } else { format!("Kotoba {current} · {update_line}") }.into();
                let version_note = v_flex().gap(px(1.)).child(div().child(version_detail)).when_some(notes_url, |d, url| {
                    d.child(
                        div()
                            .id("update-notes")
                            .self_start()
                            .cursor_pointer()
                            .text_color(link_color)
                            .text_decoration_1()
                            .text_decoration_color(link_color.opacity(0.6))
                            .hover(move |s| s.text_color(link_hover).text_decoration_color(link_hover))
                            .on_click(move |_, _, cx| cx.open_url(&url))
                            .child(t("Release notes")),
                    )
                });
                let auto_check = self.switch("update-auto", app.check_updates, Rc::new(|this, on, _, cx| this.set_check_updates(on, cx)), cx);
                vec![group(None, None, vec![line(t("Version"), Some(version_note.into_any_element()), button, &theme), line(t("Check automatically"), said(t("Once a day. Nothing is installed without you.")), auto_check, &theme)], &theme)]
            }
            // Who makes Kotoba: the app, and a sentence naming the studio
            // and its two owners, each name a link to where they are found.
            "about" => {
                let current = update::current_version();
                let link = |id: &'static str, name: &'static str, url: &'static str| {
                    div()
                        .id(id)
                        .cursor_pointer()
                        .text_color(theme.foreground)
                        .text_decoration_1()
                        .text_decoration_color(theme.foreground.opacity(0.35))
                        .hover(move |s| s.text_color(link_hover).text_decoration_color(link_hover))
                        .on_click(move |_, _, cx| cx.open_url(url))
                        .child(name)
                };
                let prose = || h_flex().flex_wrap().justify_center().text_size(px(13.5)).line_height(px(22.)).text_color(theme.muted_foreground);
                let go = |id: &'static str, icon: &'static str, label: &'static str, url: String| small_pill(id, icon, label, Press::Quiet, false, &theme).on_click(move |_, _, cx| cx.open_url(&url));
                let accent = theme.primary;
                let site = update::repo_page();
                vec![
                    // A title card, set in the middle as a film's first
                    // one is, on a plain plate of the fields' ground: who
                    // presents, the app, its version.
                    v_flex()
                        .w_full()
                        .items_center()
                        .pt(px(30.))
                        .pb(px(28.))
                        .rounded(CARD_R)
                        .border_1()
                        .border_color(theme.border)
                        .bg(card_ground(&theme))
                        .child(weave(t("{studio} presents"), vec![("studio", link("about-studio", "Alkaline Software", "https://github.com/alkaline-software").into_any_element())], prose().text_size(px(12.5))))
                        .child(div().id("about-icon").mt(px(18.)).cursor_pointer().hover(|s| s.opacity(0.82)).on_click({
                            let site = site.clone();
                            move |_, _, cx| cx.open_url(&site)
                        }).child(img("icon/app-large.png").size(px(88.)).flex_shrink_0()))
                        .child(
                            div()
                                .id("about-name")
                                .mt(px(6.))
                                .cursor_pointer()
                                .hover(|s| s.opacity(0.82))
                                .on_click({
                                    let site = site.clone();
                                    move |_, _, cx| cx.open_url(&site)
                                })
                                .text_size(px(42.))
                                .line_height(px(48.))
                                .text_color(accent)
                                .when_some(crate::fonts::wordmark_family(cx), |d, f| d.font_family(f))
                                .child("Kotoba"),
                        )
                        .child(div().mt(px(4.)).text_size(px(12.5)).text_color(theme.muted_foreground).child(tf("言葉 · Version {v}", &[("v", &current)])))
                        .into_any_element(),
                    v_flex()
                        .w_full()
                        .items_center()
                        .gap(px(16.))
                        .pt(px(8.))
                        .child(weave(
                            t("A studio by {jp} and {pingfan}."),
                            vec![("jp", link("about-jp", "Dr. John Paul Helveston", "https://www.jhelvy.com").into_any_element()), ("pingfan", link("about-pingfan", "Dr. Pingfan Hu", "https://pingfan.org").into_any_element())],
                            prose(),
                        ))
                        .child(h_flex().gap(px(8.)).child(go("about-source", "icons/github.svg", "GitHub", site.clone())).child(go("about-notes", "icons/file-text.svg", t("Release Notes"), update::release_page(current))))
                        .into_any_element(),
                ]
            }
            // Every section is one of `SECTIONS` (`show_settings`).
            _ => Vec::new(),
        };
        // The language, by the system's or by a flag and its letters, in
        // a list that drops down; a press on it is not a drag of the top.
        let (lang_icon, lang_label) = match crate::i18n::CHOICES.iter().find(|(c, _, _)| *c == app.language) {
            Some((_, flag, letters)) => (None, format!("{flag}  {letters}")),
            None => (Some("icons/globe.svg"), t("System").to_string()),
        };
        let language = page_in(
            "language-pick",
            div().w(LANGUAGE_W).on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()).child(self.settings_pick_h(PickFor::Language, "language", lang_icon, lang_label, px(28.), cx)),
        );
        let title = SECTIONS.iter().find(|(k, _, _)| *k == section).map(|(_, name, _)| t(name)).unwrap_or(t("General"));
        let notice = self.notice_line(cx);
        let rail = self.settings_rail(cx);
        h_flex()
            .size_full()
            .items_stretch()
            .child(rail)
            .child(
                v_flex()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    // The window's top is dragged by, and holds the
                    // section's name, level with the traffic lights, as
                    // the Mac's own settings do. It fades in with the page.
                    .child(Self::drag_region(h_flex().h(TOP_H).flex_shrink_0().w_full().px(PAGE_X).items_center(), cx).child(page_in(
                        match section {
                            "models" => "title-models",
                            "customize" => "title-customize",
                            "updates" => "title-updates",
                            "about" => "title-about",
                            _ => "title-general",
                        },
                        div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).child(title),
                    ))
                    // General's top right: the window's language.
                    .child(div().flex_1())
                    .when(section == "general", |d| d.child(language)))
                    .child(
                        v_flex()
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .child(
                                v_flex()
                                    .id("settings-body")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.settings_scroll)
                                    .px(PAGE_X)
                                    .pt(px(2.))
                                    .pb(px(20.))
                                    .child(page_in(
                                        match section {
                                            "models" => "page-models",
                                            "customize" => "page-customize",
                                            "updates" => "page-updates",
                                            "about" => "page-about",
                                            _ => "page-general",
                                        },
                                        v_flex().gap(px(18.)).children(notice).children(rows),
                                    )),
                            )
                            .vertical_scrollbar(&self.settings_scroll),
                    ),
            )
    }

    /// The rail of sections at the window's left, under its traffic
    /// lights: an icon and a name per section, the chosen one on a plate.
    /// General wears a dot while a permission is not given, and Updates
    /// while a newer Kotoba waits.
    fn settings_rail(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let current = self.section;
        let wanting = kotoba_core::dictate::AVAILABLE && !self.all_allowed();
        let update_waiting = self.update.available.is_some();
        Self::drag_region(
            v_flex()
                .w(RAIL_W)
                .h_full()
                .flex_shrink_0()
                .pt(TOP_H)
                .px(px(10.))
                .gap(px(2.))
                .bg(if theme.mode.is_dark() { theme.sidebar } else { theme.muted.opacity(0.5) })
                .border_r_1()
                .border_color(theme.border),
            cx,
        )
        .children(SECTIONS.iter().map(|(key, label, icon)| {
            let active = *key == current;
            let key: &'static str = key;
            let dot = (key == "general" && wanting) || (key == "updates" && update_waiting);
            h_flex()
                .id(SharedString::from(format!("settings-nav-{key}")))
                .h(px(30.))
                .px(px(10.))
                .gap(px(9.))
                .items_center()
                .rounded(px(8.))
                .cursor_pointer()
                .when(active, |d| d.bg(theme.sidebar_accent))
                .when(!active, |d| d.hover(|s| s.bg(theme.sidebar_accent.opacity(0.6))))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.section != key {
                        this.section = key;
                        this.settings_scroll.set_offset(point(px(0.), px(0.)));
                        this.mic = crate::sys::mic();
                    }
                    cx.notify();
                }))
                .child(Icon::default().path(*icon).with_size(px(15.)).text_color(if active { theme.foreground } else { theme.muted_foreground }))
                .child(div().flex_1().text_size(px(13.)).when(active, |d| d.font_weight(FontWeight::MEDIUM)).text_color(if active { theme.foreground } else { theme.sidebar_foreground }).child(t(label)))
                .when(dot, |d| d.child(div().size(px(7.)).rounded_full().bg(theme.primary)))
        }))
        .into_any_element()
    }

    /// The notice at the page's head, opening down from the title and
    /// closing back as it goes.
    fn notice_line(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let theme = cx.theme().clone();
        if let Some(n) = &self.notice {
            *self.notice_kept.borrow_mut() = n.words.clone();
        }
        let on = self.notice.is_some();
        let error = self.notice.as_ref().is_some_and(|n| n.error);
        let words = self.notice_kept.borrow().clone();
        let ink = if error { theme.danger } else { theme.muted_foreground };
        self.fold_under(
            "notice".into(),
            on,
            h_flex()
                .w_full()
                .gap(px(8.))
                .px(px(12.))
                .py(px(8.))
                .rounded(px(10.))
                .bg(if error { theme.danger.opacity(0.07) } else { theme.muted.opacity(0.6) })
                .child(Icon::default().path(if error { "icons/triangle-alert.svg" } else { "icons/info.svg" }).with_size(px(14.)).text_color(ink).flex_shrink_0())
                .child(div().flex_1().min_w_0().text_size(px(12.5)).whitespace_normal().text_color(theme.foreground).child(words))
                .into_any_element(),
            cx,
        )
    }

    /// Models: speech to text over post-processing, each with all its
    /// models, and why a download stopped.
    fn models_rows(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = cx.theme().clone();
        if !kotoba_core::dictate::AVAILABLE {
            return vec![div().text_size(px(13.5)).text_color(theme.muted_foreground).child(t("Dictation is on a Mac only for now.")).into_any_element()];
        }
        let mut rows = vec![self.dictate_lane(Lane::Speech, cx), self.dictate_lane(Lane::Clean, cx)];
        rows.extend(self.dictate_dl_error.clone().map(|e| div().px(px(14.)).text_size(px(12.)).text_color(theme.danger).child(e).into_any_element()));
        rows
    }

    /// General's first rows: the key, with More and Reset and what is
    /// said of it under them, and how long the history keeps what was
    /// written, with the way to it.
    fn shortcut_rows(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = cx.theme().clone();
        let dark = theme.mode.is_dark();
        let app = self.cfg.app.clone();
        let recording = self.dictate_key_rec;
        let field = div()
            .id("dictate-key")
            .track_focus(&self.dictate_key_focus)
            .w_full()
            .h(px(28.))
            .px(px(10.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(8.))
            .bg(theme.muted)
            .border_1()
            .border_color(if recording { theme.primary } else { gpui::transparent_black() })
            .cursor_pointer()
            .text_size(px(13.))
            .on_click(cx.listener(|this, _, window, cx| {
                this.dictate_key_rec = true;
                this.dictate_key_note = None;
                window.focus(&this.dictate_key_focus, cx);
                cx.notify();
            }))
            .child(if recording { div().text_color(theme.muted_foreground).child(t("Press the keys")).into_any_element() } else { key_combo(&dictate_key_signs(&app.dictate_key), theme.muted_foreground).into_any_element() })
            .into_any_element();
        // Modifiers alone, which no key pressed in the field can be, and
        // the F keys, are chosen from the list that drops from More.
        let more_open = self.pick.as_ref().is_some_and(|p| p.what == PickFor::DictateKey);
        let more = div()
            .relative()
            .flex_shrink_0()
            .child(
                small_pill("dictate-key-more", "icons/chevron-down.svg", t("More"), Press::Quiet, false, &theme)
                    .h(px(28.))
                    .min_w(DICTATE_PILL_W)
                    .justify_center()
                    .when(more_open, |d| d.bg(theme.muted_foreground.opacity(0.16)))
                    .managed_tooltip(|window, cx| tip(t("Modifier keys and F keys"), window, cx))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        swallow_click(window, cx);
                        this.dictate_key_rec = false;
                        this.dictate_key_note = None;
                        this.pick_toggle(PickFor::DictateKey, cx);
                    })),
            )
            .child(self.pick_mark(PickFor::DictateKey, cx));
        // Under the row: why a key was not taken, coming and going in
        // motion from what it last said.
        let note = self.dictate_key_note.clone();
        if let Some(n) = &note {
            self.dictate_kept.borrow_mut().0 = n.clone();
        }
        let said = self.dictate_kept.borrow().0.clone();
        let note = self.fold_under("dictate-key-note".into(), note.is_some(), h_flex().w_full().pb(px(10.)).justify_end().text_size(px(12.)).text_color(theme.muted_foreground).child(key_text(&said)).into_any_element(), cx);
        // With Fn chosen and the Mac giving it a job of its own, a tap
        // would do both; with F1 to F12 and the Mac's keys doing what is
        // printed on them, the key would not reach Kotoba at all. How to
        // change it, in Keyboard settings. It goes once the window comes
        // back with that done.
        let key_job = if crate::kotoba::key_has_use(&app.dictate_key) { self.key_use.flatten() } else { None };
        if let Some(job) = key_job {
            let mut kept = self.dictate_kept.borrow_mut();
            kept.1 = job;
            kept.2 = dictate_key_signs(&app.dictate_key);
        }
        let job = self.dictate_kept.borrow().1;
        let words = if job == "printed" {
            let key = self.dictate_kept.borrow().2.clone();
            format!("{} {}", tf("On Apple keyboards, {key} does what is printed on it unless Fn 🌐 is held.", &[("key", &key)]), t("Or turn on “Use F1, F2, etc. keys as standard function keys” in Keyboard Shortcuts."))
        } else {
            let also = match job {
                "input" => t("Fn 🌐 would also change the input source."),
                "emoji" => t("Fn 🌐 would also show emoji and symbols."),
                "dictation" => t("Fn 🌐 would also start Apple's dictation."),
                _ => t("Fn 🌐 also has a use in macOS."),
            };
            format!("{also} {}", t("Set it to Do Nothing."))
        };
        let warn = theme.warning;
        let guide = h_flex()
            .w_full()
            .gap(px(10.))
            .pl(px(12.))
            .pr(px(8.))
            .py(px(8.))
            .rounded(px(10.))
            .bg(warn.opacity(if dark { 0.10 } else { 0.07 }))
            .border_1()
            .border_color(warn.opacity(0.35))
            .child(Icon::default().path("icons/triangle-alert.svg").with_size(px(15.)).text_color(warn).flex_shrink_0())
            // The button goes under the words where the line is too
            // narrow for both, never squeezing them a word a line.
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(key_text_wrap(&words).flex_grow(1.).flex_shrink(1.).flex_basis(px(150.)).min_w_0().text_size(px(12.5)))
                    .child(small_pill("dictate-fn-settings", "icons/external-link.svg", "Keyboard Settings", Press::Quiet, false, &theme).flex_shrink_0().on_click(|_, _, cx| cx.open_url(crate::sys::KEYBOARD_SETTINGS_URL))),
            )
            .into_any_element();
        let guide = self.fold_under("dictate-fn-guide".into(), key_job.is_some(), div().pb(px(10.)).child(guide).into_any_element(), cx);
        let theme_ref = &theme;
        vec![
            // The key's field, More and Reset at the row's right, and
            // under the row, across the card, what is said of the key.
            v_flex()
                .child(line(
                    t("Shortcut"),
                    Some(div().child(t("Starts and stops dictation.")).into_any_element()),
                    h_flex()
                        .w(CONTROL_W)
                        .items_center()
                        .gap(px(8.))
                        .child(div().flex_1().min_w_0().child(field))
                        .child(more)
                        .child(
                            small_pill("dictate-key-reset", "icons/rotate.svg", t("Reset"), Press::Quiet, app.dictate_key == kotoba_core::config::AppConfig::default().dictate_key, &theme)
                                .h(px(28.))
                                .min_w(DICTATE_PILL_W)
                                .justify_center()
                                .flex_shrink_0()
                                .on_click(cx.listener(|this, _, _, cx| this.reset_dictate_key(cx))),
                        )
                        .into_any_element(),
                    theme_ref,
                ))
                .children(note.map(|n| div().px(px(14.)).child(n)))
                .children(guide.map(|g| div().px(px(14.)).child(g)))
                .into_any_element(),
            // How long what dictation writes is kept, and the way to it.
            line(
                t("History"),
                Some(div().child(t("Kept on this Mac only.")).into_any_element()),
                h_flex()
                    .w(CONTROL_W)
                    .items_center()
                    .gap(px(8.))
                    .child(div().w(FIELD_W).child(self.settings_pick_h(PickFor::DictateKeep, "dictate-keep", Some("icons/history.svg"), t(kotoba_core::dictate::keep(&app.dictate_keep).1).to_string(), px(28.), cx)))
                    .child(
                        // As wide as More and Reset above it, so the two
                        // rows end level.
                        small_pill("dictate-history", "icons/list-bullets.svg", t("Show History"), Press::Quiet, false, &theme).h(px(28.)).flex_1().justify_center().on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            swallow_click(window, cx);
                            this.open_dictated(cx);
                        })),
                    )
                    .into_any_element(),
                theme_ref,
            ),
        ]
    }

    /// One lane of Models, a group: its name (post-processing's with its
    /// switch at the right), and every model on its card, the most
    /// capable first, in an order that stays put as one is chosen.
    fn dictate_lane(&self, lane: Lane, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let in_use = self.lane_chosen(lane);
        let title = match lane {
            Lane::Speech => t("Speech to Text"),
            Lane::Clean => t("Post-processing"),
        };
        let models = lane.models();
        // Post-processing off: its name and its models fade to grey, over a
        // moment; the switch stays as it is.
        let live = lane == Lane::Speech || (self.cfg.app.clean_on && in_use.is_some());
        let fade = self.presence(&format!("lane-live-{}", lane.key()), live, cx);
        let dim = move |d: Div| -> AnyElement {
            match fade {
                Some((coming, n)) => d.with_animation(ElementId::Name(format!("lane-fade-{n}").into()), Animation::new(FOLD_ANIM).with_easing(ease_out_quint()), move |d, t| d.opacity(if coming { 0.5 + 0.5 * t } else { 1. - 0.5 * t })).into_any_element(),
                None => d.opacity(if live { 1. } else { 0.5 }).into_any_element(),
            }
        };
        let switch = (lane == Lane::Clean).then(|| {
            let on = self.cfg.app.clean_on && in_use.is_some();
            self.switch("clean-on", on, Rc::new(|this, on, _, cx| this.set_clean_on(on, cx)), cx)
        });
        let last = models.len().saturating_sub(1);
        let rows: Vec<AnyElement> = models.iter().enumerate().map(|(i, m)| self.model_row(lane, m, in_use, (i == 0, i == last), cx)).collect();
        v_flex().gap(px(6.)).child(group_head(Some(dim(div().child(title))), switch, &theme)).child(dim(card(rows, &theme))).into_any_element()
    }

    /// One model on its row of the lane's card: its maker's mark, its
    /// name with its size, what sets it apart and its languages, and at
    /// the right what is done with it: Download (a line filling as it
    /// comes, with Stop), Use, or "In use", with Remove once it is here.
    /// The one in use is washed with the accent, rounded where it meets
    /// the card's corners (`ends`, first and last), and a click on a row
    /// that is here takes it up.
    fn model_row(&self, lane: Lane, m: &'static dictate::Model, in_use: Option<&'static dictate::Model>, ends: (bool, bool), cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let well = theme.muted;
        let here = dictate::installed(m);
        let using = in_use.is_some_and(|u| u.key == m.key);
        // A post-processing model chosen while it is off is not in use.
        let resting = lane == Lane::Clean && !self.cfg.app.clean_on;
        let coming = self.dictate_dl.iter().find(|(k, _, _)| *k == m.key).map(|(_, got, stop)| (got.load(std::sync::atomic::Ordering::Relaxed), std::sync::Arc::clone(stop)));
        let mark = if m.maker == "Qwen" { "icons/makers/qwen.svg" } else { "icons/makers/openai.svg" };
        let size = if m.bytes >= 1_000_000_000 { format!("{:.1} GB", m.bytes as f64 / 1e9) } else { format!("{} MB", m.bytes / 1_000_000) };
        let slot = |el: AnyElement| div().w(MODEL_SLOT_W).flex_shrink_0().flex().justify_center().child(el).into_any_element();
        // Post-processing turned off is grey throughout: no accent, no red.
        let pill = |id: String, icon: &'static str, word: &'static str, kind: Press| small_pill(SharedString::from(id), icon, word, if resting { Press::Quiet } else { kind }, false, &theme).h(px(28.)).w(MODEL_SLOT_W).px(px(8.)).justify_center();
        let (first, second): (Option<AnyElement>, AnyElement) = if let Some((got, stop)) = coming {
            let share = (got as f32 / m.bytes.max(1) as f32).clamp(0.0, 1.0);
            let bar = v_flex()
                .w_full()
                .gap(px(5.))
                .child(div().h(px(3.)).w_full().rounded_full().bg(well).child(div().h_full().rounded_full().bg(theme.primary).w(relative(share))))
                .child(div().text_size(px(11.5)).text_color(theme.muted_foreground).child(format!("{}%", (share * 100.) as u32)))
                .into_any_element();
            let stop = pill(format!("dictate-stop-{}", m.key), "icons/close.svg", t("Stop"), Press::Quiet).on_click(move |_, window, cx| {
                stop.store(true, std::sync::atomic::Ordering::Relaxed);
                swallow_click(window, cx);
            });
            (Some(bar), stop.into_any_element())
        } else if here {
            let state = if using {
                let ink = if resting { theme.muted_foreground } else { theme.primary };
                h_flex().gap(px(5.)).items_center().text_size(px(12.)).text_color(ink).child(Icon::default().path("icons/check.svg").with_size(px(13.))).child(if resting { t("Chosen") } else { t("In use") }).into_any_element()
            } else {
                pill(format!("dictate-use-{}", m.key), "icons/check.svg", t("Use"), Press::Quiet)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        swallow_click(window, cx);
                        this.lane_use(lane, m, cx);
                    }))
                    .into_any_element()
            };
            let remove = pill(format!("dictate-remove-{}", m.key), "icons/trash.svg", t("Remove"), Press::Danger).on_click(cx.listener(move |this, _, window, cx| {
                swallow_click(window, cx);
                let gone = match lane {
                    Lane::Speech => dictate::remove(m),
                    Lane::Clean => kotoba_core::clean::remove(m),
                };
                if let Err(e) = gone {
                    this.say_error(tf("Not removed: {e}.", &[("e", &e)]), cx);
                }
                cx.notify();
            }));
            (Some(state), remove.into_any_element())
        } else {
            let get = pill(format!("dictate-get-{}", m.key), "icons/download.svg", t("Download"), Press::Quiet).on_click(cx.listener(move |this, _, window, cx| {
                swallow_click(window, cx);
                this.download_model(m, cx);
                cx.notify();
            }));
            (None, get.into_any_element())
        };
        let side: AnyElement = h_flex().gap(px(8.)).items_center().child(slot(first.unwrap_or_else(|| div().into_any_element()))).child(slot(second)).into_any_element();
        let hover = theme.muted_foreground.opacity(0.06);
        let lit = using && !resting;
        let inner_r = CARD_R - px(1.);
        h_flex()
            .id(SharedString::from(format!("dictate-row-{}", m.key)))
            .w_full()
            .px(px(14.))
            .py(px(8.))
            .gap(px(12.))
            .items_center()
            .when(ends.0, |d| d.rounded_t(inner_r))
            .when(ends.1, |d| d.rounded_b(inner_r))
            .when(lit, |d| d.bg(theme.primary.opacity(0.07)))
            // A model that is here, and not the one in use, is taken up by
            // a click anywhere on its row.
            .when(here && !using, |d| {
                d.cursor_pointer().hover(move |s| s.bg(hover)).on_click(cx.listener(move |this, _, window, cx| {
                    swallow_click(window, cx);
                    this.lane_use(lane, m, cx);
                }))
            })
            .child(div().size(px(30.)).flex_shrink_0().rounded(px(8.)).bg(well).flex().items_center().justify_center().child(Icon::default().path(mark).with_size(px(16.)).text_color(theme.foreground)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(1.))
                    // The name and its size, then what sets it apart and
                    // the languages it knows; the maker is its mark. In a
                    // narrow window the name gives way, cut at its end.
                    .child(h_flex().min_w_0().gap(px(8.)).items_baseline().child(div().min_w_0().truncate().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(m.name)).child(div().flex_shrink_0().text_size(px(11.5)).text_color(theme.muted_foreground).child(size)))
                    .child(div().min_w_0().truncate().text_size(px(11.5)).text_color(theme.muted_foreground).child(format!("{} · {}", t(m.about), t(m.languages)))),
            )
            .child(div().flex_shrink_0().child(side))
            .into_any_element()
    }

    /// Dictation's history: a sheet over the settings, the newest first
    /// under the day each was said, every one with its time, its length
    /// and a Copy that shows on the pointer. It rises in as it fades in,
    /// and fades out where it was (`gone`), taking no click then.
    fn render_dictated(&self, all: Rc<Vec<dictate::Said>>, serial: u32, gone: bool, cx: &mut Context<Self>) -> AnyElement {
        let wall = self.kept_scroll(format!("history-wall-{serial}").into(), Inner::Wall).0;
        let theme = cx.theme().clone();
        let dark = theme.mode.is_dark();
        let keep = dictate::keep(&self.cfg.app.dictate_keep);
        let (scroll, _) = self.kept_scroll(format!("history-{serial}").into(), Inner::Over);
        let well = if dark { theme.sidebar } else { theme.muted };
        let hover_bg = if dark { theme.foreground.opacity(0.05) } else { theme.muted.opacity(0.7) };
        let said_n = all.len();
        // Under the title, how many and for how long; with none kept the
        // middle of the sheet says it.
        let kept = match keep.0 {
            "day" => t("kept 1 day on this Mac"),
            "week" => t("kept 1 week on this Mac"),
            "month" => t("kept 1 month on this Mac"),
            "year" => t("kept 1 year on this Mac"),
            _ => t("kept on this Mac"),
        };
        let sub = (said_n > 0).then(|| format!("{} · {kept}", count(said_n, "{n} dictation", "{n} dictations")));
        let mut body = v_flex().px(px(12.)).pt(px(6.)).pb(px(14.));
        let mut day = String::new();
        for (ix, said) in all.iter().enumerate() {
            let at = chrono::DateTime::from_timestamp(said.at, 0).map(|t| t.with_timezone(&chrono::Local));
            let this_day = at.map(|t| said_day(t.date_naive())).unwrap_or_default();
            if this_day != day {
                body = body.child(div().flex_shrink_0().px(px(10.)).pt(px(if ix == 0 { 6. } else { 14. })).pb(px(4.)).text_size(px(11.5)).font_weight(FontWeight::MEDIUM).text_color(theme.muted_foreground).child(this_day.clone()));
                day = this_day;
            }
            let secs = said.secs.round() as u64;
            let mut meta = vec![at.map(|t| t.format("%H:%M").to_string()).unwrap_or_default(), format!("{}:{:02}", secs / 60, secs % 60)];
            meta.extend(dictate::model(&said.model).map(|m| m.name.to_string()));
            let group: SharedString = format!("said-row-{ix}").into();
            // A Copy beside each text it copies: the words that went in,
            // and what was heard when post-processing changed them. They
            // show on the pointer, and the one pressed keeps its tick a
            // moment.
            let copy = |key: SharedString, text: String, said: &'static str, cx: &mut Context<Self>| {
                let done = self.copied.as_ref() == Some(&key);
                div()
                    .id(key.clone())
                    .size(px(26.))
                    .flex_shrink_0()
                    .rounded(px(7.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme.popover)
                    .border_1()
                    .border_color(theme.border)
                    .cursor_pointer()
                    .opacity(if done { 1. } else { 0. })
                    .group_hover(group.clone(), |s| s.opacity(1.))
                    .hover(move |s| s.bg(well))
                    .managed_tooltip(move |window, cx| tip(if done { t("Copied") } else { said }, window, cx))
                    .when(!gone, |d| {
                        d.on_click(cx.listener(move |this, _, window, cx| {
                            swallow_click(window, cx);
                            this.copy_text(key.clone(), text.clone(), cx);
                        }))
                    })
                    .child(Icon::default().path(if done { "icons/check.svg" } else { "icons/copy.svg" }).with_size(px(13.)).text_color(if done { theme.primary } else { theme.muted_foreground }))
            };
            let line = |text: AnyElement, button: AnyElement| h_flex().items_start().gap(px(10.)).child(div().flex_1().min_w_0().child(text)).child(button);
            let heard = said.raw.clone();
            let words = line(
                div().text_size(px(13.5)).line_height(px(20.)).whitespace_normal().child(said.words.clone()).into_any_element(),
                copy(format!("said-{}-{ix}", said.at).into(), said.words.clone(), if heard.is_some() { t("Copy post-processed") } else { t("Copy") }, cx).into_any_element(),
            );
            let heard = heard.map(|raw| line(div().text_size(px(12.5)).line_height(px(18.)).whitespace_normal().text_color(theme.muted_foreground).child(raw.clone()).into_any_element(), copy(format!("said-raw-{}-{ix}", said.at).into(), raw, t("Copy as heard"), cx).into_any_element()));
            body = body.child(
                v_flex()
                    .id(("said", ix))
                    .group(group.clone())
                    .flex_shrink_0()
                    .pl(px(10.))
                    .pr(px(8.))
                    .py(px(8.))
                    .gap(px(4.))
                    .rounded(px(10.))
                    .hover(move |s| s.bg(hover_bg))
                    .child(words)
                    // As it was heard, when post-processing changed it.
                    .children(heard)
                    .child(div().text_size(px(11.5)).text_color(theme.muted_foreground.opacity(0.85)).child(meta.join(" · "))),
            );
        }
        // Nothing to show: a mark on a plate and a line, in the middle.
        let empty = (said_n == 0).then(|| {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap(px(10.))
                .pb(px(24.))
                .child(div().size(px(44.)).rounded_full().bg(well).flex().items_center().justify_center().child(Icon::default().path("icons/history.svg").with_size(px(20.)).text_color(theme.muted_foreground)))
                .child(div().text_size(px(13.5)).text_color(theme.muted_foreground).child(if keep.2 == Some(0) { t("Dictations are not saved.") } else { t("Nothing dictated yet.") }))
        });
        let body = match empty {
            Some(empty) => empty.into_any_element(),
            None => v_flex().relative().flex_1().min_h_0().child(v_flex().id(("history-rows", serial as usize)).flex_1().min_h_0().overflow_y_scroll().track_scroll(&scroll).child(body)).vertical_scrollbar(&scroll).into_any_element(),
        };
        // The folder the history is kept in, opened in Finder.
        let folder = div()
            .id(("history-folder", serial as usize))
            .size(px(20.))
            .flex_shrink_0()
            .rounded(px(5.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .text_color(theme.muted_foreground)
            .hover(|s| s.bg(theme.muted).text_color(theme.foreground))
            .managed_tooltip(|window, cx| tip(if cfg!(target_os = "macos") { t("Open in Finder") } else { t("Open folder") }, window, cx))
            .when(!gone, |d| {
                d.on_click(|_, window, cx| {
                    swallow_click(window, cx);
                    if let Some(dir) = dictate::history_file().parent() {
                        let _ = std::fs::create_dir_all(dir);
                        crate::sys::open_path(dir);
                    }
                })
            })
            .child(Icon::default().path("icons/external-link.svg").with_size(px(12.)))
            .into_any_element();
        self.over_settings("history", serial, gone, t("Dictation History"), sub, Some(folder), body, wall, Rc::new(|this, _, cx| this.close_dictated(cx)), cx)
    }

    /// A sheet over the settings: smaller than the window both ways, so
    /// it reads as lying on it, with its title, a line under it, and a ×.
    /// It rises in as it fades in, and fades out where it was (`gone`),
    /// taking no click then. A click off it, the ×, and Escape in any
    /// field on it put it away (`close`). `wall` is the sheet's own,
    /// named before anything on it, so the wheel moves nothing under it;
    /// `aside` stands at the end of the line under the title.
    #[allow(clippy::too_many_arguments)]
    fn over_settings(&self, key: &'static str, serial: u32, gone: bool, title: &'static str, sub: Option<String>, aside: Option<AnyElement>, body: AnyElement, wall: ScrollHandle, close: Rc<dyn Fn(&mut Self, &mut Window, &mut Context<Self>)>, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let n = serial as usize;
        let shut = div()
            .id(SharedString::from(format!("{key}-close")))
            .size(px(28.))
            .flex_shrink_0()
            .rounded(px(8.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .text_color(theme.muted_foreground)
            .hover(|s| s.bg(theme.muted))
            .managed_tooltip(|window, cx| tip(t("Close (esc)"), window, cx))
            .when(!gone, |d| {
                let close = Rc::clone(&close);
                d.on_click(cx.listener(move |this, _, window, cx| {
                    swallow_click(window, cx);
                    close(this, window, cx);
                }))
            })
            .child(Icon::new(IconName::Close).with_size(px(15.)));
        let sheet = v_flex()
            .id(SharedString::from(format!("{key}-sheet-{n}")))
            .occlude()
            .on_click(|_, window, cx| swallow_click(window, cx))
            .w(px(540.))
            .h(crate::kotoba::SETTINGS_H - px(72.))
            .max_w(relative(0.86))
            .max_h(relative(0.86))
            .rounded(px(18.))
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .shadow(float_shadow(&theme))
            .overflow_hidden()
            .when(!gone, |d| {
                let close = Rc::clone(&close);
                d.capture_action(cx.listener(move |this, _: &gpui_component::input::Escape, window, cx| {
                    close(this, window, cx);
                    cx.stop_propagation();
                }))
            })
            .child(
                h_flex()
                    .flex_shrink_0()
                    .pl(px(22.))
                    .pr(px(14.))
                    .pt(px(16.))
                    .pb(px(14.))
                    .items_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(v_flex().flex_1().min_w_0().gap(px(2.)).child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).child(title)).children(sub.map(|sub| h_flex().min_w_0().gap(px(4.)).items_center().child(div().min_w_0().truncate().text_size(px(12.)).text_color(theme.muted_foreground).child(sub)).children(aside))))
                    .child(shut),
            )
            .child(body);
        let scrim = div().id(SharedString::from(format!("{key}-scrim-{n}"))).track_scroll(&wall).absolute().inset_0().occlude().bg(theme.overlay).flex().items_center().justify_center();
        if gone {
            return scrim.child(sheet).with_animation(ElementId::Name(format!("{key}-out-{n}").into()), Animation::new(MENU_OUT), |d, t| d.opacity(1. - t)).into_any_element();
        }
        scrim
            .on_click(cx.listener(move |this, _, window, cx| close(this, window, cx)))
            .child(sheet.with_animation(ElementId::Name(format!("{key}-rise-{n}").into()), Animation::new(MENU_IN * 2).with_easing(ease_out_quint()), |d, t| d.mt(px(14.) * (1. - t))))
            .with_animation(ElementId::Name(format!("{key}-in-{n}").into()), Animation::new(MENU_IN * 2).with_easing(ease_out_quint()), |d, t| d.opacity(t))
            .into_any_element()
    }

    /// Customize: what the person tells dictation. The preferences are a
    /// field of one size, kept as typed, which scrolls inside itself once
    /// they are longer than it. Terms and the words to avoid are a row
    /// each under it, with how many there are and Edit, which opens the
    /// list on a sheet.
    fn customize_rows(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = cx.theme().clone();
        let Some(prompt) = self.clean_prompt.clone() else { return Vec::new() };
        let focus = prompt.read(cx).focus_handle(cx);
        // A wall over the field: a wheel on it scrolls the field, never the
        // page under it.
        let (wall, _) = self.kept_scroll("preferences-wall".into(), Inner::Wall);
        let caption = if self.cfg.app.clean_on { t("Post-processing follows these over its own rules.") } else { t("For post-processing, which is off, in Models.") };
        let preferences = v_flex()
            .gap(px(6.))
            .child(group_head(Some(div().child(t("Preferences")).into_any_element()), Some(div().text_size(px(11.5)).text_color(theme.muted_foreground).child(caption).into_any_element()), &theme))
            .child(
                div()
                    .id("clean-prompt")
                    .track_focus(&focus)
                    .track_scroll(&wall)
                    .w_full()
                    .h(PREFERENCES_H.min(self.viewport_h - PREFERENCES_ROOM).max(PREFERENCES_LEAST))
                    .rounded(CARD_R)
                    .border_1()
                    .border_color(theme.border)
                    .bg(card_ground(&theme))
                    .text_size(px(13.5))
                    .cursor_text()
                    .px(px(14.))
                    .py(px(10.))
                    .child(Textarea::new(&prompt).appearance(false).bordered(false).h(relative(1.))),
            )
            .into_any_element();
        // Each list: how many words it has, and Edit, which opens it on a
        // sheet.
        let list_row = |list: WordList, title: &'static str, detail: &'static str, n: usize, cx: &mut Context<Self>| {
            let edit = small_pill(SharedString::from(format!("{}-open", list.key())), "icons/pencil.svg", t("Edit"), Press::Quiet, false, &theme).h(px(28.)).min_w(DICTATE_PILL_W).justify_center().on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                swallow_click(window, cx);
                this.open_words(list, window, cx);
            }));
            let control = h_flex().gap(px(12.)).items_center().child(div().text_size(px(12.)).text_color(theme.muted_foreground).child(if n == 0 { t("None yet").to_string() } else { count(n, "{n} word", "{n} words") })).child(edit).into_any_element();
            line(title, Some(div().child(detail).into_any_element()), control, &theme)
        };
        let (terms, avoided) = (self.cfg.app.dictate_terms.len(), self.cfg.app.dictate_avoid.len());
        let words = group(Some(t("Words")), None, vec![list_row(WordList::Terms, t("Terms"), t("Written exactly as you save them."), terms, cx), list_row(WordList::Avoid, t("Avoid"), t("Never written."), avoided, cx)], &theme);
        vec![preferences, words]
    }

    /// A list of the person's words, on a sheet over the settings: its
    /// field with Add at the top, several words at once, and the words
    /// under it as chips, each with its own ×. One comes in, and goes,
    /// opening and closing its room.
    fn render_words(&self, list: WordList, serial: u32, gone: bool, cx: &mut Context<Self>) -> AnyElement {
        let wall = self.kept_scroll(format!("{}-wall-{serial}", list.key()).into(), Inner::Wall).0;
        let theme = cx.theme().clone();
        let dark = theme.mode.is_dark();
        let well = if dark { theme.sidebar } else { theme.muted };
        let (words, input) = self.words_of(list);
        let Some(input) = input else { return div().into_any_element() };
        let focus = input.read(cx).focus_handle(cx);
        let blank = input.read(cx).value().trim().is_empty();
        let key = list.key();
        let add = h_flex()
            .flex_shrink_0()
            .px(px(16.))
            .pt(px(16.))
            .pb(px(12.))
            .gap(px(8.))
            .items_center()
            .child(div().id(SharedString::from(format!("{key}-field"))).track_focus(&focus).flex_1().min_w_0().h(px(34.)).px(px(10.)).flex().items_center().rounded(px(10.)).bg(well).text_size(px(13.5)).cursor_text().child(Input::new(&input).appearance(false).bordered(false)))
            .child(choice_pill(SharedString::from(format!("{key}-add")), "icons/plus.svg", t("Add"), Press::Primary, blank, &theme).h(px(34.)).managed_tooltip(|window, cx| tip(t("Add (↩)"), window, cx)).when(!blank && !gone, |d| {
                d.on_click(cx.listener(move |this, _, window, cx| {
                    swallow_click(window, cx);
                    this.add_words(list, window, cx);
                }))
            }));
        // The words as they are, with those on their way out where they
        // stood.
        let mut all: Vec<(String, Option<usize>, u32, bool)> = words.iter().enumerate().map(|(ix, t)| (t.clone(), Some(ix), 0, false)).collect();
        for (l, ix, t, at, n) in &self.words_leaving {
            if *l == list && at.elapsed() < FOLD_ANIM {
                all.insert((*ix).min(all.len()), (t.clone(), None, *n, true));
            }
        }
        let chips: Vec<AnyElement> = all
            .into_iter()
            .map(|(word, ix, n, leaving)| {
                let fresh = self.words_new.iter().find(|(l, t, _)| *l == list && !leaving && *t == word).map(|(_, _, n)| *n);
                let chip = h_flex()
                    .flex_shrink_0()
                    .h(px(30.))
                    .pl(px(11.))
                    .pr(px(4.))
                    .gap(px(4.))
                    .items_center()
                    .rounded(px(8.))
                    .bg(well)
                    .child(div().max_w(px(220.)).truncate().text_size(px(13.5)).child(word.clone()))
                    .child(
                        div()
                            .id(SharedString::from(format!("{key}-x-{word}")))
                            .size(px(22.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.muted_foreground)
                            .hover(|s| s.bg(theme.foreground.opacity(0.08)).text_color(theme.foreground))
                            .managed_tooltip(|window, cx| tip(t("Remove"), window, cx))
                            .when_some(ix.filter(|_| !gone), |d, ix| {
                                d.on_click(cx.listener(move |this, _, window, cx| {
                                    swallow_click(window, cx);
                                    this.remove_word(list, ix, cx);
                                }))
                            })
                            .child(Icon::new(IconName::Close).with_size(px(12.))),
                    );
                // Its room opens as it comes and closes as it goes: the
                // chip is uncovered, and covered, from its end.
                let room = h_flex().flex_shrink_0().overflow_hidden().child(chip);
                if leaving {
                    room.with_animation(ElementId::Name(format!("{key}-out-{n}").into()), Animation::new(FOLD_ANIM).with_easing(ease_out_quint()), |d, t| d.max_w(px(260.) * (1. - t)).mr(px(8.) * (1. - t)).opacity(1. - t)).into_any_element()
                } else if let Some(n) = fresh {
                    room.with_animation(ElementId::Name(format!("{key}-in-{n}").into()), Animation::new(FOLD_ANIM).with_easing(ease_out_quint()), |d, t| d.max_w(px(260.) * t).mr(px(8.) * t).opacity(t)).into_any_element()
                } else {
                    room.mr(px(8.)).into_any_element()
                }
            })
            .collect();
        let (scroll, _) = self.kept_scroll(format!("{}-{serial}", list.key()).into(), Inner::Over);
        let (empty_icon, empty_words, title, sub) = match list {
            WordList::Terms => ("icons/case-sensitive.svg", t("No terms yet."), t("Terms"), t("Written as saved, capitals and all.")),
            WordList::Avoid => ("icons/ban.svg", t("No words to avoid yet."), t("Avoid"), t("Never written: left out of what was heard.")),
        };
        let body = if chips.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap(px(10.))
                .pb(px(24.))
                .child(div().size(px(44.)).rounded_full().bg(well).flex().items_center().justify_center().child(Icon::default().path(empty_icon).with_size(px(20.)).text_color(theme.muted_foreground)))
                .child(div().text_size(px(13.5)).text_color(theme.muted_foreground).child(empty_words))
                .into_any_element()
        } else {
            v_flex().relative().flex_1().min_h_0().child(v_flex().id(SharedString::from(format!("{key}-list-{serial}"))).flex_1().min_h_0().overflow_y_scroll().track_scroll(&scroll).child(h_flex().flex_wrap().items_start().px(px(16.)).pb(px(16.)).gap_y(px(8.)).children(chips))).vertical_scrollbar(&scroll).into_any_element()
        };
        // At the sheet's foot: the whole list copied as one line, and the
        // whole list deleted, which asks first, in the same place.
        let n = words.len();
        let confirming = self.words_confirm == Some(list);
        let copy_key: SharedString = format!("{key}-copy-all").into();
        let copied = self.copied.as_ref() == Some(&copy_key);
        let all_words = words.join(", ");
        let ask = match list {
            WordList::Terms => count(n, "Delete {n} term?", "Delete all {n} terms?"),
            WordList::Avoid => count(n, "Delete {n} word?", "Delete all {n} words?"),
        };
        let foot_pill = |id: &'static str, icon: &'static str, label: &'static str, kind: Press, off: bool| small_pill(SharedString::from(format!("{key}-{id}")), icon, label, kind, off, &theme).h(px(28.)).px(px(12.));
        let foot = if confirming {
            h_flex()
                .flex_1()
                .gap(px(8.))
                .items_center()
                .child(div().flex_1().min_w_0().truncate().text_size(px(13.)).child(ask))
                .child(foot_pill("delete-cancel", "icons/close.svg", t("Cancel"), Press::Quiet, false).when(!gone, |d| {
                    d.on_click(cx.listener(|this, _, window, cx| {
                        swallow_click(window, cx);
                        this.words_confirm = None;
                        cx.notify();
                    }))
                }))
                .child(foot_pill("delete-yes", "icons/trash.svg", t("Delete"), Press::Danger, false).when(!gone, |d| {
                    d.on_click(cx.listener(move |this, _, window, cx| {
                        swallow_click(window, cx);
                        this.delete_words(list, cx);
                    }))
                }))
                .with_animation(ElementId::Name(format!("{key}-ask-{serial}").into()), Animation::new(MENU_IN).with_easing(ease_out_quint()), |d, t| d.opacity(t))
                .into_any_element()
        } else {
            h_flex()
                .flex_1()
                .gap(px(8.))
                .items_center()
                .justify_end()
                .child(foot_pill("copy-all", if copied { "icons/check.svg" } else { "icons/copy.svg" }, if copied { t("Copied") } else { t("Copy All") }, Press::Quiet, n == 0).when(n > 0 && !gone, |d| {
                    d.on_click(cx.listener(move |this, _, window, cx| {
                        swallow_click(window, cx);
                        this.copy_text(copy_key.clone(), all_words.clone(), cx);
                    }))
                }))
                .child(foot_pill("delete-all", "icons/trash.svg", t("Delete All"), Press::Danger, n == 0).when(n > 0 && !gone, |d| {
                    d.on_click(cx.listener(move |this, _, window, cx| {
                        swallow_click(window, cx);
                        this.words_confirm = Some(list);
                        cx.notify();
                    }))
                }))
                .into_any_element()
        };
        let foot = h_flex().flex_shrink_0().h(px(52.)).px(px(16.)).items_center().border_t_1().border_color(theme.border).child(foot);
        let body = v_flex().flex_1().min_h_0().child(add).child(body).child(foot).into_any_element();
        self.over_settings(key, serial, gone, title, Some(sub.to_string()), None, body, wall, Rc::new(|this, window, cx| this.close_words(window, cx)), cx)
    }
}

/// A group of the settings: its name over it, with what stands at the
/// name's right (`aside`), and its rows on one card.
fn group(title: Option<&'static str>, aside: Option<AnyElement>, rows: Vec<AnyElement>, theme: &gpui_component::Theme) -> AnyElement {
    v_flex().gap(px(6.)).when(title.is_some() || aside.is_some(), |d| d.child(group_head(title.map(|t| div().child(t).into_any_element()), aside, theme))).child(card(rows, theme)).into_any_element()
}

/// A group's name, small and quiet, and what stands at its right, each
/// in line with the rows' names and controls under it.
fn group_head(title: Option<AnyElement>, aside: Option<AnyElement>, theme: &gpui_component::Theme) -> Div {
    h_flex()
        .min_h(px(20.))
        .px(px(14.))
        .gap(px(12.))
        .items_center()
        .justify_between()
        .child(div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).children(title))
        .children(aside)
}

/// The ground of a group: a card a shade off the page, with a hairline
/// between its rows. A row that paints its own ground rounds it where it
/// meets the card's corners (`CARD_R`): a clip in gpui is a rectangle.
fn card(rows: Vec<AnyElement>, theme: &gpui_component::Theme) -> Div {
    v_flex()
        .w_full()
        .rounded(CARD_R)
        .border_1()
        .border_color(theme.border)
        .bg(card_ground(theme))
        .children(rows.into_iter().enumerate().map(|(i, row)| div().when(i > 0, |d| d.border_t_1().border_color(theme.border)).child(row)))
}

const CARD_R: Pixels = px(12.);

fn card_ground(theme: &gpui_component::Theme) -> Hsla {
    if theme.mode.is_dark() {
        theme.sidebar
    } else {
        theme.muted.opacity(0.4)
    }
}

/// A row of a card: its name, and under it a line where the name does
/// not say it all; its control at the right.
fn line(title: &'static str, detail: Option<AnyElement>, control: AnyElement, theme: &gpui_component::Theme) -> AnyElement {
    h_flex()
        .min_h(px(46.))
        .px(px(14.))
        .py(px(8.))
        .gap(px(16.))
        .items_center()
        .child(v_flex().flex_1().min_w_0().gap(px(1.)).child(div().text_size(px(13.)).child(title)).children(detail.map(|d| div().text_size(px(11.5)).line_height(px(15.)).text_color(theme.muted_foreground).child(d))))
        .child(div().flex_shrink_0().child(control))
        .into_any_element()
}

/// A sentence with things that are not words in it (links), in the
/// language in use: `{name}` in the translated sentence is where each
/// goes, so each language puts them where its order wants them.
fn weave(sentence: &str, mut things: Vec<(&str, AnyElement)>, row: Div) -> Div {
    let mut row = row;
    let mut rest = sentence;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|c| open + c) else { break };
        if open > 0 {
            row = row.child(rest[..open].to_string());
        }
        let name = &rest[open + 1..close];
        if let Some(i) = things.iter().position(|(n, _)| *n == name) {
            row = row.child(things.remove(i).1);
        }
        rest = &rest[close + 1..];
    }
    if !rest.is_empty() {
        row = row.child(rest.to_string());
    }
    row
}
