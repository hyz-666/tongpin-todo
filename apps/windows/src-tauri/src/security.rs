//! Windows DPAPI wrapper: protect/unprotect secrets with the current user's key.
//!
//! This is the only module in the crate that needs `unsafe` (Win32 FFI), so the
//! crate-level `deny(unsafe_code)` is relaxed here via `allow`.

#![allow(unsafe_code)]

use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CryptProtectData, CryptUnprotectData,
};

const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

fn blob_from(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    }
}

fn empty_blob() -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    }
}

/// Read the output blob and release the system allocation.
unsafe fn take_output(output: &CRYPT_INTEGER_BLOB) -> Vec<u8> {
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        let _ = LocalFree(Some(HLOCAL(output.pbData as *mut core::ffi::c_void)));
    }
    bytes
}

/// Encrypt `data` under the current Windows user's credentials.
pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    let input = blob_from(data);
    let mut output = empty_blob();
    let result = unsafe {
        CryptProtectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if result.is_ok() {
        Ok(unsafe { take_output(&output) })
    } else {
        Err(format!("CryptProtectData failed: {result:?}"))
    }
}

/// Decrypt `data` previously produced by [`protect`].
pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    let input = blob_from(data);
    let mut output = empty_blob();
    let result = unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if result.is_ok() {
        Ok(unsafe { take_output(&output) })
    } else {
        Err(format!("CryptUnprotectData failed: {result:?}"))
    }
}
