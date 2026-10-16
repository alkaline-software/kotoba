//! What Kotoba asks of the operating system: the menu bar item, the keys
//! pressed in other apps and the paste into them, the microphone's and
//! Accessibility's permissions, the chimes, and the pill's window. A
//! Mac's alone for now, as dictation is; elsewhere each has a stated
//! fallback so the window still builds.

use std::path::Path;

/// Open `path` with the application the system associates with it.
pub fn open_path(path: &Path) {
    let _ = opener::open(path);
}

/// The window's own view, which gpui draws in and takes its keys by.
#[cfg(target_os = "macos")]
fn ns_view(window: &gpui::Window) -> Option<&objc2_app_kit::NSView> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else { return None };
    unsafe { (handle.ns_view.as_ptr() as *const objc2_app_kit::NSView).as_ref() }
}

/// What the keyboard and the pointer do in this app, for dictation's key
/// when it is modifiers tapped alone (`dictate::TapWatch`): the Mac's
/// modifier flags when the keys held change, `None` for a key pressed or
/// a click. gpui says which modifiers are held but not on which side, so
/// this is AppKit's own monitor, which sees every event the app is sent
/// and lets it through as it was. Only while the app is the active one,
/// as any other key of the window's.
#[cfg(target_os = "macos")]
pub fn key_events() -> futures::channel::mpsc::UnboundedReceiver<Option<u64>> {
    use objc2_app_kit::{NSEvent, NSEventMask, NSEventType};
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let mask = NSEventMask::FlagsChanged | NSEventMask::KeyDown | NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown | NSEventMask::OtherMouseDown;
    let handler = block2::RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| -> *mut NSEvent {
        let e = unsafe { event.as_ref() };
        let said = (e.r#type() == NSEventType::FlagsChanged).then(|| e.modifierFlags().0 as u64);
        let _ = tx.unbounded_send(said);
        event.as_ptr()
    });
    // Kept for the app's life: AppKit holds the handler until the
    // monitor is removed, and it never is.
    std::mem::forget(unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &handler) });
    rx
}

/// What the Mac does with Fn, the globe key, pressed alone: Keyboard
/// settings, "Press 🌐 key to" (`AppleFnUsageType` in HIToolbox's
/// domain). `None` when it does nothing; otherwise what it does, by a
/// word the window puts in a sentence of its language (`input`, `emoji`,
/// `dictation`), empty when the value is not one known here. A Mac that never had it set has a job for the key, so no
/// value is a job too. Asks `defaults`, so off the main thread.
#[cfg(target_os = "macos")]
pub fn fn_key_use() -> Option<&'static str> {
    let probe = FN_USE_PROBE.load(std::sync::atomic::Ordering::Relaxed);
    let value = if probe >= 0 {
        probe.to_string()
    } else {
        let out = std::process::Command::new("defaults").args(["read", "com.apple.HIToolbox", "AppleFnUsageType"]).output().ok()?;
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    match value.as_str() {
        "0" => None,
        "1" => Some("input"),
        "2" => Some("emoji"),
        "3" => Some("dictation"),
        _ => Some(""),
    }
}
/// Elsewhere the key has no such setting.
#[cfg(not(target_os = "macos"))]
pub fn fn_key_use() -> Option<&'static str> {
    None
}

/// `KOTOBA_GO=fnuse:<n>`: the value `fn_key_use` reads in place of the
/// Mac's, so the guidance can be seen without touching Keyboard settings.
pub static FN_USE_PROBE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

/// Whether F1 to F12 on an Apple keyboard are standard function keys
/// ("Use F1, F2, etc. keys as standard function keys", in Keyboard
/// Shortcuts, Function Keys). Off, which a Mac that never had it set
/// is, each does what is printed on it, and is an F key only with Fn
/// held. Asks `defaults`, so off the main thread.
#[cfg(target_os = "macos")]
pub fn f_keys_standard() -> bool {
    let probe = F_STATE_PROBE.load(std::sync::atomic::Ordering::Relaxed);
    if probe >= 0 {
        return probe == 1;
    }
    std::process::Command::new("defaults").args(["read", "-g", "com.apple.keyboard.fnState"]).output().is_ok_and(|out| String::from_utf8_lossy(&out.stdout).trim() == "1")
}
/// Elsewhere F keys are F keys.
#[cfg(not(target_os = "macos"))]
pub fn f_keys_standard() -> bool {
    true
}

/// `KOTOBA_GO=fstate:<0|1>`: the value `f_keys_standard` reads in place
/// of the Mac's.
pub static F_STATE_PROBE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

/// Every other Kotoba running as an app, the release's or the build's,
/// asked to quit, and made to if it is still there a few seconds on: two
/// would each take dictation's key and write everything twice. The one
/// opened last stays.
#[cfg(target_os = "macos")]
pub fn end_other_kotobas() {
    use objc2_app_kit::NSRunningApplication;
    use objc2_foundation::NSString;
    let me = std::process::id() as i32;
    let mut others = Vec::new();
    for id in ["com.pingfanhu.kotoba", "com.pingfanhu.kotoba.dev"] {
        for app in NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(id)).iter() {
            let pid = app.processIdentifier();
            if pid != me && pid > 0 {
                // Its quit lets its models go (`on_app_quit`).
                app.terminate();
                others.push(pid);
            }
        }
    }
    if others.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(3));
        for pid in others {
            if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
                if !app.isTerminated() {
                    app.forceTerminate();
                }
            }
        }
    });
}
#[cfg(not(target_os = "macos"))]
pub fn end_other_kotobas() {}

/// Keyboard settings in System Settings, where Fn's use is set.
pub const KEYBOARD_SETTINGS_URL: &str = "x-apple.systempreferences:com.apple.Keyboard-Settings.extension";

