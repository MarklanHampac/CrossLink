use std::net::UdpSocket;
use std::time::{Duration, Instant};

pub fn discover_android(timeout_ms: u64) -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.set_broadcast(true).ok()?;
    sock.set_read_timeout(Some(Duration::from_millis(250))).ok()?;
    let msg = b"CROSSLINK_DISCOVER";
    let targets = [
        "255.255.255.255:8889",
        "192.168.1.255:8889",
        "192.168.0.255:8889",
        "10.0.0.255:8889",
    ];
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut buf = [0u8; 512];
    while Instant::now() < deadline {
        for t in targets { let _ = sock.send_to(msg, t); }
        match sock.recv_from(&mut buf) {
            Ok((n, addr)) => {
                let resp = String::from_utf8_lossy(&buf[..n]).to_string();
                if let Some(port) = resp.strip_prefix("CROSSLINK_HERE:") {
                    let p = port.trim().parse::<u16>().unwrap_or(8888);
                    return Some(format!("{}:{}", addr.ip(), p));
                }
            }
            Err(_) => {}
        }
    }
    None
}
