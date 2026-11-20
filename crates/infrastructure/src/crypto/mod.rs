//! 加密与密钥分层（SPEC 4.3-3、DESIGN 2.7.3）。
//!
//! L0：OS 密钥库（DPAPI / Keychain）持有的 32B 主密钥。
//! L1：由 L0 + 应用标识 + 随机盐经 HKDF-SHA256 派生，仅内存持有。
//! L2：字段级 AES-256-GCM，AAD 绑定 `session_id`。

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

pub const MASTER_KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;
pub const AEAD_TAG_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyTier {
    L0Master,
    L1Derived,
    L2Field,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CryptoError {
    #[error("密钥长度必须为 32 字节，实际为 {0}")]
    InvalidKeyLength(usize),
    #[error("主密钥不可用：{0}")]
    MasterKeyUnavailable(String),
    #[error("密钥轮换失败，拒绝启动")]
    RekeyFailed,
    #[error("字段加密失败")]
    FieldEncryptFailed,
    #[error("字段解密失败（AAD 不匹配或密文损坏）")]
    FieldDecryptFailed,
}

/// 主密钥引用。只允许 `dpapi://` / `keychain://` 形式，禁止明文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthRef(String);

impl AuthRef {
    pub fn new(raw: impl Into<String>) -> Result<Self, CryptoError> {
        let value = raw.into();
        if value.starts_with("dpapi://") || value.starts_with("keychain://") {
            Ok(Self(value))
        } else {
            Err(CryptoError::MasterKeyUnavailable(
                "authRef 必须为 dpapi:// 或 keychain:// 引用".to_owned(),
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_plaintext(&self) -> bool {
        !(self.0.starts_with("dpapi://") || self.0.starts_with("keychain://"))
    }
}

pub trait KeyProvider: Send + Sync {
    fn load_master(&self) -> Result<[u8; MASTER_KEY_LEN], CryptoError>;

    fn store_master(&self, key: &[u8; MASTER_KEY_LEN]) -> Result<AuthRef, CryptoError>;

    fn rekey(&self) -> Result<AuthRef, CryptoError>;
}

pub fn generate_master() -> [u8; MASTER_KEY_LEN] {
    let mut key = [0_u8; MASTER_KEY_LEN];
    OsRng.fill_bytes(&mut key);
    key
}

/// L1 派生：`HKDF-SHA256(master, salt, info = app_id)`。
pub fn derive_l1(
    master: &[u8; MASTER_KEY_LEN],
    app_id: &str,
    salt: &[u8],
) -> Result<[u8; MASTER_KEY_LEN], CryptoError> {
    if app_id.trim().is_empty() {
        return Err(CryptoError::MasterKeyUnavailable(
            "应用标识不得为空".to_owned(),
        ));
    }
    let hkdf = Hkdf::<Sha256>::new(Some(salt), master);
    let mut okm = [0_u8; MASTER_KEY_LEN];
    hkdf.expand(app_id.as_bytes(), &mut okm)
        .map_err(|_| CryptoError::MasterKeyUnavailable("HKDF 派生失败".to_owned()))?;
    Ok(okm)
}

fn cipher(key: &[u8; MASTER_KEY_LEN]) -> Result<Aes256Gcm, CryptoError> {
    Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::InvalidKeyLength(key.len()))
}

/// L2 字段加密。信封格式：`nonce(12) ‖ ciphertext ‖ tag(16)`。
pub fn encrypt_field(
    key: &[u8; MASTER_KEY_LEN],
    session_id: &str,
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = cipher(key)?;
    let mut nonce_bytes = [0_u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad: session_id.as_bytes(),
            },
        )
        .map_err(|_| CryptoError::FieldEncryptFailed)?;

    let mut envelope = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ciphertext);
    Ok(envelope)
}

pub fn decrypt_field(
    key: &[u8; MASTER_KEY_LEN],
    session_id: &str,
    envelope: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if envelope.len() < NONCE_LEN + AEAD_TAG_LEN {
        return Err(CryptoError::FieldDecryptFailed);
    }
    let (nonce_bytes, ciphertext) = envelope.split_at(NONCE_LEN);
    let cipher = cipher(key)?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad: session_id.as_bytes(),
            },
        )
        .map_err(|_| CryptoError::FieldDecryptFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_ref_rejects_plaintext() {
        assert!(AuthRef::new("master-key-in-config").is_err());
        assert!(AuthRef::new("dpapi://local/meeting-copilot").is_ok());
        assert!(AuthRef::new("keychain://meeting-copilot/master").is_ok());
    }

    #[test]
    fn auth_ref_detects_plaintext() {
        let plaintext = AuthRef("raw".to_owned());
        assert!(plaintext.is_plaintext());
    }

    #[test]
    fn field_roundtrip() {
        let master = generate_master();
        let key = derive_l1(&master, "com.meetingcopilot.desktop", b"device-salt").expect("派生");
        let envelope = encrypt_field(&key, "sess-1", "确认下周灰度".as_bytes()).expect("加密");
        assert!(!envelope
            .windows(4)
            .any(|window| window == "确认".as_bytes()));
        let plaintext = decrypt_field(&key, "sess-1", &envelope).expect("解密");
        assert_eq!(plaintext, "确认下周灰度".as_bytes());
    }

    #[test]
    fn aad_mismatch_is_rejected() {
        let master = generate_master();
        let key = derive_l1(&master, "app", b"salt").expect("派生");
        let envelope = encrypt_field(&key, "sess-1", b"secret").expect("加密");
        assert_eq!(
            decrypt_field(&key, "sess-2", &envelope),
            Err(CryptoError::FieldDecryptFailed)
        );
    }

    #[test]
    fn derivation_is_deterministic_and_salt_sensitive() {
        let master = generate_master();
        let first = derive_l1(&master, "app", b"salt-a").expect("派生");
        let again = derive_l1(&master, "app", b"salt-a").expect("派生");
        let other = derive_l1(&master, "app", b"salt-b").expect("派生");
        assert_eq!(first, again);
        assert_ne!(first, other);
    }

    #[test]
    fn truncated_envelope_is_rejected() {
        let master = generate_master();
        let key = derive_l1(&master, "app", b"salt").expect("派生");
        assert_eq!(
            decrypt_field(&key, "sess", &[0_u8; 10]),
            Err(CryptoError::FieldDecryptFailed)
        );
    }
}