/// A sound of the app's own (a WAV file's bytes) played once, quietly:
/// dictation's start and stop. The last one played is held until the
/// next, or the system would let it go before it had sounded.
#[cfg(target_os = "macos")]
pub fn play_sound(wav: &'static [u8]) {
    use objc2::AllocAnyThread as _;
    use objc2_app_kit::NSSound;
    use objc2_foundation::NSData;
    thread_local! {
        static PLAYING: std::cell::RefCell<Option<objc2::rc::Retained<NSSound>>> = const { std::cell::RefCell::new(None) };
    }
    let data = NSData::with_bytes(wav);
    if let Some(sound) = NSSound::initWithData(NSSound::alloc(), &data) {
        // At 0.5 the chimes stood out over what else was playing; 0.35 and
        // 0.28 were still more than wanted.
        sound.setVolume(0.2);
        sound.play();
        PLAYING.with(|p| *p.borrow_mut() = Some(sound));
    }
}
#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
pub fn play_sound(_wav: &'static [u8]) {}

/// A picture of the app's own window, written to `path` as a PNG, for
/// looking at a state from a script (`KOTOBA_SHOT`). A process may capture
/// its own windows without Screen Recording access, which the terminal
/// that runs a probe usually lacks. `CGWindowListCreateImage` is looked
/// up by name: newer SDKs mark it unavailable in favour of
/// ScreenCaptureKit, which wants that access, while the function itself
/// is still there and still captures a window of the caller's own.
#[cfg(target_os = "macos")]
pub fn shoot_window(window: &gpui::Window, path: &std::path::Path) -> Result<(), String> {
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).map_err(|e| e.to_string())?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else { return Err("not an AppKit window".into()) };
    let view = unsafe { (handle.ns_view.as_ptr() as *const NSView).as_ref() }.ok_or("no view")?;
    let number = view.window().ok_or("the view has no window")?.windowNumber();
    shoot_number(number, path)
}

/// A picture of one of the app's own windows by its number (`shoot_window`).
#[cfg(target_os = "macos")]
fn shoot_number(number: isize, path: &std::path::Path) -> Result<(), String> {
    use std::ffi::{c_char, c_void, CString};
    use std::os::unix::ffi::OsStrExt;

    #[repr(C)]
    struct Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }
    type Capture = unsafe extern "C" fn(Rect, u32, u32, u32) -> *mut c_void;
    #[link(name = "ImageIO", kind = "framework")]
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn CFURLCreateFromFileSystemRepresentation(alloc: *const c_void, path: *const u8, len: isize, is_dir: u8) -> *mut c_void;
        fn CFStringCreateWithCString(alloc: *const c_void, text: *const c_char, encoding: u32) -> *mut c_void;
        fn CGImageDestinationCreateWithURL(url: *mut c_void, kind: *mut c_void, count: usize, options: *const c_void) -> *mut c_void;
        fn CGImageDestinationAddImage(dest: *mut c_void, image: *mut c_void, properties: *const c_void);
        fn CGImageDestinationFinalize(dest: *mut c_void) -> u8;
        fn CFRelease(obj: *mut c_void);
    }

    let bytes = path.as_os_str().as_bytes();
    unsafe {
        // RTLD_DEFAULT, then: no rectangle (the window's own bounds),
        // only this window, without its shadow.
        let capture = dlsym(-2isize as *mut c_void, c"CGWindowListCreateImage".as_ptr());
        if capture.is_null() {
            return Err("this system has no CGWindowListCreateImage".into());
        }
        let capture: Capture = std::mem::transmute(capture);
        let null = Rect { x: f64::INFINITY, y: f64::INFINITY, w: 0.0, h: 0.0 };
        // `KOTOBA_SHOT_FRAME=1` keeps what the system draws around the
        // window, its shadow and its rim.
        let framing = if std::env::var_os("KOTOBA_SHOT_FRAME").is_some() { 0 } else { 1 };
        let image = capture(null, 1 << 3, number as u32, framing);
        if image.is_null() {
            return Err("the window could not be captured".into());
        }
        let url = CFURLCreateFromFileSystemRepresentation(std::ptr::null(), bytes.as_ptr(), bytes.len() as isize, 0);
        let png = CString::new("public.png").unwrap();
        let kind = CFStringCreateWithCString(std::ptr::null(), png.as_ptr(), 0x0800_0100);
        let dest = CGImageDestinationCreateWithURL(url, kind, 1, std::ptr::null());
        let written = if dest.is_null() {
            false
        } else {
            CGImageDestinationAddImage(dest, image, std::ptr::null());
            let ok = CGImageDestinationFinalize(dest) != 0;
            CFRelease(dest);
            ok
        };
        CFRelease(kind);
        CFRelease(url);
        CFRelease(image);
        if written { Ok(()) } else { Err(format!("could not write {}", path.display())) }
    }
}
#[cfg(not(target_os = "macos"))]
pub fn shoot_window(_window: &gpui::Window, _path: &std::path::Path) -> Result<(), String> {
    Err("a picture of the window is only taken on macOS".into())
}

// Dictation in other apps. What is pressed while another app is in front
// is seen through an event tap, and the words go into that app by a paste:
// both need Accessibility, which the person grants in System Settings.


/// What was pressed while another app was in front (`outside_keys`).
#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy)]
pub enum Outside {
    /// The modifiers held changed: the Mac's flags.
    Flags(u64),
    /// Any other key, or a click.
    Other,
    /// Dictation's key, when it is a key and not a tap. Kept from the app.
    Key,
    /// Escape while dictation listens. Kept from the app.
    Escape,
}

