use aes_gcm::aead::KeyInit;
use aes_gcm::Aes256Gcm;
use sha2::{Digest, Sha256};

pub fn derive_cipher(key_material: &[u8]) -> Result<Aes256Gcm, String> {
    let key = if key_material.len() == 32 {
        key_material.to_vec()
    } else {
        Sha256::digest(key_material).to_vec()
    };
    Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())
}
