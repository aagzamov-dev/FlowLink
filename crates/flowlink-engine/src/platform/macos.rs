use super::Gate;
use crate::{
    input::{ClickTracker, Input, NativeEvent, ScrollAccumulator, ShortcutMapper},
    model::{Display, Permissions, Rect, Settings},
    Result,
};
use std::{
    collections::HashSet,
    ffi::c_void,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Point {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Size {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Bounds {
    origin: Point,
    size: Size,
}
type Event = *mut c_void;
const TAG: i64 = 0x464c4f574c494e4b;
const SHIFT: u64 = 1 << 17;
const CTRL: u64 = 1 << 18;
const ALT: u64 = 1 << 19;
const CMD: u64 = 1 << 20;
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
    fn CGGetActiveDisplayList(max: u32, ids: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayBounds(id: u32) -> Bounds;
    fn CGDisplayPixelsWide(id: u32) -> usize;
    fn CGDisplayPixelsHigh(id: u32) -> usize;
    fn CGDisplayCopyDisplayMode(id: u32) -> *const c_void;
    fn CGDisplayModeGetPixelWidth(mode: *const c_void) -> usize;
    fn CGDisplayModeGetPixelHeight(mode: *const c_void) -> usize;
    fn CGDisplayModeRelease(mode: *const c_void);
    fn CGDisplayRotation(id: u32) -> f64;
    fn CGDisplayCreateUUIDFromDisplayID(id: u32) -> *const c_void;
    fn CGEventCreate(source: *const c_void) -> Event;
    fn CGEventCreateMouseEvent(
        source: *const c_void,
        kind: u32,
        point: Point,
        button: u32,
    ) -> Event;
    fn CGEventCreateKeyboardEvent(source: *const c_void, key: u16, down: bool) -> Event;
    fn CGEventCreateScrollWheelEvent(source: *const c_void, units: u32, count: u32, ...) -> Event;
    fn CGEventGetLocation(event: Event) -> Point;
    fn CGEventGetIntegerValueField(event: Event, field: u32) -> i64;
    fn CGEventSetIntegerValueField(event: Event, field: u32, value: i64);
    fn CGEventSetLocation(event: Event, point: Point);
    fn CGEventGetFlags(event: Event) -> u64;
    fn CGEventSetFlags(event: Event, flags: u64);
    fn CGEventPost(tap: u32, event: Event);
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        mask: u64,
        callback: unsafe extern "C" fn(*mut c_void, u32, Event, *mut c_void) -> Event,
        user: *mut c_void,
    ) -> *mut c_void;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CGEventTapIsEnabled(tap: *mut c_void) -> bool;
    fn CGWarpMouseCursorPosition(point: Point) -> i32;
}
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(value: *const c_void);
    fn CFUUIDCreateString(allocator: *const c_void, uuid: *const c_void) -> *const c_void;
    fn CFStringGetCString(
        string: *const c_void,
        buffer: *mut i8,
        len: isize,
        encoding: u32,
    ) -> bool;
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: *mut c_void,
        order: isize,
    ) -> *mut c_void;
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(loop_: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRun();
    static kCFRunLoopCommonModes: *const c_void;
}
struct TapContext {
    gate: Arc<Gate>,
    tap: std::sync::atomic::AtomicPtr<c_void>,
    local_keys: [AtomicBool; 128],
    physical_keys: [AtomicBool; 128],
    physical_buttons: [AtomicBool; 5],
    cursor_x: std::sync::atomic::AtomicU64,
    cursor_y: std::sync::atomic::AtomicU64,
    local_buttons: [AtomicBool; 5],
    local_flags: std::sync::atomic::AtomicU64,
    remote_keys: [AtomicBool; 128],
    remote_buttons: [AtomicBool; 5],
}
impl TapContext {
    fn new(gate: Arc<Gate>) -> Self {
        Self {
            gate,
            tap: std::sync::atomic::AtomicPtr::new(ptr::null_mut()),
            local_keys: std::array::from_fn(|_| AtomicBool::new(false)),
            physical_keys: std::array::from_fn(|_| AtomicBool::new(false)),
            physical_buttons: std::array::from_fn(|_| AtomicBool::new(false)),
            cursor_x: std::sync::atomic::AtomicU64::new(0),
            cursor_y: std::sync::atomic::AtomicU64::new(0),
            local_buttons: std::array::from_fn(|_| AtomicBool::new(false)),
            local_flags: std::sync::atomic::AtomicU64::new(0),
            remote_keys: std::array::from_fn(|_| AtomicBool::new(false)),
            remote_buttons: std::array::from_fn(|_| AtomicBool::new(false)),
        }
    }
}
struct Injection {
    bounds: Vec<Rect>,
    mapper: ShortcutMapper,
    scroll: ScrollAccumulator,
    keys: HashSet<u16>,
    buttons: HashSet<u8>,

    clicks: ClickTracker,
}
pub struct Mac {
    context: Arc<TapContext>,
    injection: Mutex<Injection>,
}
impl Mac {
    pub fn start(gate: Arc<Gate>) -> Result<Self> {
        let context = Arc::new(TapContext::new(gate));
        let ctx = context.clone();
        std::thread::Builder::new()
            .name("flowlink-mac-events".into())
            .spawn(move || unsafe {
                let mask = [1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 22, 25, 26, 27]
                    .iter()
                    .fold(0u64, |m, t| m | (1u64 << t));
                let tap =
                    CGEventTapCreate(1, 0, 0, mask, callback, Arc::as_ptr(&ctx) as *mut c_void);
                if tap.is_null() {
                    ctx.gate.healthy.store(false, Ordering::SeqCst);
                    ctx.gate.push(NativeEvent::Failure(
                        "Grant Accessibility and Input Monitoring, then restart FlowLink".into(),
                    ));
                    return;
                }
                ctx.tap.store(tap, Ordering::SeqCst);
                let source = CFMachPortCreateRunLoopSource(ptr::null(), tap, 0);
                if source.is_null() {
                    ctx.gate.healthy.store(false, Ordering::SeqCst);
                    CFRelease(tap);
                    return;
                }
                CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
                CGEventTapEnable(tap, true);
                ctx.gate
                    .healthy
                    .store(CGEventTapIsEnabled(tap), Ordering::SeqCst);
                CFRunLoopRun();
                CFRelease(source);
                CFRelease(tap);
            })?;
        Ok(Self {
            context,
            injection: Mutex::new(Injection {
                bounds: Vec::new(),
                mapper: ShortcutMapper::default(),
                scroll: ScrollAccumulator::default(),
                keys: HashSet::new(),
                buttons: HashSet::new(),

                clicks: ClickTracker::default(),
            }),
        })
    }
    pub fn cursor(&self) -> (f64, f64) {
        unsafe {
            let e = CGEventCreate(ptr::null());
            if e.is_null() {
                return (0.0, 0.0);
            }
            let p = CGEventGetLocation(e);
            CFRelease(e);
            (p.x, p.y)
        }
    }
    pub fn has_local_input(&self) -> bool {
        self.context
            .physical_keys
            .iter()
            .chain(self.context.physical_buttons.iter())
            .any(|key| key.load(Ordering::SeqCst))
    }
    pub fn warp(&self, x: f64, y: f64) {
        if let Ok(mut state) = self.injection.lock() {
            state.bounds = displays("")
                .unwrap_or_default()
                .into_iter()
                .map(|d| d.rect)
                .collect();
        }
        self.context.cursor_x.store(x.to_bits(), Ordering::SeqCst);
        self.context.cursor_y.store(y.to_bits(), Ordering::SeqCst);
        unsafe {
            CGWarpMouseCursorPosition(Point { x, y });
        }
    }
    pub fn inject(&self, event: &Input, settings: &Settings) -> Result<()> {
        let mut state = self
            .injection
            .lock()
            .map_err(|_| "Input state unavailable")?;
        match event {
            Input::Motion { dx, dy } => {
                let x = f64::from_bits(self.context.cursor_x.load(Ordering::SeqCst));
                let y = f64::from_bits(self.context.cursor_y.load(Ordering::SeqCst));
                let (x, y) = crate::geometry::clamp_point(
                    x + dx * settings.pointer_speed,
                    y + dy * settings.pointer_speed,
                    &state.bounds,
                );
                let p = Point { x, y };
                self.context.cursor_x.store(p.x.to_bits(), Ordering::SeqCst);
                self.context.cursor_y.store(p.y.to_bits(), Ordering::SeqCst);
                let held = state.buttons.iter().min().copied();
                let kind = match held {
                    Some(0) => 6,
                    Some(1) => 7,
                    Some(_) => 27,
                    None => 5,
                };
                unsafe {
                    post(
                        CGEventCreateMouseEvent(ptr::null(), kind, p, held.unwrap_or(0) as u32),
                        self.flags(&state),
                    );
                }
            }
            Input::Button { button, down } => {
                self.context.remote_buttons[*button as usize].store(*down, Ordering::SeqCst);
                if *down {
                    state.buttons.insert(*button);
                } else {
                    state.buttons.remove(button);
                    if self.context.local_buttons[*button as usize].load(Ordering::SeqCst) {
                        return Ok(());
                    }
                }
                let x = f64::from_bits(self.context.cursor_x.load(Ordering::SeqCst));
                let y = f64::from_bits(self.context.cursor_y.load(Ordering::SeqCst));
                let (kind, number) = match button {
                    0 => (if *down { 1 } else { 2 }, 0),
                    1 => (if *down { 3 } else { 4 }, 1),
                    _ => (if *down { 25 } else { 26 }, *button as u32),
                };
                let clicks = state.clicks.count(
                    *button,
                    *down,
                    x,
                    y,
                    std::time::Instant::now(),
                    std::time::Duration::from_secs_f64(
                        objc2_app_kit::NSEvent::doubleClickInterval(),
                    ),
                );
                unsafe {
                    let event = CGEventCreateMouseEvent(ptr::null(), kind, Point { x, y }, number);
                    if !event.is_null() {
                        CGEventSetIntegerValueField(event, 1, clicks as i64);
                    }
                    post(event, self.flags(&state));
                }
            }
            Input::Scroll {
                x,
                y,
                phase,
                momentum,
            } => {
                let direction = if settings.reverse_scroll { -1.0 } else { 1.0 };
                let (sx, sy) = state.scroll.take(x * direction, y * direction);
                if sx != 0 || sy != 0 || *phase != 0 || *momentum != 0 {
                    unsafe {
                        let event = CGEventCreateScrollWheelEvent(ptr::null(), 0, 2, sy, sx);
                        if !event.is_null() {
                            CGEventSetIntegerValueField(event, 99, *phase as i64);
                            CGEventSetIntegerValueField(event, 123, *momentum as i64);
                            CGEventSetLocation(
                                event,
                                Point {
                                    x: f64::from_bits(self.context.cursor_x.load(Ordering::SeqCst)),
                                    y: f64::from_bits(self.context.cursor_y.load(Ordering::SeqCst)),
                                },
                            );
                        }
                        post(event, self.flags(&state));
                    }
                }
            }
            Input::Key { scan, down } => {
                let terminal = *down
                    && !state.keys.contains(scan)
                    && settings
                        .shortcuts
                        .iter()
                        .any(|rule| rule.scan_code == *scan)
                    && foreground_app().is_some_and(|app| settings.terminal_apps.contains(&app));
                let mapped = state.mapper.map(
                    *scan,
                    *down,
                    settings,
                    crate::model::Direction::WindowsToMac,
                    terminal,
                );
                for key in mapped {
                    let Input::Key { scan, down } = key else {
                        continue;
                    };
                    let Some(mac) = mac_key(scan) else {
                        continue;
                    };
                    self.context.remote_keys[mac as usize].store(down, Ordering::SeqCst);
                    let repeating = down && state.keys.contains(&scan);
                    if scan == 0x3a && down && !repeating {
                        self.context
                            .local_flags
                            .fetch_xor(1 << 16, Ordering::SeqCst);
                    }
                    if down {
                        state.keys.insert(scan);
                    } else {
                        state.keys.remove(&scan);
                        if self.context.local_keys[mac as usize].load(Ordering::SeqCst) {
                            continue;
                        }
                    }
                    unsafe {
                        let event = CGEventCreateKeyboardEvent(ptr::null(), mac, down);
                        if !event.is_null() {
                            CGEventSetIntegerValueField(event, 8, repeating as i64);
                        }
                        post(event, self.flags(&state));
                    }
                }
            }
        }
        Ok(())
    }
    fn flags(&self, state: &Injection) -> u64 {
        let mut f = self.context.local_flags.load(Ordering::SeqCst);
        for scan in &state.keys {
            f |= match scan {
                0x2a | 0x36 => SHIFT,
                0x1d | 0x11d => CTRL,
                0x38 | 0x138 => ALT,
                0x15b | 0x15c => CMD,
                _ => 0,
            };
        }
        f
    }
    pub fn session_available(&self) -> bool {
        unsafe { !IsSecureEventInputEnabled() }
    }
    pub fn release(&self) {
        if let Ok(mut s) = self.injection.lock() {
            for scan in s.keys.drain() {
                if let Some(mac) = mac_key(scan) {
                    if !self.context.local_keys[mac as usize].load(Ordering::SeqCst) {
                        unsafe {
                            post(
                                CGEventCreateKeyboardEvent(ptr::null(), mac, false),
                                self.context.local_flags.load(Ordering::SeqCst),
                            );
                        }
                    }
                }
            }
            let (x, y) = self.cursor();
            for b in s.buttons.drain() {
                if !self.context.local_buttons[b as usize].load(Ordering::SeqCst) {
                    let kind = match b {
                        0 => 2,
                        1 => 4,
                        _ => 26,
                    };
                    unsafe {
                        post(
                            CGEventCreateMouseEvent(ptr::null(), kind, Point { x, y }, b as u32),
                            self.context.local_flags.load(Ordering::SeqCst),
                        );
                    }
                }
            }
            s.mapper.clear();
            s.scroll = ScrollAccumulator::default();
            for key in &self.context.remote_keys {
                key.store(false, Ordering::SeqCst);
            }
            for button in &self.context.remote_buttons {
                button.store(false, Ordering::SeqCst);
            }
        }
    }
}
unsafe fn post(event: Event, flags: u64) {
    if !event.is_null() {
        CGEventSetIntegerValueField(event, 42, TAG);
        CGEventSetFlags(event, flags);
        CGEventPost(1, event);
        CFRelease(event);
    }
}
unsafe extern "C" fn callback(_: *mut c_void, kind: u32, event: Event, user: *mut c_void) -> Event {
    let ctx = &*(user as *const TapContext);
    if kind == 0xfffffffe || kind == 0xffffffff {
        ctx.gate.stop();
        let tap = ctx.tap.load(Ordering::SeqCst);
        let enabled = if kind == 0xfffffffe && !tap.is_null() {
            CGEventTapEnable(tap, true);
            CGEventTapIsEnabled(tap)
        } else {
            false
        };
        ctx.gate.healthy.store(enabled, Ordering::SeqCst);
        ctx.gate.fault.store(true, Ordering::SeqCst);
        return event;
    }
    let synthetic = CGEventGetIntegerValueField(event, 42) == TAG;
    if synthetic {
        if ctx.gate.active() && !ctx.gate.capturing() && matches!(kind, 5 | 6 | 7 | 27) {
            let p = CGEventGetLocation(event);
            ctx.gate.push(NativeEvent::Position(p.x, p.y));
        }
        return event;
    }
    let flags = CGEventGetFlags(event);
    let key = CGEventGetIntegerValueField(event, 9) as usize;
    let button = CGEventGetIntegerValueField(event, 3) as usize;
    let generation = ctx.gate.generation.load(Ordering::SeqCst);
    let captured = ctx.gate.capturing();
    let blocked = captured || (ctx.gate.active() && ctx.gate.block_local.load(Ordering::SeqCst));
    let mut input = None;
    if matches!(kind, 10..=12) && key < 128 {
        let down = if kind == 12 {
            let mask = match key {
                56 | 60 => SHIFT,
                59 | 62 => CTRL,
                58 | 61 => ALT,
                55 | 54 => CMD,
                57 => 1 << 16,
                _ => 0,
            };
            if key == 57 {
                flags & mask != 0
            } else {
                flags & mask != 0 && !ctx.physical_keys[key].load(Ordering::SeqCst)
            }
        } else {
            kind == 10
        };
        ctx.physical_keys[key].store(down && key != 57, Ordering::SeqCst);
        if !blocked || !down {
            ctx.local_keys[key].store(down, Ordering::SeqCst);
        }
        let emergency = if ctx.gate.emergency_key.load(Ordering::SeqCst) == 1 {
            53
        } else {
            111
        };
        if down && key == emergency && flags & (SHIFT | CTRL | ALT) == SHIFT | CTRL | ALT {
            ctx.gate.stop();
            ctx.gate.push(NativeEvent::Emergency);
            return ptr::null_mut();
        }
        if down && key == 124 && flags & (SHIFT | CTRL | ALT) == SHIFT | CTRL | ALT {
            ctx.gate.push(NativeEvent::Switch);
            return ptr::null_mut();
        }
        if let Some(scan) = pc_key(key as u16) {
            if key == 57 {
                ctx.gate.push(NativeEvent::Input {
                    generation,
                    event: Input::Key { scan, down: true },
                    captured,
                });
                input = Some(Input::Key { scan, down: false });
            } else {
                input = Some(Input::Key { scan, down });
            }
        }
    } else if button < 5 && matches!(kind, 1 | 2 | 3 | 4 | 25 | 26) {
        let down = matches!(kind, 1 | 3 | 25);
        ctx.physical_buttons[button].store(down, Ordering::SeqCst);
        if !blocked || !down {
            ctx.local_buttons[button].store(down, Ordering::SeqCst);
        }
        input = Some(Input::Button {
            button: button as u8,
            down,
        });
    } else if kind == 22 {
        let continuous = CGEventGetIntegerValueField(event, 88) != 0;
        let (x, y) = if continuous {
            (
                CGEventGetIntegerValueField(event, 97) as f64,
                CGEventGetIntegerValueField(event, 96) as f64,
            )
        } else {
            (
                CGEventGetIntegerValueField(event, 12) as f64 * 40.0,
                CGEventGetIntegerValueField(event, 11) as f64 * 40.0,
            )
        };
        input = Some(Input::Scroll {
            x,
            y,
            phase: CGEventGetIntegerValueField(event, 99) as u8,
            momentum: CGEventGetIntegerValueField(event, 123) as u8,
        });
    } else if matches!(kind, 5 | 6 | 7 | 27) {
        if captured {
            input = Some(Input::Motion {
                dx: CGEventGetIntegerValueField(event, 4) as f64,
                dy: CGEventGetIntegerValueField(event, 5) as f64,
            });
        } else {
            let p = CGEventGetLocation(event);
            if !blocked {
                ctx.cursor_x.store(p.x.to_bits(), Ordering::SeqCst);
                ctx.cursor_y.store(p.y.to_bits(), Ordering::SeqCst);
            }
            if !ctx.gate.active() {
                ctx.gate.push(NativeEvent::Position(p.x, p.y));
            }
        }
    }
    let mut local_flags = flags & !(SHIFT | CTRL | ALT | CMD);
    for (keys, flag) in [
        ([56, 60], SHIFT),
        ([59, 62], CTRL),
        ([58, 61], ALT),
        ([55, 54], CMD),
    ] {
        if keys
            .iter()
            .any(|&key| ctx.local_keys[key].load(Ordering::SeqCst))
        {
            local_flags |= flag;
        }
    }
    ctx.local_flags.store(local_flags, Ordering::SeqCst);
    if let Some(event) = input {
        ctx.gate.push(NativeEvent::Input {
            generation,
            event,
            captured,
        });
    }
    if blocked {
        return ptr::null_mut();
    }
    if ctx.gate.active()
        && ((kind == 11 && key < 128 && ctx.remote_keys[key].load(Ordering::SeqCst))
            || (matches!(kind, 2 | 4 | 26)
                && button < 5
                && ctx.remote_buttons[button].load(Ordering::SeqCst)))
    {
        return ptr::null_mut();
    }
    event
}
fn pc_key(key: u16) -> Option<u16> {
    static KEYS: std::sync::OnceLock<[Option<u16>; 128]> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| {
        std::array::from_fn(|key| (1..=0x1ff).find(|scan| mac_key(*scan) == Some(key as u16)))
    })
    .get(key as usize)
    .copied()
    .flatten()
}
pub fn permissions() -> Permissions {
    unsafe {
        let accessibility = AXIsProcessTrusted();
        let monitoring = CGPreflightListenEventAccess();
        Permissions {
            accessibility,
            monitoring,
            filtering: accessibility && monitoring,
        }
    }
}
pub fn request_permissions() {
    unsafe {
        CGRequestListenEventAccess();
        AXIsProcessTrustedWithOptions(ptr::null());
    }
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}
pub fn foreground_app() -> Option<String> {
    objc2_app_kit::NSWorkspace::sharedWorkspace()
        .frontmostApplication()?
        .bundleIdentifier()
        .map(|s| s.to_string())
}
pub fn copy_text(text: &str) -> Result<()> {
    use std::io::Write;
    let mut child = std::process::Command::new("/usr/bin/pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("Clipboard unavailable")?
        .write_all(text.as_bytes())?;
    if !child.wait()?.success() {
        return Err("Could not copy address".into());
    }
    Ok(())
}
pub fn displays(device_id: &str) -> Result<Vec<Display>> {
    unsafe {
        let mut ids = [0u32; 32];
        let mut count = 0;
        if CGGetActiveDisplayList(32, ids.as_mut_ptr(), &mut count) != 0 {
            return Err("Could not enumerate Mac displays".into());
        }
        let mut out = Vec::new();
        for (index, id) in ids.iter().take(count as usize).enumerate() {
            let bounds = CGDisplayBounds(*id);
            let uuid = CGDisplayCreateUUIDFromDisplayID(*id);
            let mut stable = format!("display-{id}");
            if !uuid.is_null() {
                let string = CFUUIDCreateString(ptr::null(), uuid);
                let mut buffer = [0i8; 128];
                if !string.is_null() {
                    if CFStringGetCString(string, buffer.as_mut_ptr(), 128, 0x08000100) {
                        stable = std::ffi::CStr::from_ptr(buffer.as_ptr())
                            .to_string_lossy()
                            .into_owned();
                    }
                    CFRelease(string);
                }
                CFRelease(uuid);
            }
            let mode = CGDisplayCopyDisplayMode(*id);
            let (pw, ph) = if mode.is_null() {
                (
                    CGDisplayPixelsWide(*id) as u32,
                    CGDisplayPixelsHigh(*id) as u32,
                )
            } else {
                let dimensions = (
                    CGDisplayModeGetPixelWidth(mode) as u32,
                    CGDisplayModeGetPixelHeight(mode) as u32,
                );
                CGDisplayModeRelease(mode);
                dimensions
            };
            out.push(Display {
                id: stable,
                device_id: device_id.into(),
                name: format!("Mac display {}", index + 1),
                number: (index + 1) as u32,
                rect: Rect {
                    x: bounds.origin.x,
                    y: bounds.origin.y,
                    width: bounds.size.width,
                    height: bounds.size.height,
                },
                pixel_width: pw,
                pixel_height: ph,
                scale: pw as f64 / bounds.size.width,
                rotation: CGDisplayRotation(*id),
                online: true,
            });
        }
        Ok(out)
    }
}
fn mac_key(scan: u16) -> Option<u16> {
    Some(match scan {
        0x01 => 53,
        0x02 => 18,
        0x03 => 19,
        0x04 => 20,
        0x05 => 21,
        0x06 => 23,
        0x07 => 22,
        0x08 => 26,
        0x09 => 28,
        0x0a => 25,
        0x0b => 29,
        0x0c => 27,
        0x0d => 24,
        0x0e => 51,
        0x0f => 48,
        0x10 => 12,
        0x11 => 13,
        0x12 => 14,
        0x13 => 15,
        0x14 => 17,
        0x15 => 16,
        0x16 => 32,
        0x17 => 34,
        0x18 => 31,
        0x19 => 35,
        0x1a => 33,
        0x1b => 30,
        0x1c => 36,
        0x1d => 59,
        0x1e => 0,
        0x1f => 1,
        0x20 => 2,
        0x21 => 3,
        0x22 => 5,
        0x23 => 4,
        0x24 => 38,
        0x25 => 40,
        0x26 => 37,
        0x27 => 41,
        0x28 => 39,
        0x29 => 50,
        0x2a => 56,
        0x2b => 42,
        0x2c => 6,
        0x2d => 7,
        0x2e => 8,
        0x2f => 9,
        0x30 => 11,
        0x31 => 45,
        0x32 => 46,
        0x33 => 43,
        0x34 => 47,
        0x35 => 44,
        0x36 => 60,
        0x37 => 67,
        0x38 => 58,
        0x39 => 49,
        0x3a => 57,
        0x3b => 122,
        0x3c => 120,
        0x3d => 99,
        0x3e => 118,
        0x3f => 96,
        0x40 => 97,
        0x41 => 98,
        0x42 => 100,
        0x43 => 101,
        0x44 => 109,
        0x57 => 103,
        0x58 => 111,
        0x45 => 71,
        0x47 => 89,
        0x48 => 91,
        0x49 => 92,
        0x4a => 78,
        0x4b => 86,
        0x4c => 87,
        0x4d => 88,
        0x4e => 69,
        0x4f => 83,
        0x50 => 84,
        0x51 => 85,
        0x52 => 82,
        0x53 => 65,
        0x11c => 76,
        0x11d => 62,
        0x135 => 75,
        0x138 => 61,
        0x147 => 115,
        0x148 => 126,
        0x149 => 116,
        0x14b => 123,
        0x14d => 124,
        0x14f => 119,
        0x150 => 125,
        0x151 => 121,
        0x152 => 114,
        0x153 => 117,
        0x15b => 55,
        0x15c => 54,
        _ => return None,
    })
}

#[link(name = "SystemConfiguration", kind = "framework")]
extern "C" {
    fn SCNetworkInterfaceCopyAll() -> *const c_void;
    fn SCNetworkInterfaceGetBSDName(interface: *const c_void) -> *const c_void;
    fn SCNetworkInterfaceGetInterfaceType(interface: *const c_void) -> *const c_void;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(array: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
}
unsafe fn string(value: *const c_void) -> String {
    if value.is_null() {
        return String::new();
    }
    let mut buffer = [0i8; 256];
    if CFStringGetCString(value, buffer.as_mut_ptr(), 256, 0x08000100) {
        std::ffi::CStr::from_ptr(buffer.as_ptr())
            .to_string_lossy()
            .into_owned()
    } else {
        String::new()
    }
}
pub fn interface_kind(name: &str) -> String {
    unsafe {
        let interfaces = SCNetworkInterfaceCopyAll();
        if interfaces.is_null() {
            return "LAN".into();
        }
        let mut result = "LAN".to_string();
        for index in 0..CFArrayGetCount(interfaces) {
            let interface = CFArrayGetValueAtIndex(interfaces, index);
            if string(SCNetworkInterfaceGetBSDName(interface)) == name {
                result = match string(SCNetworkInterfaceGetInterfaceType(interface)).as_str() {
                    "IEEE80211" => "Wi-Fi",
                    "Ethernet" => "Ethernet",
                    _ => "LAN",
                }
                .into();
                break;
            }
        }
        CFRelease(interfaces);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocking_local_input_still_tracks_modifier_release() {
        let native = crate::platform::Native::for_test();
        let context = TapContext::new(native.gate.clone());
        context.local_keys[56].store(true, Ordering::SeqCst);
        context.physical_keys[56].store(true, Ordering::SeqCst);
        context.local_flags.store(SHIFT, Ordering::SeqCst);
        context.gate.remote.store(true, Ordering::SeqCst);
        context.gate.block_local.store(true, Ordering::SeqCst);
        context
            .gate
            .deadline
            .store(crate::platform::clock() + 1000, Ordering::SeqCst);
        unsafe {
            let event = CGEventCreateKeyboardEvent(ptr::null(), 56, false);
            assert!(!event.is_null());
            CGEventSetFlags(event, 0);
            assert!(callback(
                ptr::null_mut(),
                12,
                event,
                &context as *const _ as *mut c_void
            )
            .is_null());
            CFRelease(event);
        }
        assert!(!context.local_keys[56].load(Ordering::SeqCst));
        assert!(!context.physical_keys[56].load(Ordering::SeqCst));
        assert_eq!(context.local_flags.load(Ordering::SeqCst) & SHIFT, 0);
    }
    #[test]
    fn keyboard_translation_roundtrips_supported_keys() {
        for scan in [0x1d, 0x11d, 0x2e, 0x15b, 0x15c, 0x14d, 0x1c, 0x11c] {
            assert_eq!(pc_key(mac_key(scan).unwrap()), Some(scan));
        }
    }
}
