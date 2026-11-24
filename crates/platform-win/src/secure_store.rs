//! Windows 凭据管理器适配：唯一目标名、写入覆盖、删除幂等。

use meeting_copilot_application::settings::{PortError, SecretBytes, SecureStore};

/// 模型服务的凭据目标名。
pub const CREDENTIAL_TARGET: &str = "meeting-copilot/llm-api-key";

/// 联网检索服务的凭据目标名。与模型 Key 分开存，互不覆盖。
pub const SEARCH_CREDENTIAL_TARGET: &str = "meeting-copilot/search-api-key";

/// 凭据存储实现；非 Windows 目标统一返回 `Unsupported`。
pub struct WindowsCredentialStore {
    target: String,
}

impl WindowsCredentialStore {
    /// 使用模型服务的目标名构造。
    pub fn new() -> Self {
        Self::with_target_name(CREDENTIAL_TARGET)
    }

    /// 用指定目标名构造：一个目标名只存一条凭据，不同服务必须用不同目标名。
    pub fn with_target_name(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
        }
    }
}

#[cfg(test)]
impl WindowsCredentialStore {
    fn with_target(target: impl Into<String>) -> Self {
        Self::with_target_name(target)
    }
}

impl Default for WindowsCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
impl SecureStore for WindowsCredentialStore {
    fn write(&self, secret: &SecretBytes) -> Result<(), PortError> {
        secret.with_bytes(|bytes| win::write(&self.target, bytes))
    }

    fn read(&self) -> Result<Option<SecretBytes>, PortError> {
        win::read(&self.target)
    }

    fn delete(&self) -> Result<(), PortError> {
        win::delete(&self.target)
    }
}

#[cfg(not(windows))]
impl SecureStore for WindowsCredentialStore {
    fn write(&self, _secret: &SecretBytes) -> Result<(), PortError> {
        Err(PortError::Unsupported)
    }

    fn read(&self) -> Result<Option<SecretBytes>, PortError> {
        Err(PortError::Unsupported)
    }

    fn delete(&self) -> Result<(), PortError> {
        Err(PortError::Unsupported)
    }
}

#[cfg(windows)]
mod win {
    use core::ffi::c_void;

    use meeting_copilot_application::settings::{PortError, SecretBytes};
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ACCESS_DENIED, ERROR_NOT_FOUND};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(core::iter::once(0)).collect()
    }

    fn map_error(code: u32) -> PortError {
        match code {
            ERROR_ACCESS_DENIED => PortError::Denied,
            _ => PortError::Unavailable,
        }
    }

    pub(super) fn write(target: &str, bytes: &[u8]) -> Result<(), PortError> {
        let target_wide = wide(target);
        let size = u32::try_from(bytes.len()).map_err(|_| PortError::Denied)?;
        let mut credential: CREDENTIALW = unsafe { core::mem::zeroed() };
        credential.Type = CRED_TYPE_GENERIC;
        credential.TargetName = target_wide.as_ptr() as *mut u16;
        credential.CredentialBlobSize = size;
        credential.CredentialBlob = bytes.as_ptr() as *mut u8;
        credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
        let ok = unsafe { CredWriteW(&credential, 0) };
        if ok == 0 {
            Err(map_error(unsafe { GetLastError() }))
        } else {
            Ok(())
        }
    }

    pub(super) fn read(target: &str) -> Result<Option<SecretBytes>, PortError> {
        let target_wide = wide(target);
        let mut ptr: *mut CREDENTIALW = core::ptr::null_mut();
        let ok = unsafe { CredReadW(target_wide.as_ptr(), CRED_TYPE_GENERIC, 0, &mut ptr) };
        if ok == 0 {
            return match unsafe { GetLastError() } {
                ERROR_NOT_FOUND => Ok(None),
                code => Err(map_error(code)),
            };
        }
        if ptr.is_null() {
            return Ok(None);
        }
        let credential = unsafe { &*ptr };
        let bytes = if credential.CredentialBlobSize == 0 || credential.CredentialBlob.is_null() {
            Vec::new()
        } else {
            let blob = unsafe {
                core::slice::from_raw_parts(
                    credential.CredentialBlob as *const u8,
                    credential.CredentialBlobSize as usize,
                )
            };
            blob.to_vec()
        };
        unsafe { CredFree(ptr as *const c_void) };
        Ok(Some(SecretBytes::new(bytes)))
    }

    pub(super) fn delete(target: &str) -> Result<(), PortError> {
        let target_wide = wide(target);
        let ok = unsafe { CredDeleteW(target_wide.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok == 0 {
            return match unsafe { GetLastError() } {
                ERROR_NOT_FOUND => Ok(()),
                code => Err(map_error(code)),
            };
        }
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::WindowsCredentialStore;
    use meeting_copilot_application::settings::{SecretBytes, SecureStore};

    const TEST_TARGET: &str = "meeting-copilot/test/secure-store";

    fn clean_store() -> WindowsCredentialStore {
        let store = WindowsCredentialStore::with_target(TEST_TARGET);
        store.delete().expect("cleanup");
        store
    }

    #[test]
    fn secure_store_roundtrip_uses_fixed_target() {
        let store = clean_store();
        store
            .write(&SecretBytes::new(b"test-key-roundtrip".to_vec()))
            .expect("write");
        let value = store.read().expect("read").expect("present");
        value.with_bytes(|bytes| assert_eq!(bytes, b"test-key-roundtrip"));
        store.delete().expect("delete");
        assert!(store.read().expect("read after delete").is_none());
    }

    #[test]
    fn secure_store_write_is_idempotent() {
        let store = clean_store();
        store
            .write(&SecretBytes::new(b"test-key-idempotent".to_vec()))
            .expect("first write");
        store
            .write(&SecretBytes::new(b"test-key-idempotent".to_vec()))
            .expect("second write");
        let value = store.read().expect("read").expect("present");
        value.with_bytes(|bytes| assert_eq!(bytes, b"test-key-idempotent"));
        store.delete().expect("delete");
    }

    #[test]
    fn secure_store_delete_is_idempotent() {
        let store = clean_store();
        store.delete().expect("first delete");
        store.delete().expect("second delete");
    }
}

#[cfg(test)]
mod platform_tests {
    #[test]
    fn secure_store_is_windows_only() {
        let source = include_str!("secure_store.rs");
        let forbidden = ["target_os = \"ma", "cos\""].concat();
        assert!(!source.contains(&forbidden));
        assert_eq!(cfg!(windows), cfg!(target_os = "windows"));
    }
}
