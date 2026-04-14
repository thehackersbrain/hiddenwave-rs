use crate::error::HiddenWaveError;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng, rand_core::RngCore},
};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const ITERATIONS: u32 = 100_000;

pub fn encrypt_payload(data: &[u8], password: &str) -> Result<Vec<u8>, HiddenWaveError> {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, ITERATIONS, &mut key_bytes);

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|_| HiddenWaveError::Crypto("Invalid key length".into()))?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let mut ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|_| HiddenWaveError::Crypto("Encryption failed".into()))?;

    let mut result = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce);
    result.append(&mut ciphertext);

    Ok(result)
}

pub fn decrypt_payload(data: &[u8], password: &str) -> Result<Vec<u8>, HiddenWaveError> {
    if data.len() < SALT_LEN + NONCE_LEN {
        return Err(HiddenWaveError::Crypto(
            "Data too short to be encrypted".into(),
        ));
    }

    let (salt, rest) = data.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);

    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, ITERATIONS, &mut key_bytes);

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|_| HiddenWaveError::Crypto("Invalid key length".into()))?;

    let nonce = Nonce::from_slice(nonce_bytes);

    cipher.decrypt(nonce, ciphertext).map_err(|_| {
        HiddenWaveError::Crypto("Decryption failed. Incorrect password or corrupted data.".into())
    })
}
