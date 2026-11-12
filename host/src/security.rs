use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub fn generate_shared_secret() -> ([u8; 32], [u8; 32]) {
    let mut priv_a = [0u8; 32];
    let mut pub_a = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut priv_a);
    pub_a.copy_from_slice(&Sha256::digest(&priv_a));
    (priv_a, pub_a)
}

pub fn export_public_key(pub_key: &[u8; 32]) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, pub_key)
}

pub fn import_public_key(s: &str) -> Result<[u8; 32], String> {
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
        .map_err(|e| format!("Base64 error: {}", e))?;
    if decoded.len() != 32 {
        return Err(format!("Invalid key length: {}", decoded.len()));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&decoded);
    Ok(key)
}

pub fn derive_cipher(shared_secret: &[u8]) -> Result<Aes256Gcm, String> {
    let hk = Hkdf::<Sha256>::new(None, shared_secret);
    let mut okm = [0u8; 32];
    hk.expand(b"crosslink-key", &mut okm).map_err(|e| e.to_string())?;
    Aes256Gcm::new_from_slice(&okm).map_err(|e| e.to_string())
}

pub fn encrypt_frame(cipher: &Aes256Gcm, plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ct = cipher.encrypt(Nonce::from_slice(&nonce_bytes), plain)
        .map_err(|e| format!("encrypt: {:?}", e))?;
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn decrypt_frame(cipher: &Aes256Gcm, frame: &[u8]) -> Result<Vec<u8>, String> {
    if frame.len() < 28 {
        return Err("frame too short".to_string());
    }
    cipher.decrypt(Nonce::from_slice(&frame[..12]), &frame[12..])
        .map_err(|e| format!("decrypt: {:?}", e))
}