//! The frame round the screen while Kotoba is being told what to do: four
//! heavy marks at its corners, as a camera's viewfinder frames what it
//! looks at. Each mark comes out of its corner along both edges as control
//! starts, reaches further as the person speaks, slides round the frame
//! with the others while the steps are worked out and done, and draws back
//! to a point as control ends. Nothing scales: a mark is a stretch of the
//! frame's own outline, longer or shorter. Under each mark is a band of
//! color a little longer than it, its hue set by where it is round the
//! screen, the colors turning slowly, and faster while it works.
//!
//! It is Core Animation's, not the toolkit's: a window over everything
//! that takes no click, with shape layers drawn by the system at the
//! screen's own rate, costing this app no frames while the model has the
//! GPU. Called on the main thread alone, as AppKit asks.

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{msg_send, MainThreadMarker};
use objc2_app_kit::{NSBackingStoreType, NSColor, NSPanel, NSScreen, NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace};
use objc2_foundation::{NSNumber, NSPoint, NSRect, NSSize, NSString, NSUserDefaults};

/// How far in from the screen's edge the frame is, and its corners' radius.
const INSET: f64 = 5.;
const RADIUS: f64 = 10.;
/// A mark's stroke, and the band of color under it.
const STROKE: f64 = 4.;
const BAND: f64 = 10.;
/// The band in eight layers, each reaching this much beside the mark and
/// letting `BAND_STEP` of the color through, so the band fades out past
/// the mark's ends in steps too small to see; all of it at most
/// `BAND_OPACITY`.
const BAND_REACH: [f64; 8] = [1.14, 1.1, 1.06, 1.02, 0.98, 0.94, 0.9, 0.84];
const BAND_STEP: f64 = 0.22;
const BAND_OPACITY: f32 = 0.92;
/// The colors' turn, in degrees a second: at rest, and added while it works.
const HUE_TURN: f64 = 12.;
const HUE_WORK: f64 = 70.;
/// Half a mark's length along the outline, each way from its corner: at
/// rest, the most the voice adds, and while it works.
const ARM: f64 = 52.;
const REACH: f64 = 110.;
const ARM_WORK: f64 = 72.;
/// How fast the marks slide round while it works, in points a second, and
/// how long they take to come up to that speed.
const SLIDE: f64 = 260.;
const SPEED_UP: f64 = 1.2;
const IN_S: f64 = 0.55;
const OUT_S: f64 = 0.4;
/// Points of a corner's arc in the outline.
const ARC_STEPS: usize = 24;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorCreateSRGB(r: f64, g: f64, b: f64, a: f64) -> *mut c_void;
    fn CGColorRelease(color: *mut c_void);
    fn CGPathCreateMutable() -> *mut c_void;
    fn CGPathMoveToPoint(path: *mut c_void, transform: *const c_void, x: f64, y: f64);
    fn CGPathAddLineToPoint(path: *mut c_void, transform: *const c_void, x: f64, y: f64);
    fn CGPathRelease(path: *const c_void);
}

#[link(name = "QuartzCore", kind = "framework")]
extern "C" {
    fn CACurrentMediaTime() -> f64;
}

/// Core Animation's `CAFrameRateRange`: the frames a second an animation
/// asks for.
#[repr(C)]
#[derive(Clone, Copy)]
struct FrameRate {
    minimum: f32,
    maximum: f32,
    preferred: f32,
}

unsafe impl objc2::Encode for FrameRate {
    const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CAFrameRateRange", &[f32::ENCODING, f32::ENCODING, f32::ENCODING]);
}

/// How an animation runs: eased in and held at its end (`In`), even and
/// over and over at the screen's rate (`Round`), or the same at 15
/// frames a second (`Drift`), for a motion so slow that a frame more is
/// not seen. Anything moving in the frame's window has WindowServer draw
/// at the screen's rate, a quarter of a core on a 5K screen at 60 Hz.
#[derive(PartialEq)]
enum Pace {
    In,
    Round,
    Drift,
}

