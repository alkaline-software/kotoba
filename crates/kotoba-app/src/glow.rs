//! The light around the screen while Kotoba is being told what to do, as
//! a phone's assistant has it: colors flowing round the edge, swelling with
//! the voice. It is Core Animation's, not the toolkit's: a window over
//! everything that takes no click, with two rings of color turning under
//! soft masks, drawn by the system at the screen's own rate and costing
//! this app no frames, while the model has the GPU.
//!
//! Called on the main thread alone, as AppKit asks.

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{msg_send, MainThreadMarker};
use objc2_app_kit::{NSBackingStoreType, NSColor, NSPanel, NSScreen, NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace};
use objc2_foundation::{NSMutableArray, NSNumber, NSPoint, NSRect, NSSize, NSString};

/// The colors round the ring, the last the first again: an even walk
/// through lilac, pink, coral, peach and periwinkle.
const COLORS: [(f64, f64, f64); 7] = [(0.74, 0.51, 0.95), (0.96, 0.73, 0.92), (1.00, 0.40, 0.47), (1.00, 0.73, 0.44), (0.78, 0.53, 1.00), (0.55, 0.62, 1.00), (0.74, 0.51, 0.95)];

/// How deep the wide light reaches in from the edge, and the bright line.
const WIDE: f64 = 170.;
const LINE: f64 = 16.;
/// The mask's scale of the screen: a soft shape loses nothing at a third
/// of the size.
const SHRINK: f64 = 3.;
/// One turn of each ring, in seconds; the two go at different speeds so
/// their colors never settle.
const TURN_WIDE: f64 = 7.;
const TURN_LINE: f64 = 4.5;
/// How much faster the rings turn while the words are at work.
const THINKING: f32 = 3.;
const IN_S: f64 = 0.55;
const OUT_S: f64 = 0.4;
/// How far the light stands out past the edge when quiet: the voice brings
/// it in.
const QUIET: f64 = 1.018;
const LOUD: f64 = 0.985;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorCreateSRGB(r: f64, g: f64, b: f64, a: f64) -> *mut c_void;
    fn CGColorRelease(color: *mut c_void);
    fn CGBitmapContextCreate(data: *mut c_void, w: usize, h: usize, bits: usize, row: usize, space: *mut c_void, info: u32) -> *mut c_void;
    fn CGBitmapContextCreateImage(context: *mut c_void) -> *mut c_void;
    fn CGContextRelease(context: *mut c_void);
    fn CGImageRelease(image: *mut c_void);
}

#[link(name = "QuartzCore", kind = "framework")]
extern "C" {
    fn CACurrentMediaTime() -> f64;
}

struct Glow {
    window: Retained<NSPanel>,
    wide: Retained<AnyObject>,
    rings: [Retained<AnyObject>; 2],
}

/// The one light, on the main thread alone.
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

