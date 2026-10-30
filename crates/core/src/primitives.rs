//! The sealing primitives of the crypto spec. Every value here (KDF, hash,
//! iteration counts, salt, IV and key sizes) must match the browser's
//! WebCrypto byte for byte; `test-vectors.json`, generated with WebCrypto,
//! is what holds them to it.
//!
//! Salts, IVs and keys are parameters rather than generated here: callers
//! draw them from a CSPRNG, and tests pass the fixed values from the vectors.
//! Ciphertexts use the WebCrypto layout: AES-GCM output with the 16-byte tag
//! appended, and no associated data.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use sha2::Sha256;

use crate::normalize::normalize;

/// PBKDF2 iterations for a word key, derived from a normalized answer.
pub const WORD_ITERATIONS: u32 = 150_000;
/// PBKDF2 iterations for the final key, derived from the joined words.
pub const FINAL_ITERATIONS: u32 = 200_000;

pub const SALT_LEN: usize = 16;
pub const IV_LEN: usize = 12;
pub const KEY_LEN: usize = 32;

pub type Salt = [u8; SALT_LEN];
pub type Iv = [u8; IV_LEN];
pub type Key = [u8; KEY_LEN];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PrimitiveError {
    /// Wrong key or tampered ciphertext; AES-GCM cannot tell them apart.
    #[error("could not decrypt: wrong key or corrupted data")]
    Decrypt,
    /// The answer normalizes to nothing, so it cannot derive a key.
    #[error("the answer is empty after normalization")]
    EmptyAnswer,
    #[error("decrypted text is not valid UTF-8")]
    NotUtf8,
}

/// A day's word, encrypted with a key derived from one accepted answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordLock {
    pub salt: Salt,
    pub iv: Iv,
    pub ct: Vec<u8>,
}

fn derive_key(secret: &str, salt: &Salt, iterations: u32) -> Key {
    let mut key = [0u8; KEY_LEN];
    pbkdf2::pbkdf2_hmac::<Sha256>(secret.as_bytes(), salt, iterations, &mut key);
    key
}

fn encrypt(key: &Key, iv: &Iv, plaintext: &[u8]) -> Vec<u8> {
    Aes256Gcm::new(key.into())
        .encrypt(iv.into(), plaintext)
        .expect("AES-GCM encryption of an in-memory buffer cannot fail")
}

fn decrypt(key: &Key, iv: &Iv, ct: &[u8]) -> Result<Vec<u8>, PrimitiveError> {
    Aes256Gcm::new(key.into())
        .decrypt(iv.into(), ct)
        .map_err(|_| PrimitiveError::Decrypt)
}

fn answer_key(answer: &str, salt: &Salt) -> Result<Key, PrimitiveError> {
    let normalized = normalize(answer);
    if normalized.is_empty() {
        return Err(PrimitiveError::EmptyAnswer);
    }
    Ok(derive_key(&normalized, salt, WORD_ITERATIONS))
}

/// Lock `word` so that `answer` (normalized here) opens it.
pub fn seal_word(answer: &str, word: &str, salt: Salt, iv: Iv) -> Result<WordLock, PrimitiveError> {
    let key = answer_key(answer, &salt)?;
    let ct = encrypt(&key, &iv, word.as_bytes());
    Ok(WordLock { salt, iv, ct })
}

/// Try a guess against a word lock. An error means the guess is wrong.
pub fn open_word(answer: &str, lock: &WordLock) -> Result<String, PrimitiveError> {
    let key = answer_key(answer, &lock.salt)?;
    let plain = decrypt(&key, &lock.iv, &lock.ct)?;
    String::from_utf8(plain).map_err(|_| PrimitiveError::NotUtf8)
}

/// Encrypt a day's puzzle bundle with its random day key.
pub fn seal_bundle(day_key: &Key, iv: Iv, bundle: &[u8]) -> Vec<u8> {
    encrypt(day_key, &iv, bundle)
}

pub fn open_bundle(day_key: &Key, iv: Iv, ct: &[u8]) -> Result<Vec<u8>, PrimitiveError> {
    decrypt(day_key, &iv, ct)
}

/// Encrypt one hint with its own random key, released separately from the day.
pub fn seal_hint(hint_key: &Key, iv: Iv, hint: &str) -> Vec<u8> {
    encrypt(hint_key, &iv, hint.as_bytes())
}

pub fn open_hint(hint_key: &Key, iv: Iv, ct: &[u8]) -> Result<String, PrimitiveError> {
    let plain = decrypt(hint_key, &iv, ct)?;
    String::from_utf8(plain).map_err(|_| PrimitiveError::NotUtf8)
}

/// The final key's input: the words in day order, joined by a single space.
fn final_secret(words: &[&str]) -> String {
    words.join(" ")
}

/// Encrypt the certificate payload with the key derived from all the words.
pub fn seal_final(words: &[&str], salt: Salt, iv: Iv, payload: &[u8]) -> Vec<u8> {
    let key = derive_key(&final_secret(words), &salt, FINAL_ITERATIONS);
    encrypt(&key, &iv, payload)
}

pub fn open_final(
    words: &[&str],
    salt: Salt,
    iv: Iv,
    ct: &[u8],
) -> Result<Vec<u8>, PrimitiveError> {
    let key = derive_key(&final_secret(words), &salt, FINAL_ITERATIONS);
    decrypt(&key, &iv, ct)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_tampered_bundle_fails() {
        let key = [7u8; KEY_LEN];
        let iv = [1u8; IV_LEN];
        let mut ct = seal_bundle(&key, iv, b"door five");
        ct[0] ^= 1;
        assert_eq!(open_bundle(&key, iv, &ct), Err(PrimitiveError::Decrypt));
    }

    #[test]
    fn primitives_answer_normalizing_to_empty_is_rejected() {
        let err = seal_word("東京", "light", [0; SALT_LEN], [0; IV_LEN]).unwrap_err();
        assert_eq!(err, PrimitiveError::EmptyAnswer);
    }

    #[test]
    fn primitives_answer_variants_open_the_same_lock() {
        let lock = seal_word("Paris", "light", [3; SALT_LEN], [4; IV_LEN]).unwrap();
        assert_eq!(open_word(" PARIS! ", &lock).unwrap(), "light");
    }
}