struct Glow {
    window: Retained<NSPanel>,
    /// The band's layers and the marks, each on the same outline, with how
    /// far each reaches beside a mark's length.
    marks: Vec<(Retained<AnyObject>, f64)>,
    /// What turns the colors faster while it works.
    spin: Retained<AnyObject>,
    /// The outline's length once round: each layer's path goes round twice.
    round: f64,
    sliding: bool,
}

/// The one frame, on the main thread alone.
struct One(Option<Glow>);
// Touched only from the main thread (`on_main`).
unsafe impl Send for One {}
static GLOW: std::sync::Mutex<One> = std::sync::Mutex::new(One(None));

fn layer(class: &str) -> Retained<AnyObject> {
    let name = std::ffi::CString::new(class).unwrap();
    let class = AnyClass::get(&name).expect("QuartzCore's layers");
    unsafe { msg_send![class, layer] }
}

fn set_key(target: &AnyObject, key: &str, value: f64) {
    let (key, value) = (NSString::from_str(key), NSNumber::new_f64(value));
    unsafe {
        let _: () = msg_send![target, setValue: &*value, forKeyPath: &*key];
    }
}

/// A Core Foundation value (a color, a path) set by key, as Core Animation
/// takes them.
fn set_ref(target: &AnyObject, key: &str, value: *const c_void) {
    let key = NSString::from_str(key);
    unsafe {
        let value: Option<&AnyObject> = (value as *const AnyObject).as_ref();
        let _: () = msg_send![target, setValue: value, forKey: &*key];
    }
}

fn set_text(target: &AnyObject, key: &str, value: &str) {
    let (key, value) = (NSString::from_str(key), NSString::from_str(value));
    unsafe {
        let _: () = msg_send![target, setValue: &*value, forKey: &*key];
    }
}

/// In one motion, or at once.
fn together(seconds: f64, change: impl FnOnce()) {
    let class = AnyClass::get(c"CATransaction").expect("QuartzCore");
    unsafe {
        let _: () = msg_send![class, begin];
        let _: () = msg_send![class, setAnimationDuration: seconds];
        if seconds == 0. {
            let _: () = msg_send![class, setDisableActions: true];
        } else {
            let name = NSString::from_str("easeOut");
            let curve = AnyClass::get(c"CAMediaTimingFunction").expect("QuartzCore");
            let curve: Retained<AnyObject> = msg_send![curve, functionWithName: &*name];
            let _: () = msg_send![class, setAnimationTimingFunction: &*curve];
        }
    }
    change();
    unsafe {
        let _: () = msg_send![class, commit];
    }
}

/// The frame's outline: a rounded box `INSET` in, as points from the
/// middle of its top, clockwise as seen, with the length run to each, and
/// where on it each corner's middle is. In the view's points, the origin
/// at the bottom left.
struct Outline {
    points: Vec<(f64, f64)>,
    run: Vec<f64>,
    corners: [f64; 4],
}

impl Outline {
    fn new(size: NSSize) -> Outline {
        let (w, h) = (size.width, size.height);
        let (left, right, bottom, top) = (INSET, w - INSET, INSET, h - INSET);
        let r = RADIUS.min((right - left) / 2.).min((top - bottom) / 2.);
        let mut points = vec![(w / 2., top)];
        let mut corners = [0.; 4];
        // Each corner's arc, from the angle it starts at, clockwise as seen
        // (the angle falling, with y up).
        let arcs = [(right - r, top - r, 90f64), (right - r, bottom + r, 0.), (left + r, bottom + r, -90.), (left + r, top - r, -180.)];
        let mut middles = Vec::new();
        for (cx, cy, from) in arcs {
            for k in 0..=ARC_STEPS {
                let a = (from - 90. * k as f64 / ARC_STEPS as f64).to_radians();
                points.push((cx + r * a.cos(), cy + r * a.sin()));
                if k == ARC_STEPS / 2 {
                    middles.push(points.len() - 1);
                }
            }
        }
        points.push((w / 2., top));
        let mut run = vec![0.];
        for k in 1..points.len() {
            let (a, b) = (points[k - 1], points[k]);
            run.push(run[k - 1] + ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt());
        }
        for (c, &m) in corners.iter_mut().zip(&middles) {
            *c = run[m];
        }
        Outline { points, run, corners }
    }