#[cfg(target_os = "macos")]
mod outside {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};

    type Ref = *mut c_void;
    type TapCallback = extern "C" fn(proxy: Ref, kind: u32, event: Ref, info: Ref) -> Ref;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: Ref) -> bool;
        static kAXTrustedCheckOptionPrompt: Ref;
        fn CGEventTapCreate(tap: u32, place: u32, options: u32, mask: u64, callback: TapCallback, info: Ref) -> Ref;
        fn CGEventTapEnable(tap: Ref, enable: bool);
        fn CGEventTapIsEnabled(tap: Ref) -> bool;
        fn CGEventGetFlags(event: Ref) -> u64;
        fn CGEventGetIntegerValueField(event: Ref, field: u32) -> i64;
        fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
        fn CGEventSetFlags(event: Ref, flags: u64);
        fn CGEventPost(tap: u32, event: Ref);
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFMachPortCreateRunLoopSource(alloc: Ref, port: Ref, order: isize) -> Ref;
        fn CFRunLoopGetMain() -> Ref;
        fn CFRunLoopAddSource(rl: Ref, source: Ref, mode: Ref);
        fn CFDictionaryCreate(alloc: Ref, keys: *const Ref, values: *const Ref, n: isize, key_cb: Ref, value_cb: Ref) -> Ref;
        fn CFRelease(r: Ref);
        fn CFMachPortIsValid(port: Ref) -> bool;
        static kCFRunLoopCommonModes: Ref;
        static kCFBooleanTrue: Ref;
        static kCFTypeDictionaryKeyCallBacks: c_void;
        static kCFTypeDictionaryValueCallBacks: c_void;
    }

    const SESSION_TAP: u32 = 1;
    const HID_TAP: u32 = 0;
    const KEY_DOWN: u32 = 10;
    const FLAGS_CHANGED: u32 = 12;
    const DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
    const DISABLED_BY_USER: u32 = 0xFFFF_FFFF;
    const KEYCODE_FIELD: u32 = 9;
    const AUTOREPEAT_FIELD: u32 = 8;
    const ESCAPE: i64 = 53;
    /// ⌘, ⇧, ⌃ and ⌥, of the Mac's flags.
    pub const MODS: u64 = 0x10_0000 | 0x2_0000 | 0x4_0000 | 0x8_0000;

    static PORT: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    /// Dictation's key as the key's code and the modifiers with it; 0 for
    /// none (a tap, or nothing bound). The code is in the low 16 bits.
    pub static KEY: AtomicU64 = AtomicU64::new(0);
    pub static ESCAPE_TAKEN: AtomicBool = AtomicBool::new(false);

    pub fn trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    /// Asks the system to list the app in Accessibility, with its own
    /// prompt the first time.
    pub fn ask_trust() -> bool {
        unsafe {
            let keys = [kAXTrustedCheckOptionPrompt];
            let values = [kCFBooleanTrue];
            let options = CFDictionaryCreate(std::ptr::null_mut(), keys.as_ptr(), values.as_ptr(), 1, &kCFTypeDictionaryKeyCallBacks as *const c_void as Ref, &kCFTypeDictionaryValueCallBacks as *const c_void as Ref);
            let ok = AXIsProcessTrustedWithOptions(options);
            CFRelease(options);
            ok
        }
    }

    extern "C" fn seen(_proxy: Ref, kind: u32, event: Ref, info: Ref) -> Ref {
        if kind == DISABLED_BY_TIMEOUT || kind == DISABLED_BY_USER {
            let port = PORT.load(Ordering::Relaxed);
            if !port.is_null() {
                unsafe { CGEventTapEnable(port, true) };
            }
            return event;
        }
        // The app's own keys are its local monitor's (`key_events`).
        if super::app_is_active() {
            return event;
        }
        let tx = unsafe { &*(info as *const futures::channel::mpsc::UnboundedSender<super::Outside>) };
        let said = match kind {
            FLAGS_CHANGED => super::Outside::Flags(unsafe { CGEventGetFlags(event) }),
            KEY_DOWN => {
                let code = unsafe { CGEventGetIntegerValueField(event, KEYCODE_FIELD) };
                let mods = unsafe { CGEventGetFlags(event) } & MODS;
                let key = KEY.load(Ordering::Relaxed);
                if code == ESCAPE && mods == 0 && ESCAPE_TAKEN.load(Ordering::Relaxed) {
                    let _ = tx.unbounded_send(super::Outside::Escape);
                    return std::ptr::null_mut();
                }
                if key != 0 && code as u64 == key & 0xFFFF && mods == key & MODS {
                    if unsafe { CGEventGetIntegerValueField(event, AUTOREPEAT_FIELD) } == 0 {
                        let _ = tx.unbounded_send(super::Outside::Key);
                    }
                    return std::ptr::null_mut();
                }
                super::Outside::Other
            }
            _ => super::Outside::Other,
        };
        let _ = tx.unbounded_send(said);
        event
    }

    /// The tap, on the main run loop, for the app's life. False when the
    /// system refuses it: Accessibility is not granted.
    pub fn start(tx: futures::channel::mpsc::UnboundedSender<super::Outside>) -> bool {
        let mask = [KEY_DOWN, FLAGS_CHANGED, 1, 3, 25].iter().fold(0u64, |m, t| m | 1 << t);
        let info = Box::into_raw(Box::new(tx)) as Ref;
        let port = unsafe { CGEventTapCreate(SESSION_TAP, 0, 0, mask, seen, info) };
        if port.is_null() {
            drop(unsafe { Box::from_raw(info as *mut futures::channel::mpsc::UnboundedSender<super::Outside>) });
            return false;
        }
        PORT.store(port, Ordering::Relaxed);
        unsafe {
            let source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), port, 0);
            CFRunLoopAddSource(CFRunLoopGetMain(), source, kCFRunLoopCommonModes);
            CFRelease(source);
            CGEventTapEnable(port, true);
        }
        true
    }

    /// The tap made earlier, switched on again when the system switched it
    /// off (Accessibility taken away and given back). False when there is
    /// none, or it will not come back on, and a new one is wanted.
    pub fn revive() -> bool {
        let port = PORT.load(Ordering::Relaxed);
        if port.is_null() || !unsafe { CFMachPortIsValid(port) } {
            return false;
        }
        unsafe {
            CGEventTapEnable(port, true);
            CGEventTapIsEnabled(port)
        }
    }

    /// ⌘V, as if pressed.
    pub fn press_paste() {
        const V: u16 = 9;
        const CMD: u64 = 0x10_0000;
        unsafe {
            for down in [true, false] {
                let e = CGEventCreateKeyboardEvent(std::ptr::null_mut(), V, down);
                if e.is_null() {
                    return;
                }
                CGEventSetFlags(e, CMD);
                CGEventPost(HID_TAP, e);
                CFRelease(e);
            }
        }
    }
}

