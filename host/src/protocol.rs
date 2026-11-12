use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum InputPacket {
    MouseMove { x: i32, y: i32, dx: i32, dy: i32, screen_width: i32, screen_height: i32 },
    MouseClick { button: String, action: String, x: i32, y: i32 },
    MouseScroll { delta_x: i32, delta_y: i32 },
    KeyEvent { vk_code: u32, scan_code: u32, action: String, unicode_char: Option<char>, ctrl: bool, alt: bool, shift: bool, meta: bool },
    TextCommit { text: String },
    Handshake { device_name: String, version: String },
    HandshakeResponse { accepted: bool, device_name: String, message: String },
    Heartbeat { timestamp: u64 },
    ReturnControl,
}

impl InputPacket {
    pub fn serialize(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}
