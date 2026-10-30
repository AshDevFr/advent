//! age operations for the key model: day and hint keys are wrapped to the
//! release recipients, source copies are encrypted to the editors
//! recipients, and any single matching identity opens either. Recipients are
//! lists so a new key can be added, everything resealed, and the old key
//! removed without locking anyone out. Everything happens in memory.

use std::io::{Read, Write};
use std::str::FromStr;

use age::secrecy::ExposeSecret;
use age::x25519;

use crate::primitives::{KEY_LEN, Key};

#[derive(Debug, thiserror::Error)]
pub enum WrapError {
    #[error("invalid age recipient {0:?}")]
    BadRecipient(String),
    #[error("invalid age identity on line {0}")]
    BadIdentity(usize),
    #[error("no recipients given")]
    NoRecipients,
    #[error("none of the given keys can open this; it was sealed to other recipients")]
    NoMatchingIdentity,
    #[error("unwrapped key has {0} bytes, expected {KEY_LEN}")]
    BadKeyLength(usize),
    #[error("age error: {0}")]
    Age(String),
}

/// Parse public keys (`age1...`), as listed in `advent.yaml`.
pub fn parse_recipients(keys: &[String]) -> Result<Vec<x25519::Recipient>, WrapError> {
    keys.iter()
        .map(|k| {
            x25519::Recipient::from_str(k.trim()).map_err(|_| WrapError::BadRecipient(k.clone()))
        })
        .collect()
}

/// Parse a key file: one `AGE-SECRET-KEY-1...` per line, blank lines and
/// `#` comments ignored, so one file can hold several keys.
pub fn parse_identities(text: &str) -> Result<Vec<x25519::Identity>, WrapError> {
    text.lines()
        .enumerate()
        .map(|(i, line)| (i + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .map(|(n, line)| x25519::Identity::from_str(line).map_err(|_| WrapError::BadIdentity(n)))
        .collect()
}

/// A fresh key pair as `(secret, public)` strings.
pub fn generate_identity() -> (String, String) {
    let identity = x25519::Identity::generate();
    let public = identity.to_public().to_string();
    (identity.to_string().expose_secret().to_string(), public)
}

/// Encrypt `data` so that any one of `recipients` can decrypt it.
pub fn encrypt_to(data: &[u8], recipients: &[x25519::Recipient]) -> Result<Vec<u8>, WrapError> {
    if recipients.is_empty() {
        return Err(WrapError::NoRecipients);
    }
    let encryptor =
        age::Encryptor::with_recipients(recipients.iter().map(|r| r as &dyn age::Recipient))
            .map_err(|e| WrapError::Age(e.to_string()))?;
    let mut out = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut out)
        .map_err(|e| WrapError::Age(e.to_string()))?;
    writer
        .write_all(data)
        .and_then(|_| writer.finish().map(|_| ()))
        .map_err(|e| WrapError::Age(e.to_string()))?;
    Ok(out)
}

/// Decrypt with whichever of `identities` matches a recipient of `data`.
pub fn decrypt_with(data: &[u8], identities: &[x25519::Identity]) -> Result<Vec<u8>, WrapError> {
    let decryptor = age::Decryptor::new(data).map_err(|e| WrapError::Age(e.to_string()))?;
    let mut reader = decryptor
        .decrypt(identities.iter().map(|i| i as &dyn age::Identity))
        .map_err(|e| match e {
            age::DecryptError::NoMatchingKeys => WrapError::NoMatchingIdentity,
            other => WrapError::Age(other.to_string()),
        })?;
    let mut out = Vec::new();
    reader
        .read_to_end(&mut out)
        .map_err(|e| WrapError::Age(e.to_string()))?;
    Ok(out)
}

/// Wrap a day or hint key to the release recipients.
pub fn wrap_key(key: &Key, recipients: &[x25519::Recipient]) -> Result<Vec<u8>, WrapError> {
    encrypt_to(key, recipients)
}

pub fn unwrap_key(wrapped: &[u8], identities: &[x25519::Identity]) -> Result<Key, WrapError> {
    let bytes = decrypt_with(wrapped, identities)?;
    let len = bytes.len();
    bytes.try_into().map_err(|_| WrapError::BadKeyLength(len))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> (x25519::Identity, x25519::Recipient) {
        let (secret, public) = generate_identity();
        (
            parse_identities(&secret).unwrap().remove(0),
            parse_recipients(&[public]).unwrap().remove(0),
        )
    }

    #[test]
    fn wrap_any_listed_recipient_can_unwrap() {
        let (a_id, a) = pair();
        let (b_id, b) = pair();
        let key = [42u8; KEY_LEN];
        let wrapped = wrap_key(&key, &[a, b]).unwrap();
        assert_eq!(unwrap_key(&wrapped, &[a_id]).unwrap(), key);
        assert_eq!(unwrap_key(&wrapped, &[b_id]).unwrap(), key);
    }

    #[test]
    fn wrap_unlisted_identity_is_refused() {
        let (_, a) = pair();
        let (c_id, _) = pair();
        let wrapped = wrap_key(&[1; KEY_LEN], &[a]).unwrap();
        assert!(matches!(
            unwrap_key(&wrapped, &[c_id]),
            Err(WrapError::NoMatchingIdentity)
        ));
    }

    #[test]
    fn wrap_key_file_with_several_keys_and_comments() {
        let (s1, p1) = generate_identity();
        let (s2, p2) = generate_identity();
        let file = format!("# my own key\n{s1}\n\n# team key from Vault\n{s2}\n");
        let ids = parse_identities(&file).unwrap();
        assert_eq!(ids.len(), 2);

        let only_second = parse_recipients(&[p2]).unwrap();
        let data = encrypt_to(b"source archive", &only_second).unwrap();
        assert_eq!(decrypt_with(&data, &ids).unwrap(), b"source archive");

        let only_first = parse_recipients(&[p1]).unwrap();
        let data = encrypt_to(b"other", &only_first).unwrap();
        assert_eq!(decrypt_with(&data, &ids).unwrap(), b"other");
    }

    #[test]
    fn wrap_malformed_recipient_is_named() {
        let err = parse_recipients(&["age1nope".into()]).unwrap_err();
        assert!(err.to_string().contains("age1nope"));
    }

    #[test]
    fn wrap_bad_identity_line_is_numbered() {
        // Identities have no Debug impl (they are secrets), so no unwrap_err.
        let result = parse_identities("# comment\nAGE-SECRET-KEY-1BAD");
        assert!(matches!(result, Err(WrapError::BadIdentity(2))));
    }

    #[test]
    fn wrap_needs_at_least_one_recipient() {
        assert!(matches!(
            encrypt_to(b"x", &[]),
            Err(WrapError::NoRecipients)
        ));
    }
}
