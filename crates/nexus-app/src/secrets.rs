//! API keys at rest: encrypted with Windows DPAPI (bound to this Windows user account), so a copied
//! settings file is useless on another machine/account. Linux and macOS have no DPAPI: keys are stored base64 in a
//! settings file only the user's account may read (see `Settings::save`), and the UI says so ([`KEPT`]).

use base64::Engine;

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// How a saved key is kept, as the UI puts it ("Your key stays …").
pub const KEPT: &str = if cfg!(windows) {
    "encrypted on this PC"
} else if cfg!(target_os = "macos") {
    "on this Mac, unencrypted but readable only by your user account"
} else {
    "on this PC, unencrypted but readable only by your user account"
};

/// Describe older Windows values according to their actual protection.
pub fn storage_label(sealed: &str) -> &'static str {
    if cfg!(windows) && sealed.starts_with("b64:") { "stored without encryption by an earlier version" } else { KEPT }
}

#[cfg(any(windows, test))]
const SAVE_FAILED: &str = "Windows could not protect this key. The saved credential has not changed.";

pub fn seal(plain: &str) -> Result<String, &'static str> {
    if plain.is_empty() {
        return Ok(String::new());
    }
    #[cfg(windows)]
    { protect_windows(plain, platform::protect) }
    #[cfg(not(windows))]
    { Ok(format!("b64:{}", B64.encode(plain.as_bytes()))) }
}

#[cfg(any(windows, test))]
fn protect_windows(plain: &str, protect: impl FnOnce(&[u8]) -> Option<Vec<u8>>) -> Result<String, &'static str> {
    protect(plain.as_bytes()).map(|bytes| format!("dpapi:{}", B64.encode(bytes))).ok_or(SAVE_FAILED)
}

/// Decrypts a stored secret; returns an empty string if it cannot be read.
pub fn open(sealed: &str) -> String {
    if let Some(rest) = sealed.strip_prefix("dpapi:") {
        B64.decode(rest).ok().and_then(|b| platform::unprotect(&b)).and_then(|b| String::from_utf8(b).ok()).unwrap_or_default()
    } else if let Some(rest) = sealed.strip_prefix("b64:") {
        B64.decode(rest).ok().and_then(|b| String::from_utf8(b).ok()).unwrap_or_default()
    } else {
        String::new()
    }
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB};

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 }
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        // SAFETY: DPAPI returned a LocalAlloc'd buffer of cbData bytes that we own and must free.
        unsafe {
            let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            LocalFree(out.pbData as _);
            v
        }
    }

    pub fn protect(data: &[u8]) -> Option<Vec<u8>> {
        let input = blob(data);
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        // SAFETY: pointers are valid for the duration of the call; flags forbid any UI prompt.
        let ok = unsafe {
            CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out)
        };
        if ok == 0 || out.pbData.is_null() {
            return None;
        }
        // SAFETY: success path, buffer owned by us.
        Some(unsafe { take(out) })
    }

    pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
        let input = blob(data);
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        // SAFETY: as above.
        let ok = unsafe {
            CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out)
        };
        if ok == 0 || out.pbData.is_null() {
            return None;
        }
        // SAFETY: success path, buffer owned by us.
        Some(unsafe { take(out) })
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn unprotect(_: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protection_failure_returns_no_storable_value() {
        assert_eq!(protect_windows("synthetic", |_| None), Err(SAVE_FAILED));
        assert_eq!(protect_windows("synthetic", |_| Some(vec![1, 2, 3])).unwrap(), "dpapi:AQID");
    }

    #[test]
    fn roundtrip() {
        let s = seal("my-secret-api-key-123").expect("test credential storage");
        assert!(!s.contains("my-secret"));
        assert_eq!(open(&s), "my-secret-api-key-123");
        assert_eq!(seal(""), Ok(String::new()));
        assert_eq!(open("garbage"), "");
    }
}
