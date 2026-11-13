use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use aes_gcm::aead::Aead;
use aes_gcm::Aes256Gcm;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::protocol::InputPacket;

const MAGIC: [u8; 4] = *b"CRLK";
const PROTO_VERSION: u8 = 1;
const MAX_FRAME_LEN: usize = 64 * 1024;

static CIPHER: OnceLock<Mutex<Option<Aes256Gcm>>> = OnceLock::new();
static WRITE_STREAM: OnceLock<Mutex<Option<TcpStream>>> = OnceLock::new();
static CONNECTED: AtomicBool = AtomicBool::new(false);
static LAST_TARGET: OnceLock<Mutex<String>> = OnceLock::new();

fn cipher_store() -> &'static Mutex<Option<Aes256Gcm>> { CIPHER.get_or_init(|| Mutex::new(None)) }
fn write_stream() -> &'static Mutex<Option<TcpStream>> { WRITE_STREAM.get_or_init(|| Mutex::new(None)) }
fn last_target() -> &'static Mutex<String> { LAST_TARGET.get_or_init(|| Mutex::new(String::new())) }

fn set_connected(connected: bool, peer_ip: Option<String>) {
    CONNECTED.store(connected, Ordering::Relaxed);
    let display_ip = peer_ip.clone().unwrap_or_else(|| "local".to_string());
    if let Ok(mut status) = crate::CONNECTION_STATUS.lock() {
        *status = if connected { format!("Connected ({})", display_ip) } else { "Disconnected".to_string() };
    }
    if let Ok(mut ip) = crate::TARGET_IP.lock() {
        if let Some(p) = peer_ip { *ip = p; }
    }
}

fn read_exact<R: Read>(stream: &mut R, buf: &mut [u8]) -> Result<(), String> {
    let mut pos = 0;
    while pos < buf.len() {
        let n = stream.read(&mut buf[pos..]).map_err(|e| format!("read failed: {}", e))?;
        if n == 0 { return Err("peer closed connection".to_string()); }
        pos += n;
    }
    Ok(())
}

fn derive_key(my_pub: &[u8; 32], peer_pub: &[u8; 32]) -> Result<Aes256Gcm, String> {
    let mut combined = Vec::with_capacity(64);
    if my_pub <= peer_pub { combined.extend_from_slice(my_pub); combined.extend_from_slice(peer_pub); }
    else { combined.extend_from_slice(peer_pub); combined.extend_from_slice(my_pub); }
    let key = Sha256::digest(&combined);
    crate::security::derive_cipher(&key)
}