    fn round(&self) -> f64 {
        *self.run.last().unwrap()
    }

    fn at(&self, d: f64) -> (f64, f64) {
        let d = d.rem_euclid(self.round());
        let k = self.run.partition_point(|&l| l <= d).clamp(1, self.run.len() - 1);
        let f = (d - self.run[k - 1]) / (self.run[k] - self.run[k - 1]).max(1e-9);
        let (a, b) = (self.points[k - 1], self.points[k]);
        (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
    }

    /// The outline twice round, from `from`. Answers a path the caller
    /// releases.
    fn path_from(&self, from: f64) -> *mut c_void {
        let round = self.round();
        let path = unsafe { CGPathCreateMutable() };
        let (x, y) = self.at(from);
        unsafe { CGPathMoveToPoint(path, std::ptr::null(), x, y) };
        let start = from.rem_euclid(round);
        let mut base = from - start;
        let mut k = self.run.partition_point(|&l| l <= start);
        loop {
            if k >= self.run.len() {
                k = 1;
                base += round;
            }
            if base + self.run[k] >= from + 2. * round {
                break;
            }
            let (x, y) = self.points[k];
            unsafe { CGPathAddLineToPoint(path, std::ptr::null(), x, y) };
            k += 1;
        }
        let (x, y) = self.at(from + 2. * round);
        unsafe { CGPathAddLineToPoint(path, std::ptr::null(), x, y) };
        path
    }
}

/// A mark's layer: the outline twice round from half a round before its
/// corner, so the corner is a quarter of the way along and a mark can
/// slide a whole round and still be on the path.
fn mark(outline: &Outline, corner: f64, size: NSSize, width: f64, color: (f64, f64, f64, f64)) -> Retained<AnyObject> {
    let shape = layer("CAShapeLayer");
    unsafe {
        let _: () = msg_send![&*shape, setFrame: NSRect::new(NSPoint::new(0., 0.), size)];
    }
    let path = outline.path_from(corner - outline.round() / 2.);
    set_ref(&shape, "path", path);
    unsafe { CGPathRelease(path) };
    let color = unsafe { CGColorCreateSRGB(color.0, color.1, color.2, color.3) };
    set_ref(&shape, "strokeColor", color);
    unsafe { CGColorRelease(color) };
    set_ref(&shape, "fillColor", std::ptr::null());
    set_key(&shape, "lineWidth", width);
    set_text(&shape, "lineCap", "butt");
    set_text(&shape, "lineJoin", "round");
    shape
}

/// Each mark `arm` each way from its corner, where a mark rests, and the
/// band's layers as far beside it as each reaches.
fn reach(glow: &Glow, arm: f64) {
    for (m, k) in &glow.marks {
        let half = arm * k / (2. * glow.round);
        set_key(m, "strokeStart", 0.25 - half);
        set_key(m, "strokeEnd", 0.25 + half);
    }
}

fn calm() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

/// Dark ink on a light Mac, light ink on a dark one, and the lightness of
/// the band's colors to go with it.
fn inks() -> ((f64, f64, f64, f64), f64) {
    let style = NSUserDefaults::standardUserDefaults().stringForKey(&NSString::from_str("AppleInterfaceStyle"));
    if style.map(|s| s.to_string() == "Dark").unwrap_or(false) {
        ((0.953, 0.945, 0.918, 1.), 0.66)
    } else {
        ((0.110, 0.129, 0.114, 1.), 0.56)
    }
}

/// A hue in degrees at saturation 0.72 and lightness `l`, as sRGB.
fn hue(h: f64, l: f64) -> (f64, f64, f64) {
    let c = (1. - (2. * l - 1.).abs()) * 0.72;
    let h = h.rem_euclid(360.) / 60.;
    let x = c * (1. - (h % 2. - 1.).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.),
        1 => (x, c, 0.),
        2 => (0., c, x),
        3 => (0., x, c),
        4 => (x, 0., c),
        _ => (c, 0., x),
    };
    let m = l - c / 2.;
    (r + m, g + m, b + m)
}

