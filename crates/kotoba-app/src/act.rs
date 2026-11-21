//! Control's hands: the steps carried out on this Mac (`control::Step`),
//! by the same means a person's hands would use. Keys and the pointer are
//! events posted to the system, a button is pressed through Accessibility,
//! and what is on the screen is read through it too. Every call here can
//! wait on another app, so all of it runs off the main thread.

use std::ffi::{c_void, CStr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use kotoba_core::control::{Held, Press, Step, Stroke, Way};
use kotoba_core::dictate::Stop;

type Ref = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Point {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Size {
    w: f64,
    h: f64,
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> Ref;
    fn AXUIElementCopyAttributeValue(element: Ref, attribute: Ref, value: *mut Ref) -> i32;
    fn AXUIElementSetAttributeValue(element: Ref, attribute: Ref, value: Ref) -> i32;
    fn AXUIElementPerformAction(element: Ref, action: Ref) -> i32;
    fn AXUIElementSetMessagingTimeout(element: Ref, seconds: f32) -> i32;
    fn AXValueGetValue(value: Ref, kind: u32, out: *mut c_void) -> bool;
    fn CGEventCreate(source: Ref) -> Ref;
    fn CGEventGetLocation(event: Ref) -> Point;
    fn CGEventSetLocation(event: Ref, at: Point);
    fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
    fn CGEventCreateMouseEvent(source: Ref, kind: u32, at: Point, button: u32) -> Ref;
    fn CGEventCreateScrollWheelEvent(source: Ref, units: u32, wheels: u32, first: i32, ...) -> Ref;
    fn CGEventSetFlags(event: Ref, flags: u64);
    fn CGEventSetIntegerValueField(event: Ref, field: u32, value: i64);
    fn CGEventPost(tap: u32, event: Ref);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(r: Ref);
    fn CFRetain(r: Ref) -> Ref;
    fn CFGetTypeID(r: Ref) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFStringCreateWithBytes(alloc: Ref, bytes: *const u8, len: isize, encoding: u32, external: bool) -> Ref;
    fn CFStringGetCStringPtr(text: Ref, encoding: u32) -> *const std::ffi::c_char;
    fn CFStringGetLength(text: Ref) -> isize;
    fn CFStringGetMaximumSizeForEncoding(len: isize, encoding: u32) -> isize;
    fn CFStringGetCString(text: Ref, buffer: *mut std::ffi::c_char, size: isize, encoding: u32) -> bool;
    fn CFArrayGetCount(array: Ref) -> isize;
    fn CFArrayGetValueAtIndex(array: Ref, at: isize) -> Ref;
    static kCFBooleanTrue: Ref;
}

const UTF8: u32 = 0x0800_0100;
const HID_TAP: u32 = 0;
/// `kCGEventSourceUserData`: marks an event as Kotoba's own, for its tap to
/// let through (`sys::outside`).
const USER_DATA: u32 = 42;
const MOUSE_CLICK_STATE: u32 = 1;
const AX_POINT: u32 = 1;
const AX_SIZE: u32 = 2;

/// A Core Foundation object, let go when dropped.
struct Cf(Ref);
// Each is used by one thread at a time, and Accessibility's calls are safe
// from any thread.
unsafe impl Send for Cf {}

impl Drop for Cf {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}

impl Cf {
    fn owned(r: Ref) -> Option<Cf> {
        (!r.is_null()).then_some(Cf(r))
    }

    fn retained(r: Ref) -> Option<Cf> {
        (!r.is_null()).then(|| Cf(unsafe { CFRetain(r) }))
    }

    fn string(text: &str) -> Cf {
        Cf(unsafe { CFStringCreateWithBytes(std::ptr::null_mut(), text.as_ptr(), text.len() as isize, UTF8, false) })
    }

    fn text(&self) -> Option<String> {
        unsafe {
            if CFGetTypeID(self.0) != CFStringGetTypeID() {
                return None;
            }
            let quick = CFStringGetCStringPtr(self.0, UTF8);
            if !quick.is_null() {
                return Some(CStr::from_ptr(quick).to_string_lossy().into_owned());
            }
            let size = CFStringGetMaximumSizeForEncoding(CFStringGetLength(self.0), UTF8) + 1;
            let mut out = vec![0 as std::ffi::c_char; size as usize];
            CFStringGetCString(self.0, out.as_mut_ptr(), size, UTF8).then(|| CStr::from_ptr(out.as_ptr()).to_string_lossy().into_owned())
        }
    }
}

fn attribute(element: Ref, name: &str) -> Option<Cf> {
    let mut value: Ref = std::ptr::null_mut();
    let key = Cf::string(name);
    let got = unsafe { AXUIElementCopyAttributeValue(element, key.0, &mut value) };
    if got == 0 {
        Cf::owned(value)
    } else {
        None
    }
}

fn words(element: Ref, name: &str) -> Option<String> {
    attribute(element, name).and_then(|v| v.text()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

fn children_of(list: Ref) -> Vec<Cf> {
    let count = unsafe { CFArrayGetCount(list) };
    (0..count.min(600)).filter_map(|i| Cf::retained(unsafe { CFArrayGetValueAtIndex(list, i) })).collect()
}

fn children(element: Ref) -> Vec<Cf> {
    attribute(element, "AXChildren").map_or(Vec::new(), |list| children_of(list.0))
}

/// What the screen shows of one thing a person could press or read.
struct Seen {
    element: Cf,
    role: String,
    name: String,
    frame: Option<(Point, Size)>,
}

fn frame_of(element: Ref) -> Option<(Point, Size)> {
    let (at, size) = (attribute(element, "AXPosition")?, attribute(element, "AXSize")?);
    let (mut point, mut extent) = (Point { x: 0., y: 0. }, Size { w: 0., h: 0. });
    let got = unsafe { AXValueGetValue(at.0, AX_POINT, &mut point as *mut Point as *mut c_void) && AXValueGetValue(size.0, AX_SIZE, &mut extent as *mut Size as *mut c_void) };
    got.then_some((point, extent))
}

/// How much a thing looks like what a person means to press, the most
/// likely first; text and fields come after what acts.
fn rank(role: &str) -> usize {
    const ROLES: [&str; 14] = ["AXButton", "AXLink", "AXMenuItem", "AXMenuBarItem", "AXCheckBox", "AXRadioButton", "AXPopUpButton", "AXMenuButton", "AXTab", "AXDisclosureTriangle", "AXRow", "AXCell", "AXTextField", "AXStaticText"];
    ROLES.iter().position(|r| *r == role).unwrap_or(ROLES.len())
}

/// Letters and digits alone, in lower case: what a name is compared by,
/// so "Save…" is "save" and "Add to Cart" is "addtocart".
fn bare(text: &str) -> String {
    text.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// The things in a tree, breadth first, with a name of their own, up to a
/// count and a time: a web page can hold many thousands.
fn collect(root: Cf, most: usize, by: Instant, out: &mut Vec<Seen>) {
    let mut waiting = std::collections::VecDeque::from([root]);
    let mut read = 0;
    while let Some(element) = waiting.pop_front() {
        read += 1;
        if read > most || Instant::now() > by {
            return;
        }
        let e = element.0;
        let role = words(e, "AXRole").unwrap_or_default();
        let name = words(e, "AXTitle").or_else(|| words(e, "AXDescription")).or_else(|| if matches!(role.as_str(), "AXStaticText" | "AXTextField" | "AXTextArea") { words(e, "AXValue") } else { None });
        waiting.extend(children(e));
        if let Some(name) = name {
            let frame = frame_of(e);
            out.push(Seen { element, role, name, frame });
        }
    }
}

/// What the app in front shows: its focused window, then its menu bar.
/// The Apple menu is left out, with Log Out, Restart and Shut Down on it.
fn screen(pid: i32) -> Result<Vec<Seen>, String> {
    let app = Cf::owned(unsafe { AXUIElementCreateApplication(pid) }).ok_or("no such app")?;
    unsafe {
        AXUIElementSetMessagingTimeout(app.0, 0.4);
        // Chromium and Electron draw nothing for Accessibility until asked.
        for ask in ["AXManualAccessibility", "AXEnhancedUserInterface"] {
            AXUIElementSetAttributeValue(app.0, Cf::string(ask).0, kCFBooleanTrue);
        }
    }
    let by = Instant::now() + Duration::from_millis(2500);
    let mut seen = Vec::new();
    let window = attribute(app.0, "AXFocusedWindow").or_else(|| attribute(app.0, "AXMainWindow")).or_else(|| attribute(app.0, "AXWindows").and_then(|list| children_of(list.0).into_iter().next()));
    if let Some(window) = window {
        collect(window, 5000, by, &mut seen);
    }
    if let Some(bar) = attribute(app.0, "AXMenuBar") {
        for menu in children(bar.0).into_iter().skip(1) {
            collect(menu, 400, by, &mut seen);
        }
    }
    Ok(seen)
}

/// The thing that best answers to a name: the name as the model wrote it,
/// else any name on the screen found in what the person said, the longest
/// one, for a model that translated what the screen calls it.
fn best<'a>(seen: &'a [Seen], name: &str, said: &str) -> Option<&'a Seen> {
    let want = bare(name);
    let close = |label: &str| -> usize {
        match () {
            _ if want.is_empty() => 0,
            _ if label == want => 3,
            _ if label.starts_with(&want) => 2,
            _ if label.contains(&want) => 1,
            _ => 0,
        }
    };
    let by_name = seen.iter().filter(|s| close(&bare(&s.name)) > 0).max_by_key(|s| (close(&bare(&s.name)), usize::MAX - rank(&s.role), usize::MAX - s.name.len()));
    by_name.or_else(|| {
        let said = bare(said);
        seen.iter().filter(|s| bare(&s.name).chars().count() >= 2 && said.contains(&bare(&s.name))).max_by_key(|s| (bare(&s.name).chars().count(), usize::MAX - rank(&s.role)))
    })
}

fn post(event: Ref) {
    unsafe {
        CGEventSetIntegerValueField(event, USER_DATA, crate::sys::POSTED);
        CGEventPost(HID_TAP, event);
        CFRelease(event);
    }
}

fn flags(held: Held) -> u64 {
    [(held.cmd, 0x10_0000u64), (held.shift, 0x2_0000), (held.ctrl, 0x4_0000), (held.alt, 0x8_0000)].iter().filter(|(on, _)| *on).map(|(_, bit)| bit).sum()
}

fn press_stroke(stroke: &Stroke) -> Result<(), Failed> {
    let code = crate::sys::key_code(&stroke.key).ok_or_else(|| Failed::Broke(stroke.key.clone()))?;
    // The arrows carry the flags a keyboard sends with them, or the Mac's
    // own shortcuts on them do not fire.
    let arrow = matches!(stroke.key.as_str(), "up" | "down" | "left" | "right");
    let flags = flags(stroke.held) | if arrow { 0xA0_0000 } else { 0 };
    for _ in 0..stroke.times {
        for down in [true, false] {
            let e = unsafe { CGEventCreateKeyboardEvent(std::ptr::null_mut(), code, down) };
            if e.is_null() {
                return Err(Failed::Broke(stroke.key.clone()));
            }
            unsafe { CGEventSetFlags(e, flags) };
            post(e);
            std::thread::sleep(Duration::from_millis(12));
        }
    }
    Ok(())
}

fn pointer() -> Point {
    unsafe {
        let e = CGEventCreate(std::ptr::null_mut());
        let at = CGEventGetLocation(e);
        CFRelease(e);
        at
    }
}

fn click_at(at: Point, how: Press) {
    let (down, up, button) = if how == Press::Right { (3, 4, 1) } else { (1, 2, 0) };
    let times = if how == Press::Double { 2 } else { 1 };
    unsafe {
        post(CGEventCreateMouseEvent(std::ptr::null_mut(), 5, at, 0));
        std::thread::sleep(Duration::from_millis(40));
        for n in 1..=times {
            for kind in [down, up] {
                let e = CGEventCreateMouseEvent(std::ptr::null_mut(), kind, at, button);
                CGEventSetIntegerValueField(e, MOUSE_CLICK_STATE, n);
                post(e);
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}

fn press_seen(seen: &Seen, how: Press) -> Result<(), Failed> {
    if how == Press::Click && unsafe { AXUIElementPerformAction(seen.element.0, Cf::string("AXPress").0) } == 0 {
        return Ok(());
    }
    let (at, size) = seen.frame.ok_or_else(|| Failed::NotOnScreen(seen.name.clone()))?;
    click_at(Point { x: at.x + size.w / 2., y: at.y + size.h / 2. }, how);
    Ok(())
}

/// The middle of the front window, where a scroll is sent, so it moves the
/// app in front wherever the pointer rests.
fn window_middle(pid: i32) -> Option<(Point, Size)> {
    let app = Cf::owned(unsafe { AXUIElementCreateApplication(pid) })?;
    let window = attribute(app.0, "AXFocusedWindow")?;
    let (at, size) = frame_of(window.0)?;
    Some((Point { x: at.x + size.w / 2., y: at.y + size.h / 2. }, size))
}

fn scroll(way: Way, pages: u8) {
    const TICKS: i32 = 12;
    let middle = front().and_then(|f| window_middle(f.pid));
    let page = middle.map_or(600., |(_, size)| size.h * 0.8) as i32;
    let (dy, dx) = match way {
        Way::Up => (page, 0),
        Way::Down => (-page, 0),
        Way::Left => (0, page),
        Way::Right => (0, -page),
    };
    for _ in 0..pages {
        for _ in 0..TICKS {
            unsafe {
                let e = CGEventCreateScrollWheelEvent(std::ptr::null_mut(), 0, 2, dy / TICKS, dx / TICKS);
                if e.is_null() {
                    return;
                }
                if let Some((at, _)) = middle {
                    CGEventSetLocation(e, at);
                }
                post(e);
            }
            std::thread::sleep(Duration::from_millis(14));
        }
    }
}

/// The app in front.
pub struct Front {
    pub pid: i32,
    pub name: String,
    id: String,
    path: String,
}

pub fn front() -> Option<Front> {
    use objc2_app_kit::NSWorkspace;
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    let path = app.bundleURL().and_then(|u| u.path()).map(|p| p.to_string()).unwrap_or_default();
    Some(Front { pid: app.processIdentifier(), name: app.localizedName().map(|n| n.to_string()).unwrap_or_default(), id: app.bundleIdentifier().map(|n| n.to_string()).unwrap_or_default(), path })
}

/// The apps on this Mac, by every name each goes by: the ones running, then
/// those in the usual folders, by their file and the name Finder shows.
fn apps() -> Vec<(String, String)> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSFileManager, NSString};
    let mut all = Vec::new();
    for app in NSWorkspace::sharedWorkspace().runningApplications().iter() {
        if let (Some(name), Some(path)) = (app.localizedName(), app.bundleURL().and_then(|u| u.path())) {
            if path.to_string().ends_with(".app") {
                all.push((name.to_string(), path.to_string()));
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let places = ["/Applications".to_string(), "/Applications/Utilities".to_string(), "/System/Applications".to_string(), "/System/Applications/Utilities".to_string(), "/System/Library/CoreServices".to_string(), format!("{home}/Applications")];
    let files = NSFileManager::defaultManager();
    let mut add = |path: &std::path::Path| {
        let (Some(file), Some(path)) = (path.file_stem().and_then(|s| s.to_str()), path.to_str()) else { return };
        all.push((file.to_string(), path.to_string()));
        let shown = files.displayNameAtPath(&NSString::from_str(path)).to_string();
        all.push((shown.strip_suffix(".app").unwrap_or(&shown).to_string(), path.to_string()));
    };
    for place in &places {
        let Ok(list) = std::fs::read_dir(place) else { continue };
        for entry in list.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "app") {
                add(&path);
            } else if path.is_dir() && place != "/System/Library/CoreServices" {
                // An app in a folder of its own: /Applications/Foo/Foo.app.
                if let Ok(inner) = std::fs::read_dir(&path) {
                    for app in inner.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "app")) {
                        add(&app);
                    }
                }
            }
        }
    }
    all
}

/// How many letters apart two names are, for a name misheard by a letter.
fn apart(a: &[char], b: &[char]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut last = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = last + usize::from(ca != cb);
            last = row[j + 1];
            row[j + 1] = cost.min(row[j] + 1).min(row[j + 1] + 1);
        }
    }
    row[b.len()]
}

/// The app that goes by a name: the same name, one that starts with it,
/// one that holds it, or one a letter or two off.
fn app_named(name: &str) -> Option<String> {
    let want = bare(name);
    if want.is_empty() {
        return None;
    }
    let all = apps();
    let chars: Vec<char> = want.chars().collect();
    let score = |label: &str| -> usize {
        let label = bare(label);
        if label == want {
            4
        } else if label.starts_with(&want) {
            3
        } else if chars.len() >= 3 && label.contains(&want) {
            2
        } else if chars.len() >= 5 && apart(&chars, &label.chars().collect::<Vec<_>>()) <= 2 {
            1
        } else {
            0
        }
    };
    // The first of the best: an app that is running over one that is not.
    let mut found: Option<(usize, String)> = None;
    for (label, path) in all {
        let s = score(&label);
        if s > 0 && found.as_ref().is_none_or(|(best, _)| s > *best) {
            found = Some((s, path));
        }
    }
    found.map(|(_, path)| path)
}

fn open_url(url: &str) -> bool {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};
    NSURL::URLWithString(&NSString::from_str(url)).is_some_and(|u| NSWorkspace::sharedWorkspace().openURL(&u))
}