/// Whether this app is the one in front.
#[cfg(target_os = "macos")]
fn app_is_active() -> bool {
    use objc2_app_kit::NSRunningApplication;
    NSRunningApplication::currentApplication().isActive()
}

/// Whether the app may watch keys pressed in other apps and press keys in
/// them: Accessibility, in System Settings.
#[cfg(target_os = "macos")]
pub fn trusted() -> bool {
    outside::trusted()
}

/// A message each time Accessibility's list changes, for any app: the
/// system's own notice, so nothing asks over and over.
#[cfg(target_os = "macos")]
pub fn trust_changes() -> futures::channel::mpsc::UnboundedReceiver<()> {
    use objc2_foundation::{NSDistributedNotificationCenter, NSNotification, NSOperationQueue, NSString};
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let block = block2::RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| {
        let _ = tx.unbounded_send(());
    });
    let center = NSDistributedNotificationCenter::defaultCenter();
    let observer = unsafe { center.addObserverForName_object_queue_usingBlock(Some(&NSString::from_str("com.apple.accessibility.api")), None, Some(&NSOperationQueue::mainQueue()), &block) };
    // Kept for the app's life, as the key monitor is.
    std::mem::forget(observer);
    rx
}

/// The system's own question, which puts the app in Accessibility's
/// list and offers to open it there.
#[cfg(target_os = "macos")]
pub fn ask_trust() -> bool {
    outside::ask_trust()
}
#[cfg(not(target_os = "macos"))]
pub fn ask_trust() -> bool {
    false
}

/// The tap over other apps switched back on, when there is one: true
/// when it is on.
#[cfg(target_os = "macos")]
pub fn revive_outside_keys() -> bool {
    outside::revive()
}

/// What is pressed while another app is in front, for dictation started
/// and stopped there. None until Accessibility is granted; asked again
/// once it is.
#[cfg(target_os = "macos")]
pub fn outside_keys() -> Option<futures::channel::mpsc::UnboundedReceiver<Outside>> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    outside::start(tx).then_some(rx)
}

/// Dictation's key as the tap matches it in other apps: a gpui binding
/// ("cmd-shift-d"), or None for a tap of modifiers or a key that has no
/// code here.
#[cfg(target_os = "macos")]
pub fn set_outside_key(binding: Option<&gpui::Keystroke>) {
    let code = binding.and_then(|k| key_code(&k.key).map(|c| (c, k)));
    let packed = code.map_or(0, |(c, k)| {
        let m = &k.modifiers;
        let mut mods = 0u64;
        if m.platform {
            mods |= 0x10_0000;
        }
        if m.shift {
            mods |= 0x2_0000;
        }
        if m.control {
            mods |= 0x4_0000;
        }
        if m.alt {
            mods |= 0x8_0000;
        }
        mods | c as u64
    });
    outside::KEY.store(packed, std::sync::atomic::Ordering::Relaxed);
}
#[cfg(not(target_os = "macos"))]
pub fn set_outside_key(_binding: Option<&gpui::Keystroke>) {}

/// Escape in another app cancels dictation, and is kept from that app,
/// while this is on.
#[cfg(target_os = "macos")]
pub fn take_escape(on: bool) {
    outside::ESCAPE_TAKEN.store(on, std::sync::atomic::Ordering::Relaxed);
}
#[cfg(not(target_os = "macos"))]
pub fn take_escape(_on: bool) {}

/// The Mac's code for a key as gpui names it, on the US layout, as the
/// system's own shortcuts are.
#[cfg(target_os = "macos")]
fn key_code(key: &str) -> Option<u16> {
    const LETTERS: [u16; 26] = [0, 11, 8, 2, 14, 3, 5, 4, 34, 38, 40, 37, 46, 45, 31, 35, 12, 15, 1, 17, 32, 9, 13, 7, 16, 6];
    const DIGITS: [u16; 10] = [29, 18, 19, 20, 21, 23, 22, 26, 28, 25];
    const F: [u16; 20] = [122, 120, 99, 118, 96, 97, 98, 100, 101, 109, 103, 111, 105, 107, 113, 106, 64, 79, 80, 90];
    let mut chars = key.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_ascii_lowercase() {
            return Some(LETTERS[(c as u8 - b'a') as usize]);
        }
        if c.is_ascii_digit() {
            return Some(DIGITS[(c as u8 - b'0') as usize]);
        }
    }
    if let Some(n) = kotoba_core::dictate::f_key(key) {
        return Some(F[n as usize - 1]);
    }
    Some(match key {
        "space" => 49,
        "enter" => 36,
        "tab" => 48,
        "-" => 27,
        "=" => 24,
        "[" => 33,
        "]" => 30,
        "\\" => 42,
        ";" => 41,
        "'" => 39,
        "," => 43,
        "." => 47,
        "/" => 44,
        "`" => 50,
        _ => return None,
    })
}

/// What the general pasteboard holds, every item in every type, to put
/// back after a paste of our own.
#[cfg(target_os = "macos")]
pub struct Clipboard {
    items: Vec<Vec<(String, Vec<u8>)>>,
    count: isize,
}