/// The wheel of colors the band shows through: a conic gradient round the
/// screen's middle, red at the right, in a square that covers the screen
/// at any turn.
fn wheel(size: NSSize, lightness: f64) -> Retained<AnyObject> {
    let side = (size.width.powi(2) + size.height.powi(2)).sqrt();
    let wheel = layer("CAGradientLayer");
    let colors: Retained<AnyObject> = unsafe { msg_send![AnyClass::get(c"NSMutableArray").expect("Foundation"), array] };
    for k in 0..=12 {
        let (r, g, b) = hue(k as f64 * 30., lightness);
        unsafe {
            let color = CGColorCreateSRGB(r, g, b, 1.);
            let _: () = msg_send![&*colors, addObject: &*(color as *const AnyObject)];
            CGColorRelease(color);
        }
    }
    set_ref(&wheel, "colors", Retained::as_ptr(&colors) as *const c_void);
    set_text(&wheel, "type", "conic");
    unsafe {
        let _: () = msg_send![&*wheel, setFrame: NSRect::new(NSPoint::new(0., 0.), NSSize::new(side, side))];
        let _: () = msg_send![&*wheel, setStartPoint: NSPoint::new(0.5, 0.5)];
        let _: () = msg_send![&*wheel, setEndPoint: NSPoint::new(1., 0.5)];
    }
    wheel
}

/// An animation of `key` on `target` from `from` to `to`, begun at `begin`
/// (media time), over `seconds`, at `pace`.
fn animate(target: &AnyObject, key: &str, name: &str, (from, to): (f64, f64), begin: f64, seconds: f64, pace: Pace) {
    let class = AnyClass::get(c"CABasicAnimation").expect("QuartzCore");
    unsafe {
        let path = NSString::from_str(key);
        let a: Retained<AnyObject> = msg_send![class, animationWithKeyPath: &*path];
        let (from, to) = (NSNumber::new_f64(from), NSNumber::new_f64(to));
        let _: () = msg_send![&*a, setFromValue: &*from];
        let _: () = msg_send![&*a, setToValue: &*to];
        let _: () = msg_send![&*a, setBeginTime: begin];
        let _: () = msg_send![&*a, setDuration: seconds];
        if pace == Pace::In {
            let curve = AnyClass::get(c"CAMediaTimingFunction").expect("QuartzCore");
            let name = NSString::from_str("easeIn");
            let ease_in: Retained<AnyObject> = msg_send![curve, functionWithName: &*name];
            let _: () = msg_send![&*a, setTimingFunction: &*ease_in];
            let forwards = NSString::from_str("forwards");
            let _: () = msg_send![&*a, setFillMode: &*forwards];
        } else {
            let _: () = msg_send![&*a, setRepeatCount: f32::INFINITY];
        }
        if pace == Pace::Drift {
            let _: () = msg_send![&*a, setPreferredFrameRateRange: FrameRate { minimum: 10., maximum: 15., preferred: 15. }];
        }
        let _: () = msg_send![&*a, setRemovedOnCompletion: false];
        let name = NSString::from_str(name);
        let _: () = msg_send![target, addAnimation: &*a, forKey: &*name];
    }
}

