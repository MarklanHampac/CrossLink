# CrossLink Architecture & Feasibility Report

## 1. System Overview

CrossLink is a low-latency, lightweight cross-device input sharing system connecting a Windows PC host to an Android target device over local network (Wi-Fi). It enables a single physical mouse and keyboard to control both the Windows desktop and an Android smartphone seamlessly.

```
       +------------------------------------+
       |         Windows Host (PC)          |
       |  - Rust Native Application         |
       |  - Win32 Low-Level Input Hooks     |
       |  - Screen Edge Detection Engine    |
       |  - Slint UI Dashboard              |
       +-----------------+------------------+
                         |
                Encrypted TCP / Wi-Fi
                         |
       +-----------------v------------------+
       |     Android Companion Device       |
       |  - Kotlin Native Application       |
       |  - AccessibilityService (Gestures) |
       |  - Overlay Cursor Renderer         |
       |  - Custom InputMethodService (IME) |
       +------------------------------------+
```

---

## 2. Windows Host Architecture (`crosslink-host`)

### Technology Selection
- **Language**: Rust 2021 edition.
- **GUI Engine**: Slint (native rendering, minimal memory footprint ~15MB, near 0% idle CPU).
- **Win32 Binding**: `windows-sys` crate.
- **Async Runtime**: `tokio` (minimal features: `net`, `sync`, `rt-multi-thread`, `time`).

### Core Modules
1. **Input Capture Engine (`hook.rs`)**:
   - `SetWindowsHookExW` registering `WH_MOUSE_LL` and `WH_KEYBOARD_LL`.
   - When cursor reaches configured display edge (e.g., right edge of primary screen):
     - Host locks Windows cursor to boundary edge or hides cursor (`ClipCursor`).
     - Mouse events are intercepted and converted into `CrossLinkInputPacket`.
     - Keypress events are intercepted when target device is active.
   - Escape key sequence or configurable hotkey (e.g., `Ctrl+Alt+S` or moving cursor back across return boundary) releases input hook and returns control to Windows immediately.

2. **Network Transceiver (`network.rs`)**:
   - mDNS service broad-caster (`mdns-sd`).
   - TCP client/server with TLS encryption (`tokio-rustls`).
   - Binary frame encoding using `bincode` / MessagePack for minimum latency (< 2ms serialization Overhead).

---

## 3. Android Companion Architecture (`crosslink-android`)

### Technology Selection
- **Language**: Kotlin 1.9+ / Android SDK 29+ (Android 10 to 15+).
- **UI Framework**: Jetpack Compose & Material 3.
- **Core System Services**:
  - `CrossLinkAccessibilityService` (`AccessibilityService`)
  - `CrossLinkIME` (`InputMethodService`)
  - `OverlayCursorView` (`WindowManager` with `TYPE_APPLICATION_OVERLAY`)

### Input Injection Feasibility & API Analysis

| Input Feature | Android API / Mechanism | Platform Restrictions & Handling |
| :--- | :--- | :--- |
| **Pointer Position** | `TYPE_APPLICATION_OVERLAY` floating view updating `WindowManager.LayoutParams.x/y` | System Alert Window permission required (`ACTION_MANAGE_OVERLAY_PERMISSION`). Overlay cannot intercept touches itself; touch events are dispatched underneath. |
| **Mouse Click / Tap** | `AccessibilityService.dispatchGesture()` with single-point `Path` | Android 7.0+ (API 24+). Works across all non-protected apps. Protected banking apps (`FLAG_SECURE` / `filterTouchesWhenObscured`) ignore overlay touch injection. |
| **Drag & Hold** | `dispatchGesture()` with multi-segment `Path` and duration control | Supported for continuous move paths. |
| **Scroll Wheel** | `dispatchGesture()` swipe gesture OR `AccessibilityNodeInfo.performAction(ACTION_SCROLL_FORWARD/BACKWARD)` | Simulates finger swipe vector based on scroll delta. |
| **Text Typing** | Custom `InputMethodService` (IME) forwarding characters to `InputConnection.commitText()` | User must enable `CrossLink Keyboard` in system settings and select it when typing in target applications. |
| **System Keys (Home/Back/Recents)** | `AccessibilityService.performGlobalAction()` | Supports `GLOBAL_ACTION_BACK`, `GLOBAL_ACTION_HOME`, `GLOBAL_ACTION_RECENTS`. |
| **Native Pointer (ADB Mode)** | `adb shell input motionevent` / `/dev/input/event*` injection via ADB Wi-Fi | Optional advanced mode. Provides hardware-level mouse cursor without accessibility overlay latency. |

---

## 4. Cryptographic Protocol & Device Pairing

1. **Discovery**: Windows host broadcasts `_crosslink._tcp.local.` via mDNS. Android companion scans for local hosts.
2. **Pairing Handshake**:
   - Android app displays a QR code containing `HostIP:Port`, ECDH Public Key, and Ephemeral Session Token.
   - Windows app scans QR code or accepts manual IP + PIN entry.
   - Mutual ECDH key exchange establishes AES-256-GCM encrypted socket session.
3. **Packet Protocol Structure** (Header + Payload):
   - `Header (4 bytes)`: Packet Magic + Event Type ID + Payload Length
   - `Event Types`:
     - `0x01` Pointer Move (`dx`, `dy`, `abs_x`, `abs_y`)
     - `0x02` Pointer Click (`button`: Left/Right/Middle, `action`: Down/Up)
     - `0x03` Pointer Scroll (`delta_x`, `delta_y`)
     - `0x04` Key Event (`key_code`, `modifiers`, `unicode_char`)
     - `0x05` Heartbeat / Ping-Pong
     - `0x06` State Switch Request / Hotkey Release

---

## 5. Directory Structure Setup

```
F:\CrossLink\
├── PROJECT_STATE.md
├── ARCHITECTURE.md
├── host/                    # Windows Rust Application
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── input/
│       │   ├── hook.rs
│       │   └── boundary.rs
│       ├── net/
│       │   ├── discovery.rs
│       │   └── protocol.rs
│       └── ui/
│           └── app.slint
└── android/                 # Android Kotlin Application
    ├── build.gradle.kts
    ├── settings.gradle.kts
    └── app/
        ├── build.gradle.kts
        └── src/main/java/com/crosslink/app/
            ├── MainActivity.kt
            ├── service/
            │   ├── CrossLinkAccessibilityService.kt
            │   └── CrossLinkIME.kt
            ├── net/
            │   └── InputReceiverServer.kt
            └── ui/
                └── CursorOverlay.kt
```