/// The words pasted into the app in front, by the pasteboard and ⌘V. The
/// pasteboard is marked as passing through, which clipboard managers skip;
/// what it held is returned, to be put back once the app has read it.
#[cfg(target_os = "macos")]
pub fn paste_words(words: &str) -> Clipboard {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::{NSData, NSString};
    let board = NSPasteboard::generalPasteboard();
    let mut items = Vec::new();
    if let Some(held) = board.pasteboardItems() {
        for item in held.iter() {
            let mut kept = Vec::new();
            for kind in item.types().iter() {
                if let Some(data) = item.dataForType(&kind) {
                    kept.push((kind.to_string(), data.to_vec()));
                }
            }
            items.push(kept);
        }
    }
    board.clearContents();
    unsafe {
        board.setString_forType(&NSString::from_str(words), NSPasteboardTypeString);
        board.setData_forType(Some(&NSData::new()), &NSString::from_str("org.nspasteboard.TransientType"));
    }
    let count = board.changeCount();
    outside::press_paste();
    Clipboard { items, count }
}

/// The pasteboard as it was before `paste_words`, unless something else
/// has been copied since.
#[cfg(target_os = "macos")]
pub fn restore_clipboard(was: Clipboard) {
    use objc2::runtime::ProtocolObject;
    use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
    use objc2_foundation::{NSArray, NSData, NSString};
    let board = NSPasteboard::generalPasteboard();
    if board.changeCount() != was.count {
        return;
    }
    board.clearContents();
    if was.items.is_empty() {
        return;
    }
    let objects: Vec<objc2::rc::Retained<ProtocolObject<dyn NSPasteboardWriting>>> = was
        .items
        .iter()
        .map(|kinds| {
            let item = NSPasteboardItem::new();
            for (kind, data) in kinds {
                item.setData_forType(&NSData::with_bytes(data), &NSString::from_str(kind));
            }
            ProtocolObject::from_retained(item)
        })
        .collect();
    board.writeObjects(&NSArray::from_retained_slice(&objects));
}

/// Where the pill that shows dictation in other apps goes: the screen
/// under the pointer, by its display's id and width, and its notch when
/// it has one, as the width of the menu bar on either side of it and the
/// notch's height.
#[cfg(target_os = "macos")]
pub struct PillPlace {
    pub display: u32,
    pub width: f32,
    pub notch: Option<(f32, f32, f32)>,
}

#[cfg(target_os = "macos")]
pub fn pill_place() -> Option<PillPlace> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSScreen};
    use objc2_foundation::{NSNumber, NSString};
    let mtm = MainThreadMarker::new()?;
    let at = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);
    let screen = screens
        .iter()
        .find(|s| {
            let f = s.frame();
            at.x >= f.origin.x && at.x < f.origin.x + f.size.width && at.y >= f.origin.y && at.y < f.origin.y + f.size.height
        })
        .or_else(|| screens.firstObject())?;
    let id = screen.deviceDescription().objectForKey(&NSString::from_str("NSScreenNumber"))?;
    let id = id.downcast_ref::<NSNumber>()?.unsignedIntValue();
    let width = screen.frame().size.width as f32;
    let top = screen.safeAreaInsets().top as f32;
    let notch = (top > 0.).then(|| (screen.auxiliaryTopLeftArea().size.width as f32, screen.auxiliaryTopRightArea().size.width as f32, top));
    Some(PillPlace { display: id, width, notch })
}

/// The pill's window made to stay over every app and every space, never
/// in the way of the pointer, and drawn with no shadow of the system's.
/// It has no title bar, so the system lets it stand over the menu bar,
/// against the screen's top edge.
#[cfg(target_os = "macos")]
pub fn float_pill(window: &gpui::Window) {
    use objc2_app_kit::NSWindowStyleMask;
    let Some(view) = ns_view(window) else { return };
    let Some(native) = view.window() else { return };
    native.setIgnoresMouseEvents(true);
    native.setHidesOnDeactivate(false);
    native.setHasShadow(false);
    native.setStyleMask(NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel);
    if let Some(screen) = native.screen() {
        let mut frame = native.frame();
        let top = screen.frame();
        frame.origin.y = top.origin.y + top.size.height - frame.size.height;
        native.setFrame_display(frame, true);
    }
}

/// The window kept over every other app's, for a probe.
#[cfg(target_os = "macos")]
pub fn keep_on_top(window: &gpui::Window) {
    if let Some(native) = ns_view(window).and_then(|v| v.window()) {
        native.setLevel(objc2_app_kit::NSFloatingWindowLevel);
    }
}
#[cfg(not(target_os = "macos"))]
pub fn keep_on_top(_window: &gpui::Window) {}

#[cfg(not(target_os = "macos"))]
pub fn trusted() -> bool {
    false
}

// Kotoba is a menu bar app: no Dock icon, one item at the menu bar's
// right, and the settings window opened from it. One click on the item
// opens the settings, the other shows its menu.

/// One choice in the menu bar item's menu: what it is called back by,
/// its name, its icon (an SVG drawn as the window draws it), the key
/// shown beside it, and whether a line comes before it.
pub struct MenuLine {
    pub key: &'static str,
    pub title: &'static str,
    pub icon: &'static [u8],
    pub keys: &'static str,
    pub gap: bool,
}

/// What the menu bar item was asked for: the settings, by the click that
/// opens them, or a choice from its menu, by its key.
#[derive(Clone, Copy, Debug)]
pub enum MenuPick {
    Settings,
    Line(&'static str),
}

#[cfg(target_os = "macos")]
mod menu_bar {
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::OnceLock;

