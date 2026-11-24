//! A scroll gesture belongs to the scroller it began in, its momentum
//! included, wherever the pointer is carried, and stops at that
//! scroller's edge.
//!
//! Every scroller that goes up and down names itself as it is drawn
//! (`inner_scroller`, `kept_scroll`). One that does not leaks: a flick in
//! it scrolls whatever the pointer reaches next.

use std::time::{Duration, Instant};

use gpui::*;

use crate::kotoba::{Inner, Kotoba};

/// A wheel event this long after the previous one begins a new gesture: a
/// mouse wheel sends no phases, and momentum comes at frame rate.
const SCROLL_GAP: Duration = Duration::from_millis(150);

impl Kotoba {
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
}
