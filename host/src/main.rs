#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod discovery;
mod network;
mod protocol;
mod security;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use slint::ComponentHandle;
use windows_sys::Win32::Foundation::{LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, ToUnicode, VK_LCONTROL, VK_LMENU, VK_LSHIFT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetSystemMetrics, PeekMessageW, SetWindowsHookExW,
    TranslateMessage, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEWHEEL,
};

use network::{connect_to_android, disconnect, is_connected, send_packet, start_auto_reconnect};
use protocol::InputPacket;

pub static TARGET_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static SCREEN_EDGE_THRESHOLD_PX: Mutex<i32> = Mutex::new(5);
pub static TARGET_IP: Mutex<String> = Mutex::new(String::new());
pub static CONNECTION_STATUS: Mutex<String> = Mutex::new(String::new());
pub static ACTIVE_DEVICE: Mutex<String> = Mutex::new(String::new());
pub static NOTICE: Mutex<String> = Mutex::new(String::new());
pub static NOTICE_ERROR: AtomicBool = AtomicBool::new(false);
pub static BUSY: AtomicBool = AtomicBool::new(false);
pub static BUSY_TEXT: Mutex<String> = Mutex::new(String::new());
pub static LAST_ACTION: Mutex<String> = Mutex::new(String::new());
pub static LAST_ACTION_AT: LazyLock<Mutex<std::time::Instant>> = LazyLock::new(|| Mutex::new(std::time::Instant::now()));
pub static LOG_LINES: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

slint::include_modules!();

fn now_ts() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let s = secs % 86400;
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}
pub(crate) fn log_event(msg: &str) {
    let line = format!("[{}] {}", now_ts(), msg);
    eprintln!("{}", line);
    if let Ok(mut lines) = LOG_LINES.lock() {
        lines.push(line);
        if lines.len() > 400 {
            let drain = lines.len() - 400;
            lines.drain(0..drain);
        }
    }
}

pub(crate) fn set_status(s: &str) {
    if let Ok(mut g) = CONNECTION_STATUS.lock() { *g = s.to_string(); }
    log_event(&format!("Status -> {}", s));
}
fn set_device(d: &str) {
    if let Ok(mut g) = ACTIVE_DEVICE.lock() { *g = d.to_string(); }
    log_event(&format!("Active device -> {}", d));
}
pub(crate) fn set_notice(msg: &str, is_error: bool) {
    if let Ok(mut n) = NOTICE.lock() { *n = msg.to_string(); }
    NOTICE_ERROR.store(is_error, Ordering::Relaxed);
    log_event(&format!("{}: {}", if is_error { "ERROR" } else { "INFO" }, msg));
}
fn set_busy(busy: bool, text: &str) {
    BUSY.store(busy, Ordering::Relaxed);
    if let Ok(mut t) = BUSY_TEXT.lock() { *t = text.to_string(); }
    if busy && !text.is_empty() { log_event(&format!("Busy -> {}", text)); }
}
fn clear_busy() { set_busy(false, ""); }
fn register_action(action: &str) {
    if let Ok(mut a) = LAST_ACTION.lock() { *a = action.to_string(); }
    if let Ok(mut t) = LAST_ACTION_AT.lock() { *t = std::time::Instant::now(); }
    log_event(&format!("Action: {}", action));
}

fn switch_to_android() {
    TARGET_ACTIVE.store(true, Ordering::Relaxed);
    set_device("Android");
    set_notice("Switched control to Android. Press Esc to return.", false);
}
fn switch_to_windows() {
    TARGET_ACTIVE.store(false, Ordering::Relaxed);
    let _ = send_packet(InputPacket::ReturnControl);
    set_device("Windows");
    set_notice("Control returned to Windows.", false);
}

