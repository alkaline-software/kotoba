//! The settings' controls: switches (`switch`), and Emaki's choices with
//! a plate that slides (`segmented`), fields whose lists drop down
//! (`settings_pick_h`, `render_pick`), what opens under a row
//! (`fold_under`), and the wheel kept with the scroller it began in
//! (`route_scroll`). Each keeps its state on `Kotoba`, which outlives the
//! window.

use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::{h_flex, v_flex, ActiveTheme as _, Icon, IconName, Sizable as _};

use crate::kotoba::{Inner, KeyGroup, Kotoba, PickFor, PickMenu, PickRow};
use crate::ui::{dictate_key_signs, float_shadow, key_combo, swallow_click, FOLD_ANIM, MENU_IN, MENU_OUT};

/// A wheel event this long after the previous one begins a new gesture: a
/// mouse wheel sends no phases, and momentum comes at frame rate.
const SCROLL_GAP: Duration = Duration::from_millis(150);

/// The settings' slide to Fn's guidance: the share of what is left that
/// each frame takes.
const GLIDE_EASE: f32 = 0.07;

impl Kotoba {
    /// A segmented control: the choices in a row on a muted track, the
    /// chosen one on a raised plate. The plate is one element drawn under
    /// the row, placed from where each segment sat on the last draw
    /// (`seg_bounds`, recorded as the row is prepainted), and it slides
    /// from the old choice to the new one over a moment, keyed on the new
    /// choice so each click plays it once. On the very first draw, before
    /// any segment has been measured, the chosen segment paints its own
    /// plate instead, so nothing flashes.
    /// It fills the width it is given, the choices sharing it, and stands
    /// as tall as the fields and buttons beside it.
    #[allow(clippy::type_complexity)]
    pub fn segmented(&self, control: &'static str, options: Vec<(&'static str, String)>, current: &'static str, on: Rc<dyn Fn(&mut Self, &'static str, &mut Window, &mut Context<Self>)>, cx: &mut Context<Self>) -> AnyElement {
        const SLIDE: Duration = Duration::from_millis(220);
        let (pad, seg_h, seg_px, seg_text, round) = (px(3.), px(22.), px(11.), px(12.5), px(6.));
        let theme = cx.theme().clone();
        let dark = theme.mode.is_dark();
        let track_bg = if dark { theme.tab_bar_segmented } else { theme.muted };
        let plate_bg = if dark { theme.secondary_active } else { theme.popover };
        let (from, to) = {
            let mut st = self.seg_state.borrow_mut();
            let e = st.entry(control).or_insert((current, current, None));
            if e.1 != current {
                e.0 = e.1;
                e.1 = current;
                e.2 = Some(Instant::now());
            }
            // The slide belongs to the change. A control drawn anew later
            // (its window opened again) is a new element, whose animation
            // would start over and slide the plate once more from a
            // choice made long before.
            let sliding = e.2.is_some_and(|at| at.elapsed() < SLIDE + Duration::from_millis(80));
            (if sliding { e.0 } else { e.1 }, e.1)
        };
        let (b_from, b_to) = {
            let b = self.seg_bounds.borrow();
            (b.get(&format!("{control}-{from}")).copied(), b.get(&format!("{control}-{to}")).copied())
        };
        let plate = match (b_from, b_to) {
            (Some(a), Some(b)) => Some(
                div()
                    .absolute()
                    .rounded(round)
                    .bg(plate_bg)
                    .shadow_sm()
                    .with_animation(ElementId::Name(format!("{control}-plate-{to}").into()), Animation::new(SLIDE).with_easing(ease_out_quint()), move |d, t| {
                        let x = a.origin.x + (b.origin.x - a.origin.x) * t;
                        let w = a.size.width + (b.size.width - a.size.width) * t;
                        d.left(x).top(b.origin.y).w(w).h(b.size.height)
                    }),
            ),
            _ => None,
        };
        let measured = plate.is_some();
        let ids: Vec<String> = options.iter().map(|(k, _)| format!("{control}-{k}")).collect();
        let seg_bounds = self.seg_bounds.clone();
        let entity = cx.entity().downgrade();
        let row = h_flex()
            .gap(px(2.))
            .flex_1()
            .min_w_0()
            // Where each segment landed, relative to the track: the row
            // sits inside the track's padding. A change is noted and the
            // next draw places the plate from it.
            .on_children_prepainted(move |bounds, _, cx| {
                let Some(first) = bounds.first() else { return };
                let mut changed = false;
                let mut map = seg_bounds.borrow_mut();
                for (id, b) in ids.iter().zip(bounds.iter()) {
                    let rel = Bounds { origin: point(b.origin.x - first.origin.x + pad, pad), size: b.size };
                    if map.get(id) != Some(&rel) {
                        map.insert(id.clone(), rel);
                        changed = true;
                    }
                }
                drop(map);
                if changed {
                    let _ = entity.update(cx, |_, cx| cx.notify());
                }
            })
            .children(options.into_iter().map(|(key, label)| {
                let active = key == current;
                let on = on.clone();
                h_flex()
                    .id(SharedString::from(format!("{control}-{key}")))
                    .h(seg_h)
                    .px(seg_px)
                    .items_center()
                    .rounded(round)
                    .cursor_pointer()
                    .text_size(seg_text)
                    .flex_1()
                    .justify_center()
                    .when(active && !measured, |d| d.bg(plate_bg).shadow_sm())
                    .when(active, |d| d.font_weight(FontWeight::MEDIUM).text_color(theme.foreground))
                    .when(!active, |d| d.text_color(theme.muted_foreground).hover(|s| s.text_color(theme.foreground)))
                    .on_click(cx.listener(move |this, _, window, cx| on(this, key, window, cx)))
                    // A look has its icon before its word.
                    .when_some(choice_icon(control, key), |d, icon| d.gap(px(5.)).child(Icon::default().path(icon).with_size(seg_text + px(1.5))))
                    .child(label)
            }));
        h_flex()
            .id(SharedString::from(format!("{control}-track")))
            .relative()
            .p(pad)
            .rounded(round + px(2.))
            .bg(track_bg)
            .w_full()
            .children(plate)
            .child(row)
            .into_any_element()
    }

    /// A switch, for what is on or off: a track that fills with the
    /// accent and a knob that slides to its end, both over a moment as it
    /// is turned (`presence`). A press anywhere on it turns it.
    #[allow(clippy::type_complexity)]
    pub fn switch(&self, id: &'static str, on: bool, turn: Rc<dyn Fn(&mut Self, bool, &mut Window, &mut Context<Self>)>, cx: &mut Context<Self>) -> AnyElement {
        const W: f32 = 36.;
        const H: f32 = 20.;
        const KNOB: f32 = 16.;
        const INSET: f32 = 2.;
        let travel = W - KNOB - INSET * 2.;
        let theme = cx.theme().clone();
        let moving = self.presence(&format!("switch-{id}"), on, cx);
        let at = move |t: f32| px(INSET + travel * t);
        let fill = div().absolute().inset_0().rounded_full().bg(theme.primary);
        let knob = div().absolute().top(px(INSET)).size(px(KNOB)).rounded_full().bg(gpui::white()).shadow_sm();
        let (fill, knob) = match moving {
            Some((coming, n)) => (
                fill.with_animation(ElementId::Name(format!("switch-{id}-fill-{n}").into()), Animation::new(FOLD_ANIM).with_easing(ease_out_quint()), move |d, t| d.opacity(if coming { t } else { 1. - t })).into_any_element(),
                knob.with_animation(ElementId::Name(format!("switch-{id}-knob-{n}").into()), Animation::new(FOLD_ANIM).with_easing(ease_out_quint()), move |d, t| d.left(at(if coming { t } else { 1. - t }))).into_any_element(),
            ),
            None => (fill.opacity(if on { 1. } else { 0. }).into_any_element(), knob.left(at(if on { 1. } else { 0. })).into_any_element()),
        };
        div()
            .id(SharedString::from(format!("switch-{id}")))
            .relative()
            .flex_shrink_0()
            .w(px(W))
            .h(px(H))
            .rounded_full()
            .bg(theme.switch)
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| {
                swallow_click(window, cx);
                turn(this, !on, window, cx)
            }))
            .child(fill)
            .child(knob)
            .into_any_element()
    }

    /// A field that opens a list, the controls' width: the choice in the
    /// middle among buttons whose words are, the mark of a list at its
    /// right, that tall to stand level with a pill beside it.
    pub fn settings_pick_h(&self, what: PickFor, id: &'static str, icon: Option<&'static str>, label: String, h: Pixels, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let open = self.pick.as_ref().is_some_and(|p| p.what == what);
        let well = theme.muted;
        let over = theme.muted_foreground.opacity(0.16);
        div()
            .relative()
            .w_full()
            .child(
                h_flex()
                    .id(SharedString::from(format!("{id}-trigger")))
                    .h(h)
                    .px(px(10.))
                    .gap(px(8.))
                    .items_center()
                    .rounded(px(8.))
                    .bg(well)
                    .cursor_pointer()
                    .text_size(px(12.5))
                    .when(open, |d| d.bg(over))
                    .hover(move |s| s.bg(over))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        swallow_click(window, cx);
                        this.pick_toggle(what, cx);
                    }))
                    .relative()
                    .child(
                        // As far from both ends as the list's mark is from
                        // its own, so the words sit in the middle.
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .px(px(18.))
                            .gap(px(7.))
                            .items_center()
                            .justify_center()
                            .children(icon.map(|icon| Icon::default().path(icon).with_size(px(14.)).text_color(theme.muted_foreground)))
                            .child(div().min_w_0().truncate().child(label)),
                    )
                    .child(div().absolute().right(px(10.)).top_0().bottom_0().flex().items_center().child(Icon::default().path("icons/chevrons-up-down.svg").with_size(px(13.)).text_color(theme.muted_foreground))),
            )
            .child(self.pick_mark(what, cx))
            .into_any_element()
    }

    /// Laid over whatever opens a list, it keeps where that is drawn.
    pub fn pick_mark(&self, what: PickFor, cx: &Context<Self>) -> impl IntoElement {
        let at = self.pill_at.clone();
        let entity = cx.entity().downgrade();
        div().absolute().inset_0().child(
            canvas(
                // What opens a list moves as what is beside it changes,
                // and its list is drawn where it was a frame ago: a move
                // asks for one more draw.
                move |b, _, cx| {
                    if at.borrow_mut().insert(what, b) != Some(b) {
                        if let Some(e) = entity.upgrade() {
                            cx.defer(move |cx| {
                                e.update(cx, |this, cx| {
                                    if this.pick.is_some() {
                                        cx.notify();
                                    }
                                })
                            });
                        }
                    }
                },
                |_, _, _, _| {},
            )
            // Over the field: with no place given it stood after it, a
            // field's height lower.
            .absolute()
            .inset_0(),
        )
    }

    /// A press on a field with a list: the list opens, or, open, closes.
    pub fn pick_toggle(&mut self, what: PickFor, cx: &mut Context<Self>) {
        if self.pick.as_ref().is_some_and(|p| p.what == what) {
            return self.close_pick(cx);
        }
        let (rows, current) = self.pick_rows(what);
        let by = self.pill_at.borrow().get(&what).copied().unwrap_or_default();
        if rows.is_empty() {
            return;
        }
        // One plugged in since the window came is in the list once read.
        if what == PickFor::Mic {
            self.read_mics(cx);
        }
        self.pick_serial += 1;
        self.pick_gone = None;
        self.key_group = None;
        self.key_group_gone = None;
        self.pick = Some(PickMenu { what, rows: Rc::new(rows), current, by, serial: self.pick_serial });
        cx.notify();
    }

    /// A kind on More's list pointed at or pressed: its keys open beside
    /// it, and the kind open before goes out fading.
    pub fn open_key_group(&mut self, group: KeyGroup, cx: &mut Context<Self>) {
        if self.key_group.is_some_and(|(g, _)| g == group) {
            return;
        }
        if let Some((g, n)) = self.key_group.take() {
            self.key_group_gone = Some((g, n, Instant::now()));
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(MENU_OUT + Duration::from_millis(20)).await;
                let _ = this.update(cx, |this, cx| {
                    if this.key_group_gone.as_ref().is_some_and(|(_, _, at)| at.elapsed() >= MENU_OUT) {
                        this.key_group_gone = None;
                    }
                    cx.notify();
                });
            })
            .detach();
        }
        self.pick_serial += 1;
        self.key_group = Some((group, self.pick_serial));
        cx.notify();
    }

    /// The open list's choices read again, when it is `what`'s: what it
    /// offers changed while it was open.
    pub fn refresh_pick(&mut self, what: PickFor) {
        if self.pick.as_ref().is_some_and(|p| p.what == what) {
            let (rows, current) = self.pick_rows(what);
            if let Some(p) = self.pick.as_mut() {
                p.rows = Rc::new(rows);
                p.current = current;
            }
        }
    }

    /// The list is put away: it fades where it was, taking no click.
    pub fn close_pick(&mut self, cx: &mut Context<Self>) {
        if let Some(mut menu) = self.pick.take() {
            menu.by = self.pill_at.borrow().get(&menu.what).copied().unwrap_or(menu.by);
            self.pick_gone = Some((menu, Instant::now()));
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(MENU_OUT + Duration::from_millis(20)).await;
                let _ = this.update(cx, |this, cx| {
                    if this.pick_gone.as_ref().is_some_and(|(_, at)| at.elapsed() >= MENU_OUT) {
                        this.pick_gone = None;
                    }
                    cx.notify();
                });
            })
            .detach();
        }
        cx.notify();
    }

    /// A choice pressed on a list. The one already set changes nothing.
    fn pick_chosen(&mut self, what: PickFor, key: &str, cx: &mut Context<Self>) {
        let same = self.pick.as_ref().is_some_and(|p| p.current == key);
        self.close_pick(cx);
        if same {
            return;
        }
        match what {
            PickFor::DictateKey => self.set_dictate_key(key.to_string(), cx),
            PickFor::DictateKeep => self.set_dictate_keep(key, cx),
            PickFor::Mic => self.set_mic(key, cx),
            PickFor::Language => self.set_language(key, cx),
        }
    }

    /// The choices of a list as they stand now, and the one that is set.
    fn pick_rows(&self, what: PickFor) -> (Vec<PickRow>, String) {
        use kotoba_core::dictate;
        match what {
            // The kinds of key, each opening its keys beside it
            // (`render_key_group`). What the Mac does with Fn, or with F1
            // to F12, is said under the key's row once it is chosen.
            PickFor::DictateKey => {
                let rows = KeyGroup::ALL.iter().map(|g| PickRow { key: g.key().to_string(), name: g.name().to_string(), rule: false }).collect();
                (rows, self.cfg.app.dictate_key.clone())
            }
            // The system's, then each language by its flag and letters.
            PickFor::Language => {
                let system = PickRow { key: "system".into(), name: crate::i18n::t("System").into(), rule: false };
                let rows = std::iter::once(system).chain(crate::i18n::CHOICES.iter().enumerate().map(|(ix, (code, flag, letters))| PickRow { key: code.to_string(), name: format!("{flag}  {letters}"), rule: ix == 0 })).collect();
                (rows, self.cfg.app.language.clone())
            }
            // The system's, by the name of the one it has, then under a
            // line each the Mac has.
            PickFor::Mic => {
                let system = PickRow { key: "system".into(), name: match &self.system_mic {
                    Some(name) => crate::i18n::tf("System ({name})", &[("name", name)]),
                    None => crate::i18n::t("System").to_string(),
                }, rule: false };
                let rows = std::iter::once(system).chain(self.mics.iter().enumerate().map(|(ix, m)| PickRow { key: m.id.clone(), name: m.name.clone(), rule: ix == 0 })).collect();
                let current = if self.cfg.app.dictate_mic.is_empty() { "system".to_string() } else { self.cfg.app.dictate_mic.clone() };
                (rows, current)
            }
            // Not saving, then the times, then always.
            PickFor::DictateKeep => {
                let rows = dictate::KEEPS.iter().enumerate().map(|(ix, k)| PickRow { key: k.0.to_string(), name: crate::i18n::t(k.1).to_string(), rule: ix == 1 }).collect();
                (rows, dictate::keep(&self.cfg.app.dictate_keep).0.to_string())
            }
        }
    }

    /// A field's list, hanging off it: up from one low in the window,
    /// down from one high in it, its edge on the field's nearer the
    /// window's side. It comes in over a moment, rising into place, and
    /// goes out fading.
    pub fn render_pick(&self, menu: PickMenu, gone: bool, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let view = window.viewport_size();
        let up = menu.by.center().y > view.height * 0.5;
        let keys = menu.what == PickFor::DictateKey;
        let right = menu.by.left() + px(240.) > view.width - px(8.);
        let gap = px(6.);
        let room = if up { menu.by.top() - gap - px(12.) } else { view.height - menu.by.bottom() - gap - px(12.) };
        let what = menu.what;
        let (scroll, _) = self.kept_scroll(format!("pick-{}", menu.serial).into(), Inner::Over);
        // The room above and below the rows is the card's, not theirs, so a
        // row cut at an edge stays inside its corner: a clip is a rectangle.
        let mut rows = v_flex().id(("pick-rows", menu.serial as usize)).max_h(room.min(px(440.)) - px(10.)).overflow_y_scroll().track_scroll(&scroll).px(px(5.)).gap(px(2.));
        let card = v_flex()
            .id(("pick-card", menu.serial as usize))
            .absolute()
            .min_w(if keys { px(220.) } else { menu.by.size.width })
            .max_w(px(if keys { 320. } else { 240. }))
            .py(px(5.))
            .overflow_hidden()
            .rounded(px(12.))
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .shadow(float_shadow(&theme))
            .map(|d| if right { d.right(view.width - menu.by.right()) } else { d.left(menu.by.left()) })
            .on_mouse_down(MouseButton::Left, |_, window, cx| swallow_click(window, cx));
        let dark = theme.mode.is_dark();
        let lit = if dark { theme.foreground.opacity(0.11) } else { theme.list_hover };
        for (ix, PickRow { key, name, rule }) in menu.rows.iter().enumerate() {
            // More's list: each kind, with the key in use at the right of
            // its own, and the mark of a list that opens beside it.
            if let Some(group) = keys.then(|| KeyGroup::ALL[ix]) {
                let open = self.key_group.is_some_and(|(g, _)| g == group);
                let at = self.key_group_at.clone();
                let quiet = theme.muted_foreground;
                rows = rows.child(
                    h_flex()
                        .id(("pick-row", ix))
                        .relative()
                        .flex_shrink_0()
                        .h(px(30.))
                        .px(px(10.))
                        .gap(px(10.))
                        .items_center()
                        .rounded(px(8.))
                        .cursor_pointer()
                        .when(open, |d| d.bg(lit))
                        .hover(move |s| s.bg(lit))
                        .when(!gone, |d| {
                            d.on_hover(cx.listener(move |this, on: &bool, _, cx| {
                                if *on {
                                    this.open_key_group(group, cx);
                                }
                            }))
                            .on_click(cx.listener(move |this, _, _, cx| this.open_key_group(group, cx)))
                        })
                        .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(name.clone()))
                        .child(div().flex_1())
                        .when(KeyGroup::of(&menu.current) == Some(group), |d| d.child(key_combo(&dictate_key_signs(&menu.current), quiet).text_size(px(13.)).text_color(quiet)))
                        .child(Icon::default().path("icons/chevron-right.svg").with_size(px(12.)).text_color(quiet))
                        .child(
                            canvas(
                                move |b, _, _| {
                                    at.borrow_mut().insert(group, b);
                                },
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .inset_0(),
                        ),
                );
                continue;
            }
            let active = *key == menu.current && !key.is_empty();
            if *rule && ix > 0 {
                rows = rows.child(div().flex_shrink_0().h(px(1.)).mx(px(6.)).my(px(3.)).bg(theme.border));
            }
            let key = key.clone();
            rows = rows.child(
                h_flex()
                    .id(("pick-row", ix))
                    .flex_shrink_0()
                    .px(px(10.))
                    .py(px(6.))
                    .gap(px(6.))
                    .items_center()
                    .rounded(px(8.))
                    .cursor_pointer()
                    // The dark palette's own hover is a shade off the
                    // card and cannot be seen on it: a wash of the ink.
                    .when(active, |d| d.bg(if dark { theme.foreground.opacity(0.06) } else { theme.muted }))
                    .hover(|s| s.bg(if dark { theme.foreground.opacity(0.11) } else { theme.list_hover }))
                    .when(!gone, |d| d.on_click(cx.listener(move |this, _, _, cx| this.pick_chosen(what, &key, cx))))
                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(name.clone()))
                    .when(active, |d| d.child(div().flex_1()).child(Icon::new(IconName::Check).with_size(px(12.)).text_color(theme.primary))),
            );
        }
        let card = card.child(rows).vertical_scrollbar(&scroll);
        // Where it rests: its foot over the field, or its head under it.
        let rest = if up { view.height - menu.by.top() + gap } else { menu.by.bottom() + gap };
        let place = move |d: Stateful<Div>, off: Pixels| if up { d.bottom(rest - off) } else { d.top(rest - off) };
        // On More's list, the open kind's keys beside it, and the kind
        // open before going out.
        let open_group = if keys { self.key_group.and_then(|(g, n)| self.render_key_group(g, n, &menu.current, gone, window, cx)) } else { None };
        let gone_group = if keys && !gone { self.key_group_gone.and_then(|(g, n, at)| (at.elapsed() < MENU_OUT).then(|| self.render_key_group(g, n, &menu.current, true, window, cx)).flatten()) } else { None };
        if gone {
            let card = card.with_animation(ElementId::Name(format!("pick-out-{}", menu.serial).into()), Animation::new(MENU_OUT), move |d, t| place(d, px(0.)).opacity(1. - t));
            return div().absolute().inset_0().child(card).children(open_group).into_any_element();
        }
        let card = card.with_animation(ElementId::Name(format!("pick-in-{}", menu.serial).into()), Animation::new(MENU_IN).with_easing(ease_out_quint()), move |d, t| place(d, px(6.) * (1. - t)).opacity(t));
        let shut = |this: &mut Self, _: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>| this.close_pick(cx);
        div().id("pick-sheet").absolute().inset_0().occlude().on_mouse_down(MouseButton::Left, cx.listener(shut)).on_mouse_down(MouseButton::Right, cx.listener(shut)).child(card).children(gone_group).children(open_group).into_any_element()
    }

    /// A kind's keys, in a list beside its row on More's: at the side
    /// with room, its first key level with the row, coming in as a list
    /// does, from the row's side. The modifiers are listed on their caps;
    /// the F keys stand four to a row, as a keyboard groups them, with a
    /// line under F12, where a Mac's own keys end.
    fn render_key_group(&self, group: KeyGroup, serial: u32, current: &str, gone: bool, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        use kotoba_core::dictate;
        let row = *self.key_group_at.borrow().get(&group)?;
        let theme = cx.theme().clone();
        let dark = theme.mode.is_dark();
        let view = window.viewport_size();
        let lit = if dark { theme.foreground.opacity(0.11) } else { theme.list_hover };
        let chosen = if dark { theme.foreground.opacity(0.06) } else { theme.muted };
        // The list's own edge is 6 from its rows; this one stands 4 off it.
        let (inner, gap) = (px(6.), px(4.));
        // About as wide and tall as it is drawn, to choose its side and
        // keep it in the window.
        let (wide, tall) = match group {
            KeyGroup::One => (px(140.), px(5. * 32. + 10.)),
            KeyGroup::Two => (px(140.), px(6. * 32. + 10.)),
            KeyGroup::F => (px(206.), px(5. * 32. + 7. + 10.)),
        };
        let to_right = row.right() + inner + gap + wide <= view.width - px(8.);
        let top = (row.top() - inner).min(view.height - px(12.) - tall).max(px(12.));
        let body: AnyElement = match group {
            KeyGroup::One | KeyGroup::Two => {
                let mut list = v_flex().gap(px(2.));
                for (ix, tap) in dictate::TAPS.iter().filter(|t| t.pair() == (group == KeyGroup::Two)).enumerate() {
                    let full = format!("{}{}", dictate::TAP_PREFIX, tap.key);
                    let active = full == current;
                    list = list.child(
                        h_flex()
                            .id((SharedString::from(format!("key-group-{}", group.key())), ix))
                            .h(px(30.))
                            .px(px(10.))
                            .gap(px(6.))
                            .items_center()
                            .rounded(px(8.))
                            .cursor_pointer()
                            .when(active, |d| d.bg(chosen))
                            .hover(move |s| s.bg(lit))
                            .when(!gone, |d| d.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.pick_chosen(PickFor::DictateKey, &full, cx))))
                            .child(key_combo(&crate::ui::tap_signs(tap), theme.muted_foreground).text_size(px(13.)).font_weight(FontWeight::MEDIUM))
                            .when(active, |d| d.child(div().flex_1()).child(Icon::new(IconName::Check).with_size(px(12.)).text_color(theme.primary))),
                    );
                }
                list.into_any_element()
            }
            KeyGroup::F => {
                let mut grid = v_flex().gap(px(2.));
                for line in 0..dictate::F_KEYS / 4 {
                    if line == 3 {
                        grid = grid.child(div().h(px(1.)).mx(px(6.)).my(px(3.)).bg(theme.border));
                    }
                    grid = grid.child(h_flex().gap(px(2.)).children((1..=4).map(|i| {
                        let n = line * 4 + i;
                        let full = format!("f{n}");
                        let active = full == current;
                        h_flex()
                            .id(("key-group-f", n as usize))
                            .w(px(47.))
                            .h(px(30.))
                            .justify_center()
                            .items_center()
                            .rounded(px(8.))
                            .cursor_pointer()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .when(active, |d| d.bg(chosen).text_color(theme.primary))
                            .hover(move |s| s.bg(lit))
                            .when(!gone, |d| d.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.pick_chosen(PickFor::DictateKey, &full, cx))))
                            .child(format!("F{n}"))
                    })));
                }
                grid.into_any_element()
            }
        };
        let card = v_flex()
            .id(("key-group", serial as usize))
            .absolute()
            .top(top)
            .min_w(px(120.))
            .p(px(5.))
            .rounded(px(12.))
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .shadow(float_shadow(&theme))
            .on_mouse_down(MouseButton::Left, |_, window, cx| swallow_click(window, cx))
            .child(body);
        // Beside the list: from its right edge, or to its left.
        let (from_left, from_right) = (row.right() + inner + gap, view.width - (row.left() - inner) + gap);
        let place = move |d: Stateful<Div>, off: Pixels| if to_right { d.left(from_left - off) } else { d.right(from_right - off) };
        let id = ElementId::Name(format!("key-group-{}-{serial}", if gone { "out" } else { "in" }).into());
        Some(if gone {
            card.with_animation(id, Animation::new(MENU_OUT), move |d, t| place(d, px(0.)).opacity(1. - t)).into_any_element()
        } else {
            card.with_animation(id, Animation::new(MENU_IN).with_easing(ease_out_quint()), move |d, t| place(d, px(6.) * (1. - t)).opacity(t)).into_any_element()
        })
    }

    /// How something small stands at this draw, asked at every draw
    /// whether it is there or not, so a change is seen as one: whether it
    /// is coming, and the number of the change, while it moves; nothing
    /// at rest. What was never drawn before is not played. One going is
    /// drawn until its time is up, and once more then as nothing.
    pub fn presence(&self, id: &str, on: bool, cx: &mut Context<Self>) -> Option<(bool, u32)> {
        let mut all = self.presences.borrow_mut();
        let now = Instant::now();
        let p = all.entry(id.to_string()).or_insert_with(|| (on, now.checked_sub(FOLD_ANIM * 2).unwrap_or(now), 0));
        if p.0 != on {
            *p = (on, now, p.2 + 1);
            if !on {
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(FOLD_ANIM + Duration::from_millis(20)).await;
                    let _ = this.update(cx, |_, cx| cx.notify());
                })
                .detach();
            }
        }
        (p.2 > 0 && p.1.elapsed() < FOLD_ANIM).then_some((on, p.2))
    }

    /// `presence` for what is played the first time it is seen too.
    fn arrival(&self, id: &str, on: bool, cx: &mut Context<Self>) -> Option<(bool, u32)> {
        if on && !self.presences.borrow().contains_key(id) {
            self.presence(id, false, cx);
        }
        self.presence(id, on, cx)
    }

    /// A card that opens down from the row over it, as a note under a
    /// row of the settings: its height runs up from nothing and back, the
    /// gap above it part of what moves. It measures itself as it is
    /// drawn, and is drawn a moment more as it closes.
    pub fn fold_under(&self, key: String, on: bool, card: AnyElement, cx: &mut Context<Self>) -> Option<AnyElement> {
        let moving = self.arrival(&key, on, cx);
        if !on && moving.is_none() {
            self.fold_h.borrow_mut().remove(&key);
            return None;
        }
        let tall = self.fold_h.borrow_mut().entry(key.clone()).or_default().clone();
        let measure = on.then(|| {
            let (tall, entity) = (tall.clone(), cx.entity().downgrade());
            canvas(
                move |b, _, cx| {
                    let h = f32::from(b.size.height);
                    if (tall.get() - h).abs() > 0.5 {
                        tall.set(h);
                        if let Some(e) = entity.upgrade() {
                            cx.defer(move |cx| e.update(cx, |_, cx| cx.notify()));
                        }
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .inset_0()
        });
        let gap = 8.;
        let inner = div().relative().w_full().flex_shrink_0().child(card).children(measure).when(!on, |d| d.child(div().absolute().inset_0().occlude()));
        let wrap = v_flex().w_full().justify_start().overflow_hidden().child(div().h(px(gap)).flex_shrink_0()).child(inner);
        Some(match moving {
            Some((coming, n)) => wrap
                .with_animation(ElementId::Name(format!("{key}-{n}").into()), Animation::new(FOLD_ANIM).with_easing(ease_out_quint()), move |d, t| {
                    let t = if coming { t } else { 1. - t };
                    let h = tall.get();
                    if h <= 0. {
                        d.h(px(0.)).opacity(0.)
                    } else {
                        d.h(px(((h + gap) * t).ceil())).opacity(t)
                    }
                })
                .into_any_element(),
            None => wrap.into_any_element(),
        })
    }

    /// A scroller drawn now: `route_scroll` keeps a gesture that begins in
    /// it with it.
    pub fn inner_scroller(&self, handle: &ScrollHandle, kind: Inner) {
        self.inner_live.borrow_mut().push((handle.clone(), kind));
    }

    /// The handle of such a scroller that has no field of its own, kept
    /// under `key` for as long as it is drawn, and whether it is new.
    pub fn kept_scroll(&self, key: SharedString, kind: Inner) -> (ScrollHandle, bool) {
        let mut kept = self.inner_kept.borrow_mut();
        let new = !kept.contains_key(&key);
        let entry = kept.entry(key).or_default();
        entry.1 = true;
        self.inner_scroller(&entry.0, kind);
        (entry.0.clone(), new)
    }

    /// Keep a scroll gesture's momentum in the scroller it began in.
    /// macOS goes on sending wheel events after the finger lifts, and gpui
    /// hands each to whatever is under the pointer by then, so a flick on
    /// a sheet carried off it scrolled the settings under it. This runs in
    /// the capture phase, before any scroll container. Of the scrollers
    /// that name themselves, the one under the pointer drawn last is the
    /// one on top: a sheet's list over the settings'. A gesture begun in
    /// one is its own; begun elsewhere and carried onto one, it moves
    /// nothing there. A sheet itself (`Inner::Wall`) ends the search, so
    /// the empty Terms sheet does not scroll the settings under it.
    pub fn route_scroll(&mut self, e: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let now = Instant::now();
        let fresh = now.duration_since(self.last_scroll) > SCROLL_GAP;
        self.last_scroll = now;
        let inner = self.inner_live.borrow().iter().rev().filter(|(h, _)| h.bounds().contains(&e.position)).find(|(h, kind)| *kind == Inner::Wall || h.max_offset().y > px(0.)).cloned();
        let begins = match e.touch_phase {
            TouchPhase::Started => true,
            TouchPhase::Moved => fresh || !self.scroll_began,
            _ => false,
        };
        if begins {
            self.scroll_began = true;
            self.scroll_inner = inner.clone().filter(|(_, kind)| *kind != Inner::Wall);
        }
        // Begun on a sheet with nothing to scroll under the pointer: a
        // field on it scrolls itself, and nothing under it moves.
        if inner.as_ref().is_some_and(|(_, kind)| *kind == Inner::Wall) && self.scroll_inner.is_none() {
            return;
        }
        if let Some((own, _)) = self.scroll_inner.clone() {
            // Its own and nobody else's, wherever the pointer has been
            // carried and whether or not it has more to give.
            let at = own.offset().y + e.delta.pixel_delta(px(20.)).y;
            own.set_offset(point(own.offset().x, at.clamp(-own.max_offset().y, px(0.))));
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if inner.is_some() {
            cx.stop_propagation();
        }
    }

    /// One frame of the settings sliding to their end, where Fn's guidance
    /// opens: a share of what is left each frame, for as long as the
    /// guidance is growing and a moment after.
    pub fn settings_glide_step(&self, window: &mut Window) {
        let Some((began, last)) = self.settings_glide.take() else { return };
        let scroll = &self.settings_scroll;
        let to = -f32::from(scroll.max_offset().y);
        let at = scroll.offset();
        let left = to - f32::from(at.y);
        let growing = began.elapsed() < FOLD_ANIM + Duration::from_millis(250);
        if left.abs() < 0.5 && !growing {
            return;
        }
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.05);
        let step = left * (1. - (-dt / GLIDE_EASE).exp());
        scroll.set_offset(point(at.x, at.y + px(step)));
        if growing || left.abs() >= 0.5 {
            self.settings_glide.set(Some((began, now)));
            window.request_animation_frame();
        }
    }

    /// Empty space along the window's top moves the window. The app owns
    /// that drag (`app_owns_titlebar_drag`): a press there becomes a move
    /// once the pointer moves with the button down, and a double click is
    /// the title bar's.
    pub fn drag_region(el: Div, cx: &mut Context<Self>) -> Div {
        el.on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, ev: &MouseDownEvent, window, _| {
                this.win_move = true;
                if ev.click_count == 2 {
                    this.win_move = false;
                    window.titlebar_double_click();
                }
            }),
        )
        .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, window, _| {
            if !this.win_move {
                return;
            }
            this.win_move = false;
            if ev.pressed_button == Some(MouseButton::Left) {
                window.start_window_move();
            }
        }))
    }
}

/// The icon before a choice's word, for the choices that have one.
fn choice_icon(control: &str, key: &str) -> Option<&'static str> {
    match (control, key) {
        ("appearance", "system") => Some("icons/device-desktop.svg"),
        ("appearance", "light") => Some("icons/sun.svg"),
        ("appearance", "dark") => Some("icons/moon.svg"),
        _ => None,
    }
}