    use futures::channel::mpsc::UnboundedSender;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
    use objc2::{define_class, msg_send, sel, AllocAnyThread as _, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSApplication, NSBitmapImageRep, NSEventMask, NSEventModifierFlags, NSEventType, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
    use objc2_foundation::{NSData, NSSize, NSString};

    use super::{MenuLine, MenuPick};

    static PICKS: OnceLock<UnboundedSender<MenuPick>> = OnceLock::new();
    /// The menu's choices by their tags.
    static KEYS: OnceLock<Vec<&'static str>> = OnceLock::new();
    /// A left click shows the menu, and a right click opens the settings.
    static MENU_ON_LEFT: AtomicBool = AtomicBool::new(false);

    thread_local! {
        /// The item and its menu, for the app's life: the item goes from
        /// the bar when it is let go.
        static ITEM: RefCell<Option<(Retained<NSStatusItem>, Retained<NSMenu>)>> = const { RefCell::new(None) };
        /// Where the menu's picture goes, for a probe (`shoot_menu`).
        static SHOT: RefCell<Option<std::path::PathBuf>> = const { RefCell::new(None) };
        /// What the item and the menu call, which they hold weakly.
        static TARGET: RefCell<Option<Retained<Target>>> = const { RefCell::new(None) };
    }

    fn send(pick: MenuPick) {
        if let Some(tx) = PICKS.get() {
            let _ = tx.unbounded_send(pick);
        }
    }

    define_class!(
        // The item and the menu's choices call here, on the main thread,
        // and what was asked goes on to the app as a message.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "KotobaMenuTarget"]
        struct Target;

        unsafe impl NSObjectProtocol for Target {}

        impl Target {
            /// A click on the item, either button. A left click with
            /// Control held is a right one, as everywhere on a Mac.
            #[unsafe(method(clicked:))]
            fn clicked(&self, _sender: Option<&AnyObject>) {
                let mtm = self.mtm();
                let right = NSApplication::sharedApplication(mtm).currentEvent().is_some_and(|e| e.r#type() == NSEventType::RightMouseUp || e.modifierFlags().contains(NSEventModifierFlags::Control));
                if right == MENU_ON_LEFT.load(Ordering::Relaxed) {
                    send(MenuPick::Settings);
                    return;
                }
                show_menu(mtm);
            }

            /// For a probe: a picture of each window the menu put up,
            /// taken while it is shown, and the menu closed.
            #[unsafe(method(shoot:))]
            fn shoot(&self, _sender: Option<&AnyObject>) {
                let mtm = self.mtm();
                if let Some(path) = SHOT.with(|s| s.borrow_mut().take()) {
                    for (i, w) in NSApplication::sharedApplication(mtm).windows().iter().filter(|w| w.isVisible()).enumerate() {
                        let class = w.class().name().to_string_lossy().into_owned();
                        let named = path.with_file_name(format!("{}-{i}.png", path.file_stem().unwrap_or_default().to_string_lossy()));
                        eprintln!("kotoba: window {i} {class} {:?} {:?}", w.frame(), super::shoot_number(w.windowNumber(), &named));
                    }
                }
                if let Some((_, menu)) = ITEM.with(|i| i.borrow().clone()) {
                    menu.cancelTracking();
                }
            }

            #[unsafe(method(pick:))]
            fn pick(&self, sender: Option<&NSMenuItem>) {
                let key = sender.and_then(|s| KEYS.get()?.get(usize::try_from(s.tag()).ok()?).copied());
                if let Some(key) = key {
                    send(MenuPick::Line(key));
                }
            }
        }
    );

    impl Target {
        fn new(mtm: MainThreadMarker) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(());
            unsafe { msg_send![super(this), init] }
        }
    }

    /// The menu, under the item. It is the item's only while it is shown,
    /// so the click that opens the settings reaches the target; the click
    /// below returns once the menu has closed.
    fn show_menu(mtm: MainThreadMarker) {
        let Some((status, menu)) = ITEM.with(|i| i.borrow().clone()) else { return };
        status.setMenu(Some(&menu));
        if let Some(button) = status.button(mtm) {
            unsafe { button.performClick(None) };
        }
        status.setMenu(None);
    }

    /// An icon for a menu's line from its SVG, at the menu's 16 points,
    /// drawn for one and two pixels a point, as a template the system
    /// tints for the light, the dark and the highlighted line.
    fn line_image(svg: &[u8]) -> Option<Retained<NSImage>> {
        let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default()).ok()?;
        let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(16., 16.));
        for scale in [1u32, 2] {
            let side = 16 * scale;
            let mut pixmap = resvg::tiny_skia::Pixmap::new(side, side)?;
            let k = side as f32 / tree.size().width();
            resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pixmap.as_mut());
            let png = pixmap.encode_png().ok()?;
            let rep = NSBitmapImageRep::imageRepWithData(&NSData::with_bytes(&png))?;
            rep.setSize(NSSize::new(16., 16.));
            image.addRepresentation(&rep);
        }
        image.setTemplate(true);
        Some(image)
    }

    pub fn install(tx: UnboundedSender<MenuPick>, lines: &[MenuLine], menu_on_left: bool, shown: bool) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let _ = PICKS.set(tx);
        let _ = KEYS.set(lines.iter().map(|l| l.key).collect());
        MENU_ON_LEFT.store(menu_on_left, Ordering::Relaxed);
        let target = Target::new(mtm);
        let menu = NSMenu::new(mtm);
        for (i, line) in lines.iter().enumerate() {
            if line.gap {
                menu.addItem(&NSMenuItem::separatorItem(mtm));
            }
            let it = unsafe { NSMenuItem::initWithTitle_action_keyEquivalent(NSMenuItem::alloc(mtm), &NSString::from_str(line.title), Some(sel!(pick:)), &NSString::from_str(line.keys)) };
            unsafe { it.setTarget(Some(&target)) };
            it.setTag(i as isize);
            it.setImage(line_image(line.icon).as_deref());
            // An app built on the macOS 27 SDK has its menus' images
            // hidden unless each item asks for its own
            // (`preferredImageVisibility`, `.visible` = 1); before 27
            // there is no such choice and they are always drawn.
            if it.respondsToSelector(sel!(setPreferredImageVisibility:)) {
                let _: () = unsafe { msg_send![&*it, setPreferredImageVisibility: 1isize] };
            }
            menu.addItem(&it);
        }
        let bar = NSStatusBar::systemStatusBar();
        let status = bar.statusItemWithLength(NSVariableStatusItemLength);
        if let Some(button) = status.button(mtm) {
            let data = NSData::with_bytes(include_bytes!("../assets/icons/menubar.png"));
            if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
                image.setSize(NSSize::new(18., 18.));
                image.setTemplate(true);
                button.setImage(Some(&image));
            }
            button.setToolTip(Some(&NSString::from_str("Kotoba")));
            unsafe {
                button.setTarget(Some(&target));
                button.setAction(Some(sel!(clicked:)));
            }
            button.sendActionOn(NSEventMask::LeftMouseUp | NSEventMask::RightMouseUp);
        }
        status.setVisible(shown);
        ITEM.with(|i| *i.borrow_mut() = Some((status, menu)));
        TARGET.with(|t| *t.borrow_mut() = Some(target));
    }

    /// The item's frame on the screen, from the top left of the main
    /// screen as a click is posted, and whether it is in the bar.
    pub fn frame() -> Option<(f64, f64, f64, f64, bool)> {
        let mtm = MainThreadMarker::new()?;
        let (status, _) = ITEM.with(|i| i.borrow().clone())?;
        let window = status.button(mtm)?.window()?;
        let f = window.frame();
        let top = objc2_app_kit::NSScreen::screens(mtm).firstObject()?.frame().size.height;
        Some((f.origin.x, top - f.origin.y - f.size.height, f.size.width, f.size.height, status.isVisible()))
    }

    /// For a probe: the menu shown, and a moment later, from inside its
    /// tracking, its picture written beside `path` and the menu closed.
    pub fn shoot(path: std::path::PathBuf) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let Some(target) = TARGET.with(|t| t.borrow().clone()) else { return };
        SHOT.with(|s| *s.borrow_mut() = Some(path));
        let modes = objc2_foundation::NSArray::from_slice(&[unsafe { objc2_foundation::NSRunLoopCommonModes }]);
        let _: () = unsafe { msg_send![&*target, performSelector: sel!(shoot:), withObject: None::<&AnyObject>, afterDelay: 0.8f64, inModes: &*modes] };
        show_menu(mtm);
    }

    /// Each choice of the menu named again, by its tag: the language
    /// changed.
    pub fn retitle(lines: &[MenuLine]) {
        ITEM.with(|i| {
            if let Some((_, menu)) = i.borrow().as_ref() {
                for (tag, line) in lines.iter().enumerate() {
                    if let Some(item) = menu.itemWithTag(tag as isize) {
                        item.setTitle(&NSString::from_str(line.title));
                    }
                }
            }
        });
    }

    pub fn set(menu_on_left: bool, shown: bool) {
        MENU_ON_LEFT.store(menu_on_left, Ordering::Relaxed);
        ITEM.with(|i| {
            if let Some((status, _)) = i.borrow().as_ref() {
                status.setVisible(shown);
            }
        });
    }
}