/// An app opened, or brought to the front, and waited for: what follows
/// goes to it, and a keystroke sent too soon lands in the app before it.
fn open_app(path: &str, stop: &Stop) -> Result<(), Failed> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    if !NSWorkspace::sharedWorkspace().openURL(&url) {
        return Err(Failed::Broke(path.to_string()));
    }
    let by = Instant::now() + Duration::from_secs(6);
    while Instant::now() < by && !stop.stopped() {
        if front().is_some_and(|f| f.path == path) {
            std::thread::sleep(Duration::from_millis(350));
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(Failed::Broke(path.to_string()))
}

/// Terminals: a line typed there and entered is a command run, so Enter
/// stays the person's own to press.
const TERMINALS: [&str; 8] = ["com.apple.Terminal", "com.googlecode.iterm2", "com.mitchellh.ghostty", "com.github.wez.wezterm", "net.kovidgoyal.kitty", "dev.warp.Warp-Stable", "org.alacritty", "co.zeit.hyper"];

/// Why steps stopped short.
#[derive(Debug)]
pub enum Failed {
    /// No app goes by this name.
    NoApp(String),
    /// Nothing on the screen answers to this name.
    NoItem(String),
    NotOnScreen(String),
    /// Enter, held back in a terminal.
    Terminal,
    Stopped,
    Broke(String),
}

/// The steps, in turn. `said` is what the person said, for a name on the
/// screen the model translated; `agent` holds Enter back in a terminal for
/// each call, an agent's calls each being one step.
pub fn run(steps: &[Step], said: &str, agent: bool, stop: &Stop) -> Result<(), Failed> {
    let mut typed = agent;
    for step in steps {
        if stop.stopped() {
            return Err(Failed::Stopped);
        }
        match step {
            Step::Open(name) => match app_named(name) {
                Some(path) => open_app(&path, stop)?,
                // A site said as a name: "open youtube".
                None if name.chars().all(|c| c.is_ascii_alphanumeric()) => {
                    open_url(&format!("https://{}.com", name.to_lowercase()));
                    std::thread::sleep(Duration::from_millis(700));
                }
                None => return Err(Failed::NoApp(name.clone())),
            },
            Step::Web(url) => {
                if !open_url(url) {
                    return Err(Failed::Broke(url.clone()));
                }
                std::thread::sleep(Duration::from_millis(700));
            }
            Step::Keys(strokes) => {
                for stroke in strokes {
                    let enter = stroke.key == "enter" && stroke.held == Held::default();
                    if enter && typed && front().is_some_and(|f| TERMINALS.contains(&f.id.as_str())) {
                        return Err(Failed::Terminal);
                    }
                    press_stroke(stroke)?;
                }
                std::thread::sleep(Duration::from_millis(120));
            }
            Step::Type(words) => {
                typed = true;
                let was = crate::sys::paste_words(words);
                // As a dictation's paste: the app reads it, and what the
                // clipboard held goes back.
                std::thread::sleep(Duration::from_millis(600));
                crate::sys::restore_clipboard(was);
            }
            Step::Click(how, None) => click_at(pointer(), *how),
            Step::Click(how, Some(name)) => {
                let pid = front().ok_or_else(|| Failed::NoItem(name.clone()))?.pid;
                let seen = screen(pid).map_err(Failed::Broke)?;
                let found = best(&seen, name, said).ok_or_else(|| Failed::NoItem(name.clone()))?;
                press_seen(found, *how)?;
                std::thread::sleep(Duration::from_millis(250));
            }
            Step::Scroll(way, pages) => scroll(*way, *pages),
            Step::Ask => {}
        }
    }
    Ok(())
}

/// What the agent saw last, for it to press by number.
static LOOKED: Mutex<Vec<Seen>> = Mutex::new(Vec::new());

/// The app in front and what it shows, numbered, for an agent to read.
/// The text is the app's, not the person's (`agent`).
pub fn look() -> Result<String, String> {
    let app = front().ok_or("no app is in front")?;
    let seen = screen(app.pid)?;
    let mut out = format!("App in front: {}\n", app.name);
    if seen.is_empty() {
        out.push_str("Nothing can be read from this app.\n");
    }
    for (n, s) in seen.iter().enumerate().take(600) {
        let role = s.role.strip_prefix("AX").unwrap_or(&s.role);
        out.push_str(&format!("{n}. {role}: {}\n", s.name.chars().take(160).collect::<String>().replace('\n', " ")));
    }
    *LOOKED.lock().unwrap() = seen;
    Ok(out)
}

/// A press on something `look` numbered.
pub fn press_looked(n: usize, how: Press) -> Result<(), Failed> {
    let looked = LOOKED.lock().unwrap();
    let seen = looked.get(n).ok_or_else(|| Failed::NoItem(n.to_string()))?;
    press_seen(seen, how)
}

/// The numbered list let go: it holds references into another app.
pub fn forget() {
    LOOKED.lock().unwrap().clear();
}