unsafe extern "system" fn mouse_hook_proc(n_code: i32, w_param: WPARAM, l_param: isize) -> LRESULT {
    if n_code >= 0 {
        let m = &*(l_param as *const MSLLHOOKSTRUCT);
        let pt = m.pt;
        let sw = GetSystemMetrics(0);
        let sh = GetSystemMetrics(1);
        let threshold = *SCREEN_EDGE_THRESHOLD_PX.lock().unwrap_or_else(|e| e.into_inner());

        if TARGET_ACTIVE.load(Ordering::Relaxed) {
            match w_param as u32 {
                WM_MOUSEWHEEL => {
                    let delta = ((m.mouseData >> 16) as i16) as i32;
                    let _ = send_packet(InputPacket::MouseScroll { delta_x: 0, delta_y: delta / 2 });
                }
                WM_LBUTTONDOWN => { let _ = send_packet(InputPacket::MouseClick { button: "left".into(), action: "down".into(), x: pt.x, y: pt.y }); }
                WM_LBUTTONUP => { let _ = send_packet(InputPacket::MouseClick { button: "left".into(), action: "up".into(), x: pt.x, y: pt.y }); }
                WM_RBUTTONDOWN => { let _ = send_packet(InputPacket::MouseClick { button: "right".into(), action: "down".into(), x: pt.x, y: pt.y }); }
                WM_RBUTTONUP => { let _ = send_packet(InputPacket::MouseClick { button: "right".into(), action: "up".into(), x: pt.x, y: pt.y }); }
                WM_MBUTTONDOWN => { let _ = send_packet(InputPacket::MouseClick { button: "middle".into(), action: "down".into(), x: pt.x, y: pt.y }); }
                WM_MBUTTONUP => { let _ = send_packet(InputPacket::MouseClick { button: "middle".into(), action: "up".into(), x: pt.x, y: pt.y }); }
                _ => {
                    let _ = send_packet(InputPacket::MouseMove { x: pt.x, y: pt.y, dx: 0, dy: 0, screen_width: sw, screen_height: sh });
                }
            }
            return 1;
        }

        if pt.x >= sw - threshold && is_connected() {
            switch_to_android();
            return 1;
        }
    }
    unsafe { CallNextHookEx(0, n_code, w_param, l_param) }
}

unsafe extern "system" fn keyboard_hook_proc(n_code: i32, w_param: WPARAM, l_param: isize) -> LRESULT {
    if n_code >= 0 {
        let kb = &*(l_param as *const KBDLLHOOKSTRUCT);
        let is_down = (kb.flags & 0x80) == 0;

        if kb.vkCode == 0x1B && is_down && TARGET_ACTIVE.load(Ordering::Relaxed) {
            switch_to_windows();
            return 1;
        }

        if TARGET_ACTIVE.load(Ordering::Relaxed) {
            let mut unicode_char: Option<char> = None;
            if is_down {
                let mut buf: [u16; 2] = [0; 2];
                let state: [u8; 256] = [0; 256];
                let len = unsafe { ToUnicode(kb.vkCode, kb.scanCode, state.as_ptr(), buf.as_mut_ptr(), 2, 0) };
                if len > 0 { unicode_char = char::from_u32(buf[0] as u32); }
            }
            let _ = send_packet(InputPacket::KeyEvent {
                vk_code: kb.vkCode,
                scan_code: kb.scanCode,
                action: if is_down { "down".into() } else { "up".into() },
                unicode_char,
                ctrl: unsafe { GetAsyncKeyState(VK_LCONTROL as i32) } < 0,
                alt: unsafe { GetAsyncKeyState(VK_LMENU as i32) } < 0,
                shift: unsafe { GetAsyncKeyState(VK_LSHIFT as i32) } < 0,
                meta: false,
            });
            return 1;
        }
    }
    unsafe { CallNextHookEx(0, n_code, w_param, l_param) }
}