fn screen_under_pointer(mtm: MainThreadMarker) -> Option<Retained<NSScreen>> {
    use objc2_app_kit::NSEvent;
    let at = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);
    let found = screens.iter().find(|s| {
        let f = s.frame();
        at.x >= f.origin.x && at.x < f.origin.x + f.size.width && at.y >= f.origin.y && at.y < f.origin.y + f.size.height
    });
    found.or_else(|| screens.firstObject())
}

/// The marks come out of the corners.
pub fn show() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let mut one = GLOW.lock().unwrap();
    if one.0.is_some() {
        return;
    }
    let Some(screen) = screen_under_pointer(mtm) else { return };
    let frame = screen.frame();
    let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
    let window = NSPanel::initWithContentRect_styleMask_backing_defer(mtm.alloc(), frame, style, NSBackingStoreType::Buffered, false);
    unsafe { window.setReleasedWhenClosed(false) };
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setHasShadow(false);
    window.setIgnoresMouseEvents(true);
    window.setHidesOnDeactivate(false);
    // Over the menu bar and the Dock, under the pill.
    window.setLevel(25);
    window.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Stationary | NSWindowCollectionBehavior::IgnoresCycle | NSWindowCollectionBehavior::FullScreenAuxiliary);
    let Some(view) = window.contentView() else { return };
    let root = layer("CALayer");
    view.setLayer(Some(unsafe { &*(Retained::as_ptr(&root) as *const objc2_quartz_core::CALayer) }));
    view.setWantsLayer(true);
    let size = frame.size;
    let outline = Outline::new(size);
    let (ink, lightness) = inks();
    let add = |to: &AnyObject, sub: &AnyObject| unsafe {
        let _: () = msg_send![to, addSublayer: sub];
    };
    // The band: the wheel of colors, in a layer that turns it while it
    // works, seen through the band's own layers (`stencil`).
    let (band, stencil, spin) = (layer("CALayer"), layer("CALayer"), layer("CALayer"));
    let side = (size.width.powi(2) + size.height.powi(2)).sqrt();
    unsafe {
        let whole = NSRect::new(NSPoint::new(0., 0.), size);
        let _: () = msg_send![&*band, setFrame: whole];
        let _: () = msg_send![&*stencil, setFrame: whole];
        let _: () = msg_send![&*spin, setFrame: NSRect::new(NSPoint::new((size.width - side) / 2., (size.height - side) / 2.), NSSize::new(side, side))];
        let _: () = msg_send![&*band, setOpacity: BAND_OPACITY];
        let _: () = msg_send![&*band, setMask: &*stencil];
    }
    let wheel = wheel(size, lightness);
    add(&spin, &wheel);
    add(&band, &spin);
    add(&root, &band);
    let mut marks = Vec::new();
    for k in BAND_REACH {
        for &corner in &outline.corners {
            let m = mark(&outline, corner, size, BAND, (1., 1., 1., 1.));
            set_key(&m, "opacity", BAND_STEP);
            add(&stencil, &m);
            marks.push((m, k));
        }
    }
    for &corner in &outline.corners {
        let m = mark(&outline, corner, size, STROKE, ink);
        add(&root, &m);
        marks.push((m, 1.));
    }
    // The colors turn, clockwise as seen, the way the marks slide.
    if !calm() {
        let turn = (0., -std::f64::consts::TAU);
        animate(&wheel, "transform.rotation.z", "turn", turn, unsafe { CACurrentMediaTime() }, 360. / HUE_TURN, Pace::Drift);
    }
    let glow = Glow { window, marks, spin, round: outline.round(), sliding: false };
    together(0., || reach(&glow, 0.));
    glow.window.orderFrontRegardless();
    together(IN_S, || reach(&glow, ARM));
    one.0 = Some(glow);
}

/// The voice's loudness, 0 to 1: the marks reach further with it.
pub fn level(loud: f32) {
    let one = GLOW.lock().unwrap();
    let Some(glow) = &one.0 else { return };
    if glow.sliding || calm() {
        return;
    }
    let arm = ARM + REACH * (loud.clamp(0., 1.) as f64).sqrt();
    together(0.14, || reach(glow, arm));
}

