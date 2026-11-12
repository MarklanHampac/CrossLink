use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::protocol::InputPacket;

const MAGIC: [u8; 8] = *b"CRLKPAIR";
const PROTO_VERSION: u16 = 1;
const MAX_FRAME_LEN: u32 = 64 * 1024;

static CIPHER: OnceLock<Mutex<Option<Aes256Gcm>>> = OnceLock::new();
static WRITE_STREAM: OnceLock<Mutex<Option<TcpStream>>> = OnceLock::new();
static CONNECTED: AtomicBool = AtomicBool::new(false);
static LAST_TARGET: OnceLock<Mutex<String>> = OnceLock::new();

fn cipher_store() -> &'static Mutex<Option<Aes256Gcm>> {
    CIPHER.get_or_init(|| Mutex::new(None))
}

fn write_stream() -> &'static Mutex<Option<TcpStream>> {
    WRITE_STREAM.get_or_init(|| Mutex::new(None))
}

fn last_target() -> &'static Mutex<String> {
    LAST_TARGET.get_or_init(|| Mutex::new(String::new()))
}

fn set_connected(connected: bool, peer_ip: Option<String>) {
    CONNECTED.store(connected, Ordering::Relaxed);
    let display_ip = peer_ip.as_ref().map(|s| s.as_str()).unwrap_or("local");
    if let Ok(mut status) = crate::CONNECTION_STATUS.lock() {
        *status = if connected { format!("Connected ({})", display_ip) } else { "Disconnected".to_string() };
    }
    if let Ok(mut ip) = crate::TARGET_IP.lock() {
        *ip = peer_ip.unwrap_or_default();
    }
}

fn read_exact<R: Read>(mut stream: R, buf: &mut [u8]) -> Result<(), String> {
    let mut pos = 0;
    while pos < buf.len() {
        let n = stream.read(&mut buf[pos..]).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("peer closed".to_string());
        }
        pos += n;
    }
    Ok(())
}

