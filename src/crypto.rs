//! Minimal crypto for the passphrase-protected store on Unix.
//!
//! Only two operations are needed: PBKDF2-HMAC-SHA256 key derivation and
//! AES-256-GCM sealing. Both come from `ring`, which supplies the platform
//! primitives without pulling in a large TLS stack.
//!
//! `LessSafeKey` is used deliberately: the nonce is generated once per save and
//! never reused for the same key, because every save derives a fresh salt and
//! therefore a fresh key. The type name is a warning about reuse, not a ban.

use crate::error::AppError;

/// Derive a 32-byte key from a passphrase and salt.
pub fn pbkdf2_key(passphrase: &str, salt: &[u8], iterations: u32) -> Result<[u8; 32], AppError> {
    use ring::pbkdf2;
    use std::num::NonZeroU32;

    let n =
        NonZeroU32::new(iterations).ok_or_else(|| AppError::store("PBKDF2 の反復回数が 0 です"))?;
    let mut key = [0u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        n,
        salt,
        passphrase.as_bytes(),
        &mut key,
    );
    Ok(key)
}

/// Encrypt `plain` with AES-256-GCM, returning ciphertext followed by the tag.
pub fn aes_gcm_seal(key: &[u8; 32], nonce: &[u8], plain: &[u8]) -> Result<Vec<u8>, AppError> {
    use ring::aead;

    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| AppError::store("AES-256-GCM の鍵を作れません"))?;
    let key = aead::LessSafeKey::new(unbound);
    let mut buf = plain.to_vec();
    key.seal_in_place_append_tag(nonce_of(nonce)?, aead::Aad::empty(), &mut buf)
        .map_err(|_| AppError::store("暗号化に失敗しました"))?;
    Ok(buf)
}

/// Decrypt ciphertext+tag produced by [`aes_gcm_seal`].
pub fn aes_gcm_open(key: &[u8; 32], nonce: &[u8], sealed: &[u8]) -> Result<Vec<u8>, AppError> {
    use ring::aead;

    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| AppError::store("AES-256-GCM の鍵を作れません"))?;
    let key = aead::LessSafeKey::new(unbound);
    let mut buf = sealed.to_vec();
    let plain = key
        .open_in_place(nonce_of(nonce)?, aead::Aad::empty(), &mut buf)
        .map_err(|_| AppError::store("復号に失敗しました"))?;
    Ok(plain.to_vec())
}

fn nonce_of(nonce: &[u8]) -> Result<ring::aead::Nonce, AppError> {
    use std::convert::TryInto;
    let bytes: [u8; 12] = nonce
        .try_into()
        .map_err(|_| AppError::store("nonce の長さが 12 バイトではありません"))?;
    Ok(ring::aead::Nonce::assume_unique_for_key(bytes))
}

/// Fill `len` bytes from the OS CSPRNG.
pub fn random_bytes(len: usize) -> Result<Vec<u8>, AppError> {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut buf = vec![0u8; len];
    SystemRandom::new()
        .fill(&mut buf)
        .map_err(|_| AppError::store("乱数を生成できません"))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_with_same_key_and_nonce() {
        let key = [7u8; 32];
        let nonce = [9u8; 12];
        let plain = b"hello, odquota";
        let sealed = aes_gcm_seal(&key, &nonce, plain).expect("seal");
        let opened = aes_gcm_open(&key, &nonce, &sealed).expect("open");
        assert_eq!(opened, plain);
    }

    #[test]
    fn wrong_key_fails_to_open() {
        let key = [7u8; 32];
        let other = [8u8; 32];
        let nonce = [9u8; 12];
        let sealed = aes_gcm_seal(&key, &nonce, b"secret").expect("seal");
        assert!(aes_gcm_open(&other, &nonce, &sealed).is_err());
    }

    #[test]
    fn wrong_nonce_fails_to_open() {
        let key = [7u8; 32];
        let nonce = [9u8; 12];
        let other_nonce = [1u8; 12];
        let sealed = aes_gcm_seal(&key, &nonce, b"secret").expect("seal");
        assert!(aes_gcm_open(&key, &other_nonce, &sealed).is_err());
    }

    #[test]
    fn pbkdf2_is_deterministic_and_salt_sensitive() {
        let salt_a = [1u8; 16];
        let salt_b = [2u8; 16];
        let k1 = pbkdf2_key("pass", &salt_a, 1000).expect("k1");
        let k2 = pbkdf2_key("pass", &salt_a, 1000).expect("k2");
        let k3 = pbkdf2_key("pass", &salt_b, 1000).expect("k3");
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn random_bytes_differ() {
        let a = random_bytes(16).expect("a");
        let b = random_bytes(16).expect("b");
        assert_ne!(a, b);
    }
}
