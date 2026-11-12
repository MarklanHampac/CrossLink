use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MouseAction {
    Down,
    Up,
    Click,
    DoubleClick,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyAction {
    Down,
    Up,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputPacket {
    /// Absolute cursor position (normalized 0.0 - 1.0 or pixel coordinates)
    MouseMove {
        x: i32,
        y: i32,
        dx: i32,
        dy: i32,
        screen_width: i32,
        screen_height: i32,
    },
    /// Mouse click event
    MouseClick {
        button: MouseButton,
        action: MouseAction,
        x: i32,
        y: i32,
    },
    /// Scroll event (vertical/horizontal delta)
    MouseScroll {
        delta_x: i32,
        delta_y: i32,
    },
    /// Keyboard key event
    KeyEvent {
        vk_code: u32,
        scan_code: u32,
        action: KeyAction,
        unicode_char: Option<char>,
        ctrl: bool,
        alt: bool,
        shift: bool,
        meta: bool,
    },
    /// Text insertion shortcut
    TextCommit {
        text: String,
    },
    /// Handshake request
    Handshake {
        device_name: String,
        version: String,
        pin: String,
    },
    /// Handshake response
    HandshakeResponse {
        accepted: bool,
        device_name: String,
        message: String,
    },
    /// Ping / Heartbeat packet
    Heartbeat {
        timestamp: u64,
    },
    /// Switch input focus back to Windows host
    ReturnControl,
}

impl InputPacket {
    pub fn serialize(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}