/// The menu bar item and its menu of `lines`, whose choices come down
/// the channel; `menu_on_left` swaps the clicks, and `shown` is whether
/// the item is in the bar at all.
#[cfg(target_os = "macos")]
pub fn menu_bar(lines: &[MenuLine], menu_on_left: bool, shown: bool) -> futures::channel::mpsc::UnboundedReceiver<MenuPick> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    menu_bar::install(tx, lines, menu_on_left, shown);
    rx
}
#[cfg(not(target_os = "macos"))]
pub fn menu_bar(_lines: &[MenuLine], _menu_on_left: bool, _shown: bool) -> futures::channel::mpsc::UnboundedReceiver<MenuPick> {
    futures::channel::mpsc::unbounded().1
}

/// The settings changed which click shows the menu, or whether the item
/// is in the bar: it changes at once.
#[cfg(target_os = "macos")]
pub fn set_menu_bar(menu_on_left: bool, shown: bool) {
    menu_bar::set(menu_on_left, shown);
}
#[cfg(not(target_os = "macos"))]
pub fn set_menu_bar(_menu_on_left: bool, _shown: bool) {}

/// The menu's choices named again, in the language now in use.
#[cfg(target_os = "macos")]
pub fn retitle_menu(lines: &[MenuLine]) {
    menu_bar::retitle(lines);
}
#[cfg(not(target_os = "macos"))]
pub fn retitle_menu(_lines: &[MenuLine]) {}

/// For a probe: the item's frame (`menu_bar::frame`), and a picture of
/// its menu (`menu_bar::shoot`).
#[cfg(target_os = "macos")]
pub fn menu_bar_frame() -> Option<(f64, f64, f64, f64, bool)> {
    menu_bar::frame()
}
#[cfg(target_os = "macos")]
pub fn shoot_menu(path: std::path::PathBuf) {
    menu_bar::shoot(path);
}

/// Whether Kotoba opens when the person logs in, as the system has it:
/// the person may change it in System Settings, General, Login Items, so
/// it is read from there and never kept in the settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Login {
    Off,
    On,
    /// Asked for, and waiting on the person in Login Items.
    Waiting,
}

#[cfg(target_os = "macos")]
pub fn login() -> Login {
    use objc2_service_management::{SMAppService, SMAppServiceStatus};
    match unsafe { SMAppService::mainAppService().status() } {
        SMAppServiceStatus::Enabled => Login::On,
        SMAppServiceStatus::RequiresApproval => Login::Waiting,
        _ => Login::Off,
    }
}
#[cfg(not(target_os = "macos"))]
pub fn login() -> Login {
    Login::Off
}

/// Kotoba made to open at login, or not, as the app it is: the bundle
/// this process runs from.
#[cfg(target_os = "macos")]
pub fn set_login(on: bool) -> Result<(), String> {
    use objc2_service_management::SMAppService;
    let app = unsafe { SMAppService::mainAppService() };
    let done = if on { unsafe { app.registerAndReturnError() } } else { unsafe { app.unregisterAndReturnError() } };
    done.map_err(|e| e.localizedDescription().to_string())
}
#[cfg(not(target_os = "macos"))]
pub fn set_login(_on: bool) -> Result<(), String> {
    Err("opening at login is on a Mac only for now".into())
}

/// System Settings, at Login Items.
#[cfg(target_os = "macos")]
pub fn open_login_items() {
    unsafe { objc2_service_management::SMAppService::openSystemSettingsLoginItems() };
}
#[cfg(not(target_os = "macos"))]
pub fn open_login_items() {}