/// The marks settle to their working length and slide round together,
/// coming up to speed, while the steps are worked out and carried out.
pub fn thinking() {
    let mut one = GLOW.lock().unwrap();
    let Some(glow) = &mut one.0 else { return };
    if glow.sliding {
        return;
    }
    together(0.3, || reach(glow, ARM_WORK));
    if calm() {
        return;
    }
    glow.sliding = true;
    // A round is half of each path. The first stretch eases in: the
    // system's ease-in ends 1.72 times as fast as its average, so it
    // covers what the slide's speed over `SPEED_UP` would, over 1.72.
    let ease = SLIDE * SPEED_UP / 1.724 / (2. * glow.round);
    let round_s = glow.round / SLIDE;
    let now = unsafe { CACurrentMediaTime() } + 0.3;
    for (m, k) in &glow.marks {
        let half = ARM_WORK * k / (2. * glow.round);
        for (key, at) in [("strokeStart", 0.25 - half), ("strokeEnd", 0.25 + half)] {
            animate(m, key, &format!("{key}.in"), (at, at + ease), now, SPEED_UP, Pace::In);
            // Then round and round: a round on is the same place, so
            // starting again is not seen.
            animate(m, key, &format!("{key}.turn"), (at + ease, at + ease + 0.5), now + SPEED_UP, round_s, Pace::Round);
        }
    }
    // The colors turn faster, coming up to speed with the marks.
    let (fast, tau) = (HUE_WORK.to_radians(), std::f64::consts::TAU);
    let first = fast * SPEED_UP / 1.724;
    animate(&glow.spin, "transform.rotation.z", "in", (0., -first), now, SPEED_UP, Pace::In);
    animate(&glow.spin, "transform.rotation.z", "turn", (-first, -first - tau), now + SPEED_UP, tau / fast, Pace::Round);
}

/// Each mark draws back to its middle, wherever it is, and the window goes
/// with them.
pub fn hide() {
    let Some(glow) = GLOW.lock().unwrap().0.take() else { return };
    // Held where the slide has them, then let go of the slide.
    let ends: Vec<(f64, f64)> = glow
        .marks
        .iter()
        .map(|(m, _)| unsafe {
            let seen: Option<Retained<AnyObject>> = msg_send![&**m, presentationLayer];
            let seen = seen.unwrap_or_else(|| m.clone());
            let (a, b): (f64, f64) = (msg_send![&*seen, strokeStart], msg_send![&*seen, strokeEnd]);
            (a, b)
        })
        .collect();
    together(0., || {
        for ((m, _), &(a, b)) in glow.marks.iter().zip(&ends) {
            unsafe {
                let _: () = msg_send![&**m, removeAllAnimations];
            }
            set_key(m, "strokeStart", a);
            set_key(m, "strokeEnd", b);
        }
    });
    together(OUT_S, || {
        for ((m, _), &(a, b)) in glow.marks.iter().zip(&ends) {
            set_key(m, "strokeStart", (a + b) / 2.);
            set_key(m, "strokeEnd", (a + b) / 2.);
        }
    });
    let window = glow.window;
    // Gone from the screen at the motion's end.
    let block = block2::RcBlock::new(move |_: std::ptr::NonNull<objc2_foundation::NSTimer>| window.orderOut(None));
    unsafe {
        let timer = objc2_foundation::NSTimer::timerWithTimeInterval_repeats_block(OUT_S + 0.05, false, &block);
        objc2_foundation::NSRunLoop::mainRunLoop().addTimer_forMode(&timer, objc2_foundation::NSRunLoopCommonModes);
    }
}

/// The window's number, for a picture of it (`sys::shoot_number`).
pub fn number() -> Option<isize> {
    GLOW.lock().unwrap().0.as_ref().map(|g| g.window.windowNumber())
}
