mod network;
mod protocol;
mod security;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use slint::ComponentHandle;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetSystemMetrics, PeekMessageW,
    TranslateMessage, MSG, WH_MOUSE_LL, WH_KEYBOARD_LL, MOUSEHOOKSTRUCT, KBDLLHOOKSTRUCT,
    SetWindowsHookExW, UnhookWindowsHookEx,
};
use windows_sys::Win32::Foundation::{LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    ToUnicode, VK_LCONTROL, VK_LSHIFT, VK_LMENU, GetAsyncKeyState,
};

use network::{send_packet, connect_to_android, disconnect, is_connected, start_auto_reconnect};
use protocol::{InputPacket, KeyAction};

pub static TARGET_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static SCREEN_EDGE_THRESHOLD_PX: Mutex<i32> = Mutex::new(5);
pub static TARGET_IP: Mutex<String> = Mutex::new(String::new());
pub static CONNECTION_STATUS: Mutex<String> = Mutex::new(String::new());
pub static ACTIVE_DEVICE: Mutex<String> = Mutex::new(String::new());

slint::include_modules!();

fn set_status(status: &str) {
    if let Ok(mut s) = CONNECTION_STATUS.lock() {
        *s = status.to_string();
    }
}

fn set_device(device: &str) {
    if let Ok(mut d) = ACTIVE_DEVICE.lock() {
        *d = device.to_string();
    }
}

fn switch_to_android() {
    TARGET_ACTIVE.store(true, Ordering::Relaxed);
    set_device("Android");
    eprintln!("[CrossLink] Switching to Android");
}

fn switch_to_windows() {
    TARGET_ACTIVE.store(false, Ordering::Relaxed);
    let _ = send_packet(InputPacket::ReturnControl);
    set_device("Windows");
    eprintln!("[CrossLink] Returning to Windows");
}

unsafe extern "system" fn mouse_hook_proc(n_code: i32, w_param: WPARAM, l_param: isize) -> LRESULT {
    if n_code >= 0 {
        let mouse_msg = &*(l_param as *const MOUSEHOOKSTRUCT);
        let pt = mouse_msg.pt;
        let screen_width = GetSystemMetrics(0);
        let threshold = *SCREEN_EDGE_THRESHOLD_PX.lock().unwrap_or_else(|e| e.into_inner());

        if TARGET_ACTIVE.load(Ordering::Relaxed) {
            let _ = send_packet(InputPacket::MouseMove {
                x: pt.x,
                y: pt.y,
                dx: 0,
                dy: 0,
                screen_width,
                screen_height: GetSystemMetrics(1),
            });
            return 1;
        }

        if pt.x >= screen_width - threshold {
            switch_to_android();
            return 1;
        }
    }
    unsafe { CallNextHookEx(0, n_code, w_param, l_param) }
}

unsafe extern "system" fn keyboard_hook_proc(n_code: i32, w_param: WPARAM, l_param: isize) -> LRESULT {
    if n_code >= 0 {
        let kb = &*(l_param as *const KBDLLHOOKSTRUCT);

        if kb.vkCode == 0x1B && TARGET_ACTIVE.load(Ordering::Relaxed) {
            switch_to_windows();
            return 1;
        }

        if TARGET_ACTIVE.load(Ordering::Relaxed) {
            let is_down = (kb.flags & 0x80) == 0;
            let mut unicode_char: Option<char> = None;
            if is_down {
                let mut buf: [u16; 2] = [0; 2];
                let state: [u8; 256] = [0; 256];
                let len = ToUnicode(
                    kb.vkCode as u32,
                    kb.scanCode as u32,
                    state.as_ptr(),
                    buf.as_mut_ptr(),
                    2,
                    0,
                );
                if len > 0 {
                    unicode_char = char::from_u32(buf[0] as u32);
                }
            }
            let _ = send_packet(InputPacket::KeyEvent {
                vk_code: kb.vkCode as u32,
                scan_code: kb.scanCode as u32,
                action: if is_down { KeyAction::Down } else { KeyAction::Up },
                unicode_char,
                ctrl: GetAsyncKeyState(VK_LCONTROL as i32) < 0,
                alt: GetAsyncKeyState(VK_LMENU as i32) < 0,
                shift: GetAsyncKeyState(VK_LSHIFT as i32) < 0,
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

    let app = App::new().expect("Failed to init Slint");
    let weak = app.as_weak();

    app.on_switch_device({
        let w = weak.clone();
        move || {
            if TARGET_ACTIVE.load(Ordering::Relaxed) {
                switch_to_windows();
            } else if is_connected() {
                switch_to_android();
            } else {
                set_status("Not Paired");
            }
            if let Some(a) = w.upgrade() {
                a.set_is_target_active(TARGET_ACTIVE.load(Ordering::Relaxed));
            }
        }
    });

    app.on_connect({
        move || {
            set_status("Connecting...");
            let (ip, port) = {
                let t = TARGET_IP.lock().unwrap().clone();
                match t.rsplit_once(':') {
                    Some((i, p)) => (i.to_string(), p.parse().unwrap_or(8888u16)),
                    None => ("192.168.1.100".to_string(), 8888u16),
                }
            };
            if let Err(e) = connect_to_android(&ip, port) {
                eprintln!("Connect error: {}", e);
                set_status("Connection Failed");
            }
        }
    });

    app.on_disconnect(|| {
        disconnect();
        switch_to_windows();
        set_status("Disconnected");
    });

    app.on_settings(|| {
        eprintln!("[CrossLink] Settings not yet implemented");
    });

    app.on_quit(|| {
        disconnect();
        std::process::exit(0);
    });

    let w = weak.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if let Some(app) = w.upgrade() {
            app.set_connection_status(CONNECTION_STATUS.lock().unwrap().as_str().into());
            app.set_active_device(ACTIVE_DEVICE.lock().unwrap().as_str().into());
            app.set_target_ip(TARGET_IP.lock().unwrap().as_str().into());
            app.set_is_target_active(TARGET_ACTIVE.load(Ordering::Relaxed));
        }
    });

    start_auto_reconnect();

    let _hook = std::thread::spawn(|| unsafe {
        let hmod = GetModuleHandleW(std::ptr::null());
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), hmod, 0);
        let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), hmod, 0);
        if mouse == 0 || kb == 0 {
            eprintln!("Hook installation failed");
            return;
        }
        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, 0, 0, 0, 0x0001) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        UnhookWindowsHookEx(mouse);
        UnhookWindowsHookEx(kb);
    });

    app.run().expect("UI error");
}