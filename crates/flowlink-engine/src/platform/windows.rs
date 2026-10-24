use super::Gate;
use crate::{
    input::{Input, NativeEvent, ScrollAccumulator, ShortcutMapper},
    model::{Display, Rect, Settings},
    Result,
};
use std::{
    collections::HashSet,
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
const TAG: usize = 0x464c4f57;
struct Context {
    gate: Arc<Gate>,
    held: [AtomicBool; 512],
    local_keys: [AtomicBool; 512],
    physical_buttons: [AtomicBool; 5],
    local_buttons: [AtomicBool; 5],
    remote_keys: [AtomicBool; 512],
    remote_buttons: [AtomicBool; 5],
    remote_x: AtomicU64,
    remote_y: AtomicU64,
    x: AtomicI32,
    y: AtomicI32,
    anchor_x: AtomicI32,
    anchor_y: AtomicI32,
}
static CONTEXT: OnceLock<Arc<Context>> = OnceLock::new();
#[derive(Default)]
struct Injection {
    bounds: Vec<Rect>,
    keys: HashSet<u16>,
    buttons: HashSet<u8>,
    mapper: ShortcutMapper,
    scroll: ScrollAccumulator,
}
pub struct Windows {
    context: Arc<Context>,
    injection: Mutex<Injection>,
}
impl Windows {
    pub fn start(gate: Arc<Gate>) -> Result<Self> {
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        let context = Arc::new(Context {
            gate,
            held: std::array::from_fn(|_| AtomicBool::new(false)),
            local_keys: std::array::from_fn(|_| AtomicBool::new(false)),
            physical_buttons: std::array::from_fn(|_| AtomicBool::new(false)),
            local_buttons: std::array::from_fn(|_| AtomicBool::new(false)),
            remote_keys: std::array::from_fn(|_| AtomicBool::new(false)),
            remote_buttons: std::array::from_fn(|_| AtomicBool::new(false)),
            remote_x: AtomicU64::new(0),
            remote_y: AtomicU64::new(0),
            x: AtomicI32::new(0),
            y: AtomicI32::new(0),
            anchor_x: AtomicI32::new(0),
            anchor_y: AtomicI32::new(0),
        });
        CONTEXT
            .set(context.clone())
            .map_err(|_| "Input hooks already initialized")?;
        let ctx = context.clone();
        std::thread::Builder::new()
            .name("flowlink-windows-hooks".into())
            .spawn(move || unsafe {
                let keyboard =
                    SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), ptr::null_mut(), 0);
                let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), ptr::null_mut(), 0);
                if keyboard.is_null() || mouse.is_null() {
                    ctx.gate.healthy.store(false, Ordering::SeqCst);
                    ctx.gate.push(NativeEvent::Failure(
                        "Windows input hooks could not start".into(),
                    ));
                    if !keyboard.is_null() {
                        UnhookWindowsHookEx(keyboard);
                    }
                    if !mouse.is_null() {
                        UnhookWindowsHookEx(mouse);
                    }
                    return;
                }
                ctx.gate.healthy.store(true, Ordering::SeqCst);
                let mut msg = std::mem::zeroed();
                while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                ctx.gate.stop();
                ctx.gate.healthy.store(false, Ordering::SeqCst);
                UnhookWindowsHookEx(keyboard);
                UnhookWindowsHookEx(mouse);
            })?;
        Ok(Self {
            context,
            injection: Mutex::new(Injection::default()),
        })
    }
    pub fn cursor(&self) -> (f64, f64) {
        unsafe {
            let mut p = POINT { x: 0, y: 0 };
            GetCursorPos(&mut p);
            (p.x as f64, p.y as f64)
        }
    }
    pub fn has_local_input(&self) -> bool {
        self.context
            .held
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
        self.context.remote_x.store(x.to_bits(), Ordering::SeqCst);
        self.context.remote_y.store(y.to_bits(), Ordering::SeqCst);
        self.context.x.store(x as i32, Ordering::SeqCst);
        self.context.y.store(y as i32, Ordering::SeqCst);
        self.context.anchor_x.store(x as i32, Ordering::SeqCst);
        self.context.anchor_y.store(y as i32, Ordering::SeqCst);
        unsafe {
            SetCursorPos(x as i32, y as i32);
        }
    }
    pub fn session_available(&self) -> bool {
        unsafe {
            use windows_sys::Win32::System::StationsAndDesktops::*;
            let desktop = OpenInputDesktop(0, 0, DESKTOP_READOBJECTS);
            if desktop.is_null() {
                return false;
            }
            let mut name = [0u16; 64];
            let mut needed = 0;
            let ok = GetUserObjectInformationW(
                desktop,
                UOI_NAME,
                name.as_mut_ptr().cast(),
                128,
                &mut needed,
            ) != 0;
            CloseDesktop(desktop);
            ok && String::from_utf16_lossy(&name[..name.iter().position(|v| *v == 0).unwrap_or(64)])
                == "Default"
        }
    }
    pub fn release(&self) {
        if let Ok(mut state) = self.injection.lock() {
            for scan in state.keys.drain() {
                self.context.remote_keys[scan as usize].store(false, Ordering::SeqCst);
                if !self.context.local_keys[scan as usize].load(Ordering::SeqCst) {
                    let _ = send_key(scan, false);
                }
            }
            for button in state.buttons.drain() {
                self.context.remote_buttons[button as usize].store(false, Ordering::SeqCst);
                if !self.context.local_buttons[button as usize].load(Ordering::SeqCst) {
                    let _ = send_button(button, false);
                }
            }
            state.mapper.clear();
            state.scroll = ScrollAccumulator::default();
        }
    }
    pub fn inject(&self, event: &Input, settings: &Settings) -> Result<()> {
        let mut state = self.injection.lock().map_err(|_| "Input unavailable")?;
        match event {
            Input::Motion { dx, dy } => unsafe {
                let left = GetSystemMetrics(SM_XVIRTUALSCREEN) as f64;
                let top = GetSystemMetrics(SM_YVIRTUALSCREEN) as f64;
                let width = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(2) as f64;
                let height = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(2) as f64;
                let x = (f64::from_bits(self.context.remote_x.load(Ordering::SeqCst))
                    + dx * settings.pointer_speed)
                    .clamp(left, left + width - 1.0);
                let y = (f64::from_bits(self.context.remote_y.load(Ordering::SeqCst))
                    + dy * settings.pointer_speed)
                    .clamp(top, top + height - 1.0);
                let (x, y) = crate::geometry::clamp_point(x, y, &state.bounds);
                self.context.remote_x.store(x.to_bits(), Ordering::SeqCst);
                self.context.remote_y.store(y.to_bits(), Ordering::SeqCst);
                send_mouse(
                    ((x - left) * 65535.0 / (width - 1.0)).round() as i32,
                    ((y - top) * 65535.0 / (height - 1.0)).round() as i32,
                    0,
                    MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                )?;
            },
            Input::Scroll { x, y, .. } => {
                let factor = if settings.reverse_scroll { -3.0 } else { 3.0 };
                let (x, y) = state.scroll.take(x * factor, y * factor);
                if x != 0 {
                    send_mouse(0, 0, x as u32, MOUSEEVENTF_HWHEEL)?;
                }
                if y != 0 {
                    send_mouse(0, 0, y as u32, MOUSEEVENTF_WHEEL)?;
                }
            }
            Input::Button { button, down } => {
                if *down {
                    for scan in state.mapper.pointer_modifiers() {
                        state.keys.insert(scan);
                        self.context.remote_keys[scan as usize].store(true, Ordering::SeqCst);
                        send_key(scan, true)?;
                    }
                }
                self.context.remote_buttons[*button as usize].store(*down, Ordering::SeqCst);
                if *down {
                    state.buttons.insert(*button);
                } else {
                    state.buttons.remove(button);
                }
                if *down || !self.context.local_buttons[*button as usize].load(Ordering::SeqCst) {
                    send_button(*button, *down)?;
                }
            }
            Input::Key { scan, down } => {
                for key in state.mapper.map(
                    *scan,
                    *down,
                    settings,
                    crate::model::Direction::MacToWindows,
                    false,
                ) {
                    let Input::Key { scan, down } = key else {
                        continue;
                    };
                    self.context.remote_keys[scan as usize].store(down, Ordering::SeqCst);
                    if down {
                        state.keys.insert(scan);
                    } else {
                        state.keys.remove(&scan);
                    }
                    if down || !self.context.local_keys[scan as usize].load(Ordering::SeqCst) {
                        send_key(scan, down)?;
                    }
                }
            }
        }
        Ok(())
    }
}
fn send(input: INPUT) -> Result<()> {
    if unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) } != 1 {
        return Err("Windows rejected input. Protected or elevated windows cannot be controlled at this permission level".into());
    }
    Ok(())
}
fn send_key(scan: u16, down: bool) -> Result<()> {
    send(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: scan & 0xff,
                dwFlags: KEYEVENTF_SCANCODE
                    | if scan & 0x100 != 0 {
                        KEYEVENTF_EXTENDEDKEY
                    } else {
                        0
                    }
                    | if down { 0 } else { KEYEVENTF_KEYUP },
                time: 0,
                dwExtraInfo: TAG,
            },
        },
    })
}
fn send_mouse(dx: i32, dy: i32, data: u32, flags: u32) -> Result<()> {
    send(INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: TAG,
            },
        },
    })
}
fn send_button(button: u8, down: bool) -> Result<()> {
    let flag = match (button, down) {
        (0, true) => MOUSEEVENTF_LEFTDOWN,
        (0, false) => MOUSEEVENTF_LEFTUP,
        (1, true) => MOUSEEVENTF_RIGHTDOWN,
        (1, false) => MOUSEEVENTF_RIGHTUP,
        (2, true) => MOUSEEVENTF_MIDDLEDOWN,
        (2, false) => MOUSEEVENTF_MIDDLEUP,
        (_, true) => MOUSEEVENTF_XDOWN,
        (_, false) => MOUSEEVENTF_XUP,
    };
    send_mouse(
        0,
        0,
        if button >= 3 { (button - 2) as u32 } else { 0 },
        flag,
    )
}
unsafe extern "system" fn keyboard_hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(ptr::null_mut(), code, w, l);
    }
    let Some(ctx) = CONTEXT.get() else {
        return CallNextHookEx(ptr::null_mut(), code, w, l);
    };
    let k = &*(l as *const KBDLLHOOKSTRUCT);
    if k.dwExtraInfo == TAG {
        return CallNextHookEx(ptr::null_mut(), code, w, l);
    }
    let scan = (k.scanCode as u16)
        | if k.flags & LLKHF_EXTENDED != 0 {
            0x100
        } else {
            0
        };
    let down = matches!(w as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
    if scan < 512 {
        ctx.held[scan as usize].store(down, Ordering::SeqCst);
    }
    let held = |s: usize| ctx.held[s].load(Ordering::SeqCst);
    if down
        && scan == ctx.gate.emergency_key.load(Ordering::SeqCst)
        && (held(0x1d) || held(0x11d))
        && (held(0x38) || held(0x138))
        && (held(0x2a) || held(0x36))
    {
        ctx.gate.stop();
        ctx.gate.push(NativeEvent::Emergency);
        return 1;
    }
    if down
        && scan == 0x14d
        && (held(0x1d) || held(0x11d))
        && (held(0x38) || held(0x138))
        && (held(0x2a) || held(0x36))
    {
        ctx.gate.push(NativeEvent::Switch);
        return 1;
    }
    let generation = ctx.gate.generation.load(Ordering::SeqCst);
    let captured = ctx.gate.capturing();
    let blocked = captured || (ctx.gate.active() && ctx.gate.block_local.load(Ordering::SeqCst));
    if scan < 512 && (!blocked || !down) {
        ctx.local_keys[scan as usize].store(down, Ordering::SeqCst);
    }
    ctx.gate.push(NativeEvent::Input {
        generation,
        event: Input::Key { scan, down },
        captured,
    });
    if captured
        || (ctx.gate.active()
            && (ctx.gate.block_local.load(Ordering::SeqCst)
                || (!down && scan < 512 && ctx.remote_keys[scan as usize].load(Ordering::SeqCst))))
    {
        1
    } else {
        CallNextHookEx(ptr::null_mut(), code, w, l)
    }
}
unsafe extern "system" fn mouse_hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(ptr::null_mut(), code, w, l);
    }
    let Some(ctx) = CONTEXT.get() else {
        return CallNextHookEx(ptr::null_mut(), code, w, l);
    };
    let m = &*(l as *const MSLLHOOKSTRUCT);
    if m.dwExtraInfo == TAG {
        if w as u32 == WM_MOUSEMOVE && ctx.gate.active() && !ctx.gate.capturing() {
            ctx.gate
                .push(NativeEvent::Position(m.pt.x as f64, m.pt.y as f64));
        }
        return CallNextHookEx(ptr::null_mut(), code, w, l);
    }
    let generation = ctx.gate.generation.load(Ordering::SeqCst);
    let remote = ctx.gate.capturing();
    let blocked = remote || (ctx.gate.active() && ctx.gate.block_local.load(Ordering::SeqCst));
    let message = w as u32;
    if message == WM_MOUSEMOVE {
        let px = ctx.x.swap(m.pt.x, Ordering::SeqCst);
        let py = ctx.y.swap(m.pt.y, Ordering::SeqCst);
        if remote {
            let ax = ctx.anchor_x.load(Ordering::SeqCst);
            let ay = ctx.anchor_y.load(Ordering::SeqCst);
            if m.pt.x != ax || m.pt.y != ay {
                ctx.gate.push(NativeEvent::Input {
                    generation,
                    captured: true,
                    event: Input::Motion {
                        dx: (m.pt.x - px) as f64,
                        dy: (m.pt.y - py) as f64,
                    },
                });
                ctx.x.store(ax, Ordering::SeqCst);
                ctx.y.store(ay, Ordering::SeqCst);
                SetCursorPos(ax, ay);
            }
        } else {
            if !blocked {
                ctx.remote_x
                    .store((m.pt.x as f64).to_bits(), Ordering::SeqCst);
                ctx.remote_y
                    .store((m.pt.y as f64).to_bits(), Ordering::SeqCst);
            }
            ctx.anchor_x.store(m.pt.x, Ordering::SeqCst);
            ctx.anchor_y.store(m.pt.y, Ordering::SeqCst);
            if !ctx.gate.active() {
                ctx.gate
                    .push(NativeEvent::Position(m.pt.x as f64, m.pt.y as f64));
            }
        }
    } else {
        let event = match message {
            WM_LBUTTONDOWN | WM_LBUTTONUP => Some(Input::Button {
                button: 0,
                down: message == WM_LBUTTONDOWN,
            }),
            WM_RBUTTONDOWN | WM_RBUTTONUP => Some(Input::Button {
                button: 1,
                down: message == WM_RBUTTONDOWN,
            }),
            WM_MBUTTONDOWN | WM_MBUTTONUP => Some(Input::Button {
                button: 2,
                down: message == WM_MBUTTONDOWN,
            }),
            WM_XBUTTONDOWN | WM_XBUTTONUP => Some(Input::Button {
                button: if m.mouseData >> 16 == 1 { 3 } else { 4 },
                down: message == WM_XBUTTONDOWN,
            }),
            WM_MOUSEWHEEL => Some(Input::Scroll {
                phase: 0,
                momentum: 0,
                x: 0.0,
                y: ((m.mouseData >> 16) as i16) as f64 / 3.0,
            }),
            WM_MOUSEHWHEEL => Some(Input::Scroll {
                phase: 0,
                momentum: 0,
                x: ((m.mouseData >> 16) as i16) as f64 / 3.0,
                y: 0.0,
            }),
            _ => None,
        };
        if let Some(event) = event {
            if let Input::Button { button, down } = &event {
                ctx.physical_buttons[*button as usize].store(*down, Ordering::SeqCst);
                if !blocked || !down {
                    ctx.local_buttons[*button as usize].store(*down, Ordering::SeqCst);
                }
                if !down
                    && ctx.gate.active()
                    && ctx.remote_buttons[*button as usize].load(Ordering::SeqCst)
                {
                    return 1;
                }
            }
            ctx.gate.push(NativeEvent::Input {
                generation,
                event,
                captured: remote,
            });
        }
    }
    if blocked {
        1
    } else {
        CallNextHookEx(ptr::null_mut(), code, w, l)
    }
}
pub fn displays(device_id: &str) -> Result<Vec<Display>> {
    unsafe extern "system" fn enumerate(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> windows_sys::core::BOOL {
        let output = &mut *(data as *mut Vec<Display>);
        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO) != 0 {
            let end = info.szDevice.iter().position(|c| *c == 0).unwrap_or(32);
            let name = String::from_utf16_lossy(&info.szDevice[..end]);
            let r = info.monitorInfo.rcMonitor;
            let mut dx = 96;
            let mut dy = 96;
            GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
            let mut device: DISPLAY_DEVICEW = std::mem::zeroed();
            device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            let id = if EnumDisplayDevicesW(
                info.szDevice.as_ptr(),
                0,
                &mut device,
                EDD_GET_DEVICE_INTERFACE_NAME,
            ) != 0
            {
                let end = device
                    .DeviceID
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(device.DeviceID.len());
                String::from_utf16_lossy(&device.DeviceID[..end])
            } else {
                name.clone()
            };
            let mut mode: DEVMODEW = std::mem::zeroed();
            mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            let rotation =
                if EnumDisplaySettingsW(info.szDevice.as_ptr(), ENUM_CURRENT_SETTINGS, &mut mode)
                    != 0
                {
                    mode.Anonymous1.Anonymous2.dmDisplayOrientation as f64 * 90.0
                } else {
                    0.0
                };
            output.push(Display {
                id,
                device_id: String::new(),
                name,
                number: output.len() as u32 + 1,
                rect: Rect {
                    x: r.left as f64,
                    y: r.top as f64,
                    width: (r.right - r.left) as f64,
                    height: (r.bottom - r.top) as f64,
                },
                pixel_width: (r.right - r.left) as u32,
                pixel_height: (r.bottom - r.top) as u32,
                scale: dx as f64 / 96.0,
                rotation,
                online: true,
            });
        }
        1
    }
    let mut output: Vec<Display> = Vec::new();
    unsafe {
        if EnumDisplayMonitors(
            ptr::null_mut(),
            ptr::null(),
            Some(enumerate),
            &mut output as *mut _ as LPARAM,
        ) == 0
        {
            return Err("Could not enumerate Windows displays".into());
        }
    }
    for display in &mut output {
        display.device_id = device_id.into();
    }
    Ok(output)
}
pub fn copy_text(text: &str) -> Result<()> {
    use std::io::Write;
    let mut child = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$input | Set-Clipboard",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
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

pub fn interface_kind(_name: &str, index: Option<u32>) -> String {
    unsafe {
        use windows_sys::Win32::NetworkManagement::IpHelper::*;
        let Some(index) = index else {
            return "LAN".into();
        };
        let mut row: MIB_IF_ROW2 = std::mem::zeroed();
        row.InterfaceIndex = index;
        if GetIfEntry2(&mut row) != 0 {
            return "LAN".into();
        }
        match row.Type {
            6 => "Ethernet",
            71 => "Wi-Fi",
            _ => "LAN",
        }
        .into()
    }
}