fn encrypt_frame(cipher: &Aes256Gcm, plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    let ct = cipher.encrypt(aes_gcm::Nonce::from_slice(&nonce), plain)
        .map_err(|e| format!("encrypt: {:?}", e))?;
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn decrypt_frame(cipher: &Aes256Gcm, frame: &[u8]) -> Result<Vec<u8>, String> {
    if frame.len() < 28 { return Err("frame too short".to_string()); }
    cipher.decrypt(aes_gcm::Nonce::from_slice(&frame[..12]), &frame[12..])
        .map_err(|e| format!("decrypt: {:?}", e))
}

pub fn send_packet(packet: InputPacket) -> Result<(), String> {
    let plain = packet.serialize().map_err(|e| format!("serialize: {:?}", e))?;
    let frame = {
        let c = cipher_store().lock().map_err(|e| e.to_string())?;
        let c = c.as_ref().ok_or_else(|| "Not paired/encrypted".to_string())?;
        encrypt_frame(c, &plain)?
    };
    if frame.len() > MAX_FRAME_LEN { return Err("Frame too large".to_string()); }
    let mut payload = Vec::with_capacity(4 + frame.len());
    payload.extend_from_slice(&(frame.len() as u32).to_le_bytes());
    payload.extend_from_slice(&frame);
    let guard = write_stream().lock().map_err(|e| e.to_string())?;
    let s = guard.as_ref().ok_or_else(|| "Not connected".to_string())?;
    let mut s2 = s.try_clone().map_err(|e| e.to_string())?;
    drop(guard);
    s2.write_all(&payload).and_then(|_| s2.flush()).map_err(|e| format!("send failed: {}", e))
}

fn read_packet(stream: &mut TcpStream, cipher: &Aes256Gcm) -> Result<InputPacket, String> {
    let mut len_buf = [0u8; 4];
    read_exact(stream, &mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > MAX_FRAME_LEN { return Err(format!("bad frame len {}", len)); }
    let mut frame = vec![0u8; len];
    read_exact(stream, &mut frame)?;
    let plain = decrypt_frame(cipher, &frame)?;
    InputPacket::deserialize(&plain).map_err(|e| format!("deser: {:?}", e))
}

pub fn connect_to_android(ip: &str, port: u16) -> Result<(), String> {
    crate::log_event(&format!("TCP: connecting to {}:{}...", ip, port));
    let mut stream = TcpStream::connect((ip, port)).map_err(|e| format!("TCP connect to {}:{} failed: {}", ip, port, e))?;
    stream.set_nodelay(true).ok();
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    crate::log_event("TCP: socket connected, sending handshake...");

    let mut my_pub = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut my_pub);
    let name = b"CrossLink-Host";
    let mut hello = Vec::new();
    hello.extend_from_slice(&MAGIC);
    hello.push(PROTO_VERSION);
    hello.push(name.len() as u8);
    hello.extend_from_slice(name);
    hello.extend_from_slice(&my_pub);
    stream.write_all(&hello).and_then(|_| stream.flush()).map_err(|e| format!("handshake send failed: {}", e))?;

    let mut magic = [0u8; 4];
    read_exact(&mut stream, &mut magic)?;
    if magic != MAGIC { return Err("handshake failed: invalid response (is CrossLink Android app listening on this port?)".to_string()); }
    let mut ver = [0u8; 1]; read_exact(&mut stream, &mut ver)?;
    let _ = ver;
    let mut nlen = [0u8; 1]; read_exact(&mut stream, &mut nlen)?;
    let mut pname = vec![0u8; nlen[0] as usize];
    read_exact(&mut stream, &mut pname)?;
    let mut peer_pub = [0u8; 32];
    read_exact(&mut stream, &mut peer_pub)?;
    let peer_name = String::from_utf8_lossy(&pname).to_string();
    crate::log_event(&format!("Handshake: peer='{}' version={}", peer_name, ver[0]));

    let cipher = derive_key(&my_pub, &peer_pub)?;
    *cipher_store().lock().unwrap() = Some(cipher.clone());
    *write_stream().lock().unwrap() = Some(stream.try_clone().map_err(|e| e.to_string())?);

    let _ = send_packet(InputPacket::Handshake { device_name: "CrossLink-Host".to_string(), version: "0.1.0".to_string() });

    stream.set_read_timeout(None).ok();
    let target = format!("{}:{}", ip, port);
    *last_target().lock().unwrap() = target.clone();
    set_connected(true, Some(target.clone()));
    crate::set_notice(&format!("Connected to {} ({}).", peer_name, target), false);
    crate::log_event(&format!("Connected to {} as {}", target, peer_name));

    thread::spawn(move || receive_loop(stream, cipher, target));
    Ok(())
}

fn receive_loop(mut stream: TcpStream, cipher: Aes256Gcm, target: String) {
    let mut last_recv = Instant::now();
    let mut last_hb = Instant::now();
    let mut reason: Option<String> = None;
    loop {
        if last_recv.elapsed() > Duration::from_secs(15) { reason = Some("peer timeout (no packets received)".to_string()); break; }
        if last_hb.elapsed() > Duration::from_secs(3) { let _ = send_packet(InputPacket::Heartbeat { timestamp: 0 }); last_hb = Instant::now(); }
        stream.set_read_timeout(Some(Duration::from_millis(500))).ok();
        match read_packet(&mut stream, &cipher) {
            Ok(InputPacket::ReturnControl) => {
                crate::TARGET_ACTIVE.store(false, Ordering::Relaxed);
                if let Ok(mut d) = crate::ACTIVE_DEVICE.lock() { *d = "Windows".to_string(); }
                last_recv = Instant::now();
            }
            Ok(_) => { last_recv = Instant::now(); }
            Err(e) if e.contains("timed out") || e.contains("WouldBlock") || e.contains("10060") => continue,
            Err(e) => { reason = Some(format!("connection lost: {}", e)); break; }
        }
    }
    set_connected(false, None);
    let msg = reason.unwrap_or_else(|| "disconnected".to_string());
    crate::log_event(&format!("Disconnected from {}: {}", target, msg));
    crate::set_notice(&format!("Disconnected from {}: {}", target, msg), true);
}

pub fn disconnect() {
    *write_stream().lock().unwrap() = None;
    *cipher_store().lock().unwrap() = None;
    set_connected(false, None);
}

pub fn is_connected() -> bool { CONNECTED.load(Ordering::Relaxed) }

pub fn start_auto_reconnect() {
    thread::spawn(|| loop {
        thread::sleep(Duration::from_secs(5));
        if is_connected() { continue; }
        let t = last_target().lock().unwrap().clone();
        if t.is_empty() { continue; }
        if let Some((ip, port)) = t.rsplit_once(':') {
            if let Ok(p) = port.parse::<u16>() {
                let _ = connect_to_android(ip, p);
            }
        }
    });
}