fn encrypt_frame(cipher: &Aes256Gcm, plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ct = cipher.encrypt(Nonce::from_slice(&nonce_bytes), plain)
        .map_err(|e| format!("encrypt error: {:?}", e))?;
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn decrypt_frame(cipher: &Aes256Gcm, frame: &[u8]) -> Result<Vec<u8>, String> {
    if frame.len() < 28 {
        return Err("frame too short".to_string());
    }
    cipher.decrypt(Nonce::from_slice(&frame[..12]), &frame[12..])
        .map_err(|e| format!("decrypt error: {:?}", e))
}

pub fn send_packet(packet: InputPacket) -> Result<(), String> {
    let plain = bincode::serialize(&packet).map_err(|e| format!("serialize: {:?}", e))?;
    let frame = {
        let c = cipher_store().lock().map_err(|e| e.to_string())?;
        let c = c.as_ref().ok_or_else(|| "Not paired".to_string())?;
        encrypt_frame(c, &plain)?
    };
    if frame.len() as u32 > MAX_FRAME_LEN {
        return Err("Frame too large".to_string());
    }
    let mut payload = Vec::with_capacity(4 + frame.len());
    payload.extend_from_slice(&(frame.len() as u32).to_le_bytes());
    payload.extend_from_slice(&frame);
    let s = write_stream().lock().map_err(|e| e.to_string())?;
    let s = s.as_ref().ok_or_else(|| "Not connected".to_string())?;
    let mut guard = s.try_clone().map_err(|e| e.to_string())?;
    drop(s);
    guard.write_all(&payload).and_then(|_| guard.flush()).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_packet(stream: &mut TcpStream, cipher: &Aes256Gcm) -> Result<InputPacket, String> {
    let mut len_buf = [0u8; 4];
    read_exact(&mut *stream, &mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > MAX_FRAME_LEN as usize {
        return Err(format!("Bad length: {}", len));
    }
    let mut frame = vec![0u8; len];
    read_exact(&mut *stream, &mut frame)?;
    let plain = decrypt_frame(cipher, &frame)?;
    bincode::deserialize(&plain).map_err(|e| format!("deser: {:?}", e))
}

pub fn connect_to_android(ip: &str, port: u16) -> Result<(), String> {
    let mut stream = TcpStream::connect((ip, port)).map_err(|e| e.to_string())?;
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).map_err(|e| e.to_string())?;

    // Generate ECDH-like keypair
    let (_my_priv, my_pub) = crate::security::generate_shared_secret();
    let client_pub = crate::security::export_public_key(&my_pub);

    let client_name = "CrossLink-Host";
    let pin = std::env::var("CROSSLINK_PIN").unwrap_or_default();
    let mut hello = Vec::new();
    hello.extend_from_slice(&MAGIC);
    hello.extend_from_slice(&PROTO_VERSION.to_le_bytes());
    hello.extend_from_slice(&(client_name.len() as u16).to_le_bytes());
    hello.extend_from_slice(client_name.as_bytes());
    hello.extend_from_slice(&(pin.len() as u16).to_le_bytes());
    hello.extend_from_slice(pin.as_bytes());
    hello.extend_from_slice(&(client_pub.len() as u16).to_le_bytes());
    hello.extend_from_slice(client_pub.as_bytes());
    stream.write_all(&hello).and_then(|_| stream.flush()).map_err(|e| e.to_string())?;

    // Read reply
    read_exact(&mut stream, &mut [0u8; 8])?; // magic
    read_exact(&mut stream, &mut [0u8; 2])?; // version
    let mut name_len = [0u8; 2];
    read_exact(&mut stream, &mut name_len)?;
    let name_len = u16::from_le_bytes(name_len) as usize;
    let mut name = vec![0u8; name_len.min(64)];
    read_exact(&mut stream, &mut name)?;
    let mut pub_len = [0u8; 2];
    read_exact(&mut stream, &mut pub_len)?;
    let pub_len = u16::from_le_bytes(pub_len) as usize;
    let mut peer_pub = vec![0u8; pub_len.min(32)];
    read_exact(&mut stream, &mut peer_pub)?;
    let device_name: String = String::from_utf8(name).unwrap_or_default();

    // Compute shared secret
    let peer_bytes: &[u8] = &peer_pub;
    let my_bytes: &[u8] = &my_pub;
    let shared = Sha256::digest(&[my_bytes, peer_bytes].concat()).to_vec();

    let cipher = crate::security::derive_cipher(&shared)?;

    *cipher_store().lock().unwrap() = Some(cipher.clone());
    *write_stream().lock().unwrap() = Some(stream.try_clone().map_err(|e| e.to_string())?);

    send_packet(InputPacket::HandshakeResponse {
        accepted: true,
        device_name: "CrossLink-Host".to_string(),
        message: format!("paired with {}", device_name),
    })?;

    stream.set_read_timeout(None).map_err(|e| e.to_string())?;
    let target = format!("{}:{}", ip, port);
    *last_target().lock().unwrap() = target.clone();
    set_connected(true, Some(target.clone()));
    eprintln!("CrossLink paired with: {}", device_name);

    thread::spawn(move || receive_loop(stream, &cipher, target));
    Ok(())
}

fn receive_loop(mut stream: TcpStream, cipher: &Aes256Gcm, target: String) {
    let mut last_recv = Instant::now();
    let mut last_hb = Instant::now();
    loop {
        if last_recv.elapsed() > Duration::from_secs(10) {
            eprintln!("Peer {} timeout", target);
            break;
        }
        if last_hb.elapsed() > Duration::from_secs(2) {
            let _ = send_packet(InputPacket::Heartbeat { timestamp: 0 });
            last_hb = Instant::now();
        }
        match read_packet(&mut stream, cipher) {
            Ok(p) => {
                last_recv = Instant::now();
                if let InputPacket::ReturnControl = p {
                    crate::TARGET_ACTIVE.store(false, Ordering::Relaxed);
                    if let Ok(mut d) = crate::ACTIVE_DEVICE.lock() {
                        *d = "Windows".to_string();
                    }
                }
            }
            Err(e) => {
                eprintln!("Recv error: {}", e);
                break;
            }
        }
    }
    set_connected(false, None);
}

pub fn disconnect() {
    *write_stream().lock().unwrap() = None;
    *cipher_store().lock().unwrap() = None;
    set_connected(false, None);
}

pub fn is_connected() -> bool {
    CONNECTED.load(Ordering::Relaxed)
}

pub fn start_auto_reconnect() {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(5));
            if is_connected() {
                continue;
            }
            let t = last_target().lock().unwrap().clone();
            if t.is_empty() {
                continue;
            }
            if let Some((ip, port)) = t.rsplit_once(':') {
                if let Ok(p) = port.parse::<u16>() {
                    eprintln!("Reconnecting to {}...", t);
                    let _ = connect_to_android(ip, p);
                }
            }
        }
    });
}