fn frame_of(target: &AnyObject, frame: NSRect) {
    unsafe {
        let _: () = msg_send![target, setFrame: frame];
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

/// A ring of color, turning, as wide as the screen's diagonal so its
/// corners stay past the screen's as it turns.
fn ring(size: NSSize, turn: f64, reversed: bool, still: bool) -> Retained<AnyObject> {
    let side = (size.width * size.width + size.height * size.height).sqrt() * 1.05;
    let ring = layer("CAGradientLayer");
    let colors = NSMutableArray::<AnyObject>::new();
    for (r, g, b) in COLORS {
        unsafe {
            let color = CGColorCreateSRGB(r, g, b, 1.);
            colors.addObject(&*(color as *mut AnyObject));
            CGColorRelease(color);
        }
    }
    frame_of(&ring, NSRect::new(NSPoint::new((size.width - side) / 2., (size.height - side) / 2.), NSSize::new(side, side)));
    unsafe {
        let conic = NSString::from_str("conic");
        let _: () = msg_send![&*ring, setType: &*conic];
        let _: () = msg_send![&*ring, setColors: &*colors];
        let _: () = msg_send![&*ring, setStartPoint: NSPoint::new(0.5, 0.5)];
        let _: () = msg_send![&*ring, setEndPoint: NSPoint::new(0.5, 0.)];
    }
    if still {
        return ring;
    }
    let class = AnyClass::get(c"CABasicAnimation").expect("QuartzCore");
    unsafe {
        let path = NSString::from_str("transform.rotation.z");
        let spin: Retained<AnyObject> = msg_send![class, animationWithKeyPath: &*path];
        let (from, to) = (NSNumber::new_f64(0.), NSNumber::new_f64(if reversed { -std::f64::consts::TAU } else { std::f64::consts::TAU }));
        let _: () = msg_send![&*spin, setFromValue: &*from];
        let _: () = msg_send![&*spin, setToValue: &*to];
        let _: () = msg_send![&*spin, setDuration: turn];
        let _: () = msg_send![&*spin, setRepeatCount: f32::INFINITY];
        let _: () = msg_send![&*spin, setRemovedOnCompletion: false];
        let key = NSString::from_str("turn");
        let _: () = msg_send![&*ring, addAnimation: &*spin, forKey: &*key];
    }
    ring
}

/// How far a point is inside the screen, in points: the nearest edge, the
/// four blended, so lines of one depth curve round the corners. A rounded
/// box's distance keeps a crease from each corner toward the middle.
fn inside(x: f64, y: f64, w: f64, h: f64, soft: f64) -> f64 {
    let near = [x, w - x, y, h - y].iter().map(|d| (-d / soft).exp()).sum::<f64>();
    -soft * near.ln()
}

/// A mask of the light's depth: full at the edge, nothing `depth` in, the
/// fall of it `curve`. An image of alpha alone, a third of the screen's
/// size, which the system stretches.
fn mask(size: NSSize, depth: f64, curve: f64) -> Retained<AnyObject> {
    let (w, h) = ((size.width / SHRINK).ceil() as usize, (size.height / SHRINK).ceil() as usize);
    let mut alpha = vec![0u8; w * h];
    for (row, line) in alpha.chunks_mut(w).enumerate() {
        for (col, px) in line.iter_mut().enumerate() {
            let d = inside((col as f64 + 0.5) * SHRINK, (row as f64 + 0.5) * SHRINK, size.width, size.height, depth / 4.).max(0.);
            *px = ((1. - (d / depth).min(1.)).powf(curve) * 255.) as u8;
        }
    }
    let shape = layer("CALayer");
    frame_of(&shape, NSRect::new(NSPoint::new(0., 0.), size));
    unsafe {
        // `kCGImageAlphaOnly`: eight bits a pixel, no color space.
        let context = CGBitmapContextCreate(alpha.as_mut_ptr() as *mut c_void, w, h, 8, w, std::ptr::null_mut(), 7);
        let image = CGBitmapContextCreateImage(context);
        let _: () = msg_send![&*shape, setContents: image as *mut AnyObject];
        CGImageRelease(image);
        CGContextRelease(context);
    }
    shape
}

fn band(size: NSSize, depth: f64, curve: f64, turn: f64, reversed: bool, still: bool) -> (Retained<AnyObject>, Retained<AnyObject>) {
    let holder = layer("CALayer");
    frame_of(&holder, NSRect::new(NSPoint::new(0., 0.), size));
    let ring = ring(size, turn, reversed, still);
    let shape = mask(size, depth, curve);
    unsafe {
        let _: () = msg_send![&*holder, addSublayer: &*ring];
        let _: () = msg_send![&*holder, setMask: &*shape];
    }
    (holder, ring)
}

fn calm() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
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

/// The light comes in from past the screen's edge.
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
    let still = calm();
    let size = frame.size;
    let (wide, spin_wide) = band(size, WIDE, 1.9, TURN_WIDE, false, still);
    let (line, spin_line) = band(size, LINE, 1.2, TURN_LINE, true, still);
    unsafe {
        let _: () = msg_send![&*root, addSublayer: &*wide];
        let _: () = msg_send![&*root, addSublayer: &*line];
    }
    together(0., || {
        set_key(&root, "opacity", 0.);
        set_key(&wide, "transform.scale", 1.14);
    });
    window.orderFrontRegardless();
    together(IN_S, || {
        set_key(&root, "opacity", 1.);
        set_key(&wide, "transform.scale", QUIET);
    });
    one.0 = Some(Glow { window, wide, rings: [spin_wide, spin_line] });
}

/// The voice's loudness, 0 to 1: the light swells in with it.
pub fn level(loud: f32) {
    let one = GLOW.lock().unwrap();
    let Some(glow) = &one.0 else { return };
    if calm() {
        return;
    }
    let scale = QUIET + (LOUD - QUIET) * (loud.clamp(0., 1.) as f64).sqrt();
    together(0.14, || set_key(&glow.wide, "transform.scale", scale));
}

/// The rings turn faster, from where they are, while the steps are worked
/// out and carried out, and the light rests at its full depth.
pub fn thinking() {
    let one = GLOW.lock().unwrap();
    let Some(glow) = &one.0 else { return };
    let quick = !calm();
    together(0.3, || set_key(&glow.wide, "transform.scale", 1.));
    if !quick {
        return;
    }
    for ring in &glow.rings {
        unsafe {
            let now = CACurrentMediaTime();
            let there: f64 = msg_send![&**ring, convertTime: now, fromLayer: std::ptr::null::<AnyObject>()];
            let _: () = msg_send![&**ring, setTimeOffset: there];
            let _: () = msg_send![&**ring, setBeginTime: now];
            let _: () = msg_send![&**ring, setSpeed: THINKING];
        }
    }
}

/// The light goes back out past the edge, and its window with it.
pub fn hide() {
    let Some(glow) = GLOW.lock().unwrap().0.take() else { return };
    if let Some(root) = glow.window.contentView().and_then(|v| v.layer()) {
        let root: &AnyObject = unsafe { &*(Retained::as_ptr(&root) as *const AnyObject) };
        together(OUT_S, || {
            set_key(root, "opacity", 0.);
            set_key(&glow.wide, "transform.scale", 1.14);
        });
    }
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