// The window's language. On a Mac an app's language is its own
// `AppleLanguages` in its defaults, which System Settings, Language &
// Region, Applications writes, and which the system reads for the app's
// own questions (the microphone's). Kotoba's setting is the same one.

#[cfg(target_os = "macos")]
mod languages {
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSArray, NSBundle, NSString, NSUserDefaults};

    const KEY: &str = "AppleLanguages";

    /// `AppleLanguages` as one domain of the defaults has it, alone: not
    /// the search list's, which falls back from the app's to the system's.
    fn list(domain: &str) -> Vec<String> {
        let defaults = NSUserDefaults::standardUserDefaults();
        let Some(dict) = defaults.persistentDomainForName(&NSString::from_str(domain)) else { return Vec::new() };
        let Some(value) = dict.objectForKey(&NSString::from_str(KEY)) else { return Vec::new() };
        let Ok(all) = value.downcast::<NSArray>() else { return Vec::new() };
        all.iter().filter_map(|o| o.downcast::<NSString>().ok().map(|s| s.to_string())).collect()
    }

    pub fn system() -> Vec<String> {
        list("NSGlobalDomain")
    }

    pub fn own() -> Option<String> {
        let id = NSBundle::mainBundle().bundleIdentifier()?;
        list(&id.to_string()).into_iter().next()
    }

    pub fn set_own(code: Option<&str>) {
        let defaults = NSUserDefaults::standardUserDefaults();
        let key = NSString::from_str(KEY);
        match code {
            Some(code) => {
                let one = NSArray::from_retained_slice(&[NSString::from_str(code)]);
                let one: &AnyObject = &one;
                unsafe { defaults.setObject_forKey(Some(one), &key) };
            }
            None => defaults.removeObjectForKey(&key),
        }
    }
}

/// The system's preferred languages, in its order, apart from any the
/// app was given of its own.
#[cfg(target_os = "macos")]
pub fn system_languages() -> Vec<String> {
    let all = languages::system();
    if all.is_empty() { sys_locale::get_locales().collect() } else { all }
}
#[cfg(not(target_os = "macos"))]
pub fn system_languages() -> Vec<String> {
    sys_locale::get_locales().collect()
}

/// The language the app was given of its own, if any: by Kotoba's
/// setting or by System Settings.
#[cfg(target_os = "macos")]
pub fn own_language() -> Option<String> {
    languages::own()
}
#[cfg(not(target_os = "macos"))]
pub fn own_language() -> Option<String> {
    None
}

/// The app given a language of its own (a bundle's code, `zh-Hans`), or
/// none, to follow the system's.
#[cfg(target_os = "macos")]
pub fn set_own_language(code: Option<&str>) {
    languages::set_own(code);
}
#[cfg(not(target_os = "macos"))]
pub fn set_own_language(_code: Option<&str>) {}

/// No Dock icon and no menu of its own at the screen's top: an app of
/// the menu bar. gpui makes every app a regular one as it starts, over
/// the bundle's `LSUIElement`, so this is said again after it.
#[cfg(target_os = "macos")]
pub fn stay_in_menu_bar() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    let Some(mtm) = MainThreadMarker::new() else { return };
    NSApplication::sharedApplication(mtm).setActivationPolicy(NSApplicationActivationPolicy::Accessory);
}
#[cfg(not(target_os = "macos"))]
pub fn stay_in_menu_bar() {}

/// In the Dock and ⌘-Tab while the settings window is open, as any app
/// with a window is, so it is found again once another app covers it;
/// back to the menu bar alone when it closes.
#[cfg(target_os = "macos")]
pub fn in_dock(on: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    let Some(mtm) = MainThreadMarker::new() else { return };
    let policy = if on { NSApplicationActivationPolicy::Regular } else { NSApplicationActivationPolicy::Accessory };
    NSApplication::sharedApplication(mtm).setActivationPolicy(policy);
}
#[cfg(not(target_os = "macos"))]
pub fn in_dock(_on: bool) {}

/// Where the app stands with the microphone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mic {
    /// Never asked: the system asks the first time it is opened, or when
    /// `ask_mic` asks.
    Unasked,
    Allowed,
    /// Refused, or not the person's to give: only System Settings changes it.
    Refused,
}

#[cfg(target_os = "macos")]
pub fn mic() -> Mic {
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};
    let Some(audio) = (unsafe { AVMediaTypeAudio }) else { return Mic::Unasked };
    match unsafe { AVCaptureDevice::authorizationStatusForMediaType(audio) } {
        AVAuthorizationStatus::Authorized => Mic::Allowed,
        AVAuthorizationStatus::NotDetermined => Mic::Unasked,
        _ => Mic::Refused,
    }
}
#[cfg(not(target_os = "macos"))]
pub fn mic() -> Mic {
    Mic::Refused
}

/// The system's own question for the microphone. What the person
/// answers comes down the channel once, on whatever thread the system
/// answers on.
#[cfg(target_os = "macos")]
pub fn ask_mic() -> futures::channel::oneshot::Receiver<bool> {
    use objc2_av_foundation::{AVCaptureDevice, AVMediaTypeAudio};
    let (tx, rx) = futures::channel::oneshot::channel();
    let Some(audio) = (unsafe { AVMediaTypeAudio }) else { return rx };
    let tx = std::sync::Mutex::new(Some(tx));
    let block = block2::RcBlock::new(move |granted: objc2::runtime::Bool| {
        if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
            let _ = tx.send(granted.as_bool());
        }
    });
    unsafe { AVCaptureDevice::requestAccessForMediaType_completionHandler(audio, &block) };
    rx
}

/// System Settings, Privacy & Security, at Accessibility and at the
/// microphone: where a refusal is undone.
pub const ACCESSIBILITY_SETTINGS_URL: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
pub const MICROPHONE_SETTINGS_URL: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone";