fn main() {
    eprintln!("CrossLink Host starting...");
    set_status("Disconnected");
    set_device("Windows");
    set_notice("Ready. Enter phone IP or click Discover, then Connect.", false);

    let app = App::new().expect("Failed to init Slint");
    let weak = app.as_weak();

    app.on_target_ip_changed(|ip| { if let Ok(mut t) = TARGET_IP.lock() { *t = ip.to_string(); } });

    app.on_clear_log(move || {
        register_action("Clear Log clicked");
        if let Ok(mut lines) = LOG_LINES.lock() { lines.clear(); }
        log_event("Log cleared.");
    });

    app.on_switch_device({
        let w = weak.clone();
        move || {
            if TARGET_ACTIVE.load(Ordering::Relaxed) {
                register_action("Return to PC clicked");
                switch_to_windows();
            } else if is_connected() {
                register_action("Switch to Android clicked");
                switch_to_android();
            } else {
                register_action("Switch to Android clicked");
                set_notice("Cannot switch: not connected to Android.", true);
            }
            if let Some(a) = w.upgrade() { a.set_is_target_active(TARGET_ACTIVE.load(Ordering::Relaxed)); }
        }
    });

    app.on_discover(move || {
        register_action("Discover clicked");
        set_busy(true, "Discovering devices on LAN (UDP 8889)...");
        set_status("Discovering...");
        set_notice("Searching for CrossLink on local network...", false);
        std::thread::spawn(move || {
            if let Some(ip) = discovery::discover_android(1500) {
                if let Ok(mut t) = TARGET_IP.lock() { *t = ip.clone(); }
                set_status("Device found");
                set_notice(&format!("Found device at {}. Click Connect.", ip), false);
            } else {
                set_status("Not found");
                set_notice("No device found via UDP discovery. Ensure phone app is open/listening, same Wi-Fi, and UDP 8889 allowed; or enter IP manually.", true);
            }
            clear_busy();
        });
    });

    app.on_connect(move || {
        register_action("Connect clicked");
        let ipraw = TARGET_IP.lock().unwrap().clone();
        let (ip, port) = match ipraw.rsplit_once(':') {
            Some((i, p)) => (i.trim().to_string(), p.parse().unwrap_or(8888u16)),
            None => (ipraw.trim().to_string(), 8888u16),
        };
        if ip.is_empty() || ip == "0.0.0.0" {
            set_notice("Enter a valid phone IP address first (e.g. 192.168.1.50:8888).", true);
            return;
        }
        set_busy(true, &format!("Connecting to {}:{}...", ip, port));
        set_status("Connecting...");
        set_notice(&format!("Connecting to {}:{} (TCP)...", ip, port), false);
        std::thread::spawn(move || {
            match connect_to_android(&ip, port) {
                Ok(()) => set_notice(&format!("Connected to {}:{}.", ip, port), false),
                Err(e) => {
                    set_status("Connection Failed");
                    set_notice(&format!("Connection failed: {}. Check same Wi-Fi, phone app listening, IP/port, and firewall (TCP 8888).", e), true);
                }
            }
            clear_busy();
        });
    });

    app.on_disconnect(move || {
        register_action("Disconnect clicked");
        set_busy(true, "Disconnecting...");
        disconnect();
        switch_to_windows();
        set_status("Disconnected");
        set_notice("Disconnected from Android device.", false);
        clear_busy();
    });

    app.on_quit(|| { register_action("Exit clicked"); disconnect(); std::process::exit(0); });

    let w = weak.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(150));
        if let Some(app) = w.upgrade() {
            app.set_connection_status(CONNECTION_STATUS.lock().unwrap().as_str().into());
            app.set_active_device(ACTIVE_DEVICE.lock().unwrap().as_str().into());
            app.set_target_ip(TARGET_IP.lock().unwrap().as_str().into());
            app.set_is_target_active(TARGET_ACTIVE.load(Ordering::Relaxed));
            app.set_busy(BUSY.load(Ordering::Relaxed));
            app.set_busy_text(BUSY_TEXT.lock().unwrap().as_str().into());
            app.set_notice(NOTICE.lock().unwrap().as_str().into());
            app.set_notice_error(NOTICE_ERROR.load(Ordering::Relaxed));
            let last_action = LAST_ACTION.lock().unwrap().clone();
            app.set_last_action(last_action.as_str().into());
            let pulse = LAST_ACTION_AT
                .lock()
                .map(|t| t.elapsed() < std::time::Duration::from_millis(1200))
                .unwrap_or(false);
            app.set_last_action_pulse(pulse);
            let log_text = LOG_LINES.lock().map(|v| v.join("\n")).unwrap_or_default();
            app.set_log_text(log_text.as_str().into());
        }
    });

    start_auto_reconnect();

    std::thread::spawn(|| unsafe {
        let hmod = GetModuleHandleW(std::ptr::null());
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), hmod, 0);
        let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), hmod, 0);
        if mouse == 0 || kb == 0 { eprintln!("Hook installation failed"); return; }
        let mut msg: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut msg, 0, 0, 0, 0x0001) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });

    app.run().expect("UI error");
}
