use std::path::PathBuf;
use crate::core::errors::CoreError;

#[cfg(target_os = "windows")]
mod win_dpapi {
    use std::ptr;
    use crate::core::errors::CoreError;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct DATA_BLOB {
        cbData: u32,
        pbData: *mut u8,
    }

    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(
            pDataIn: *const DATA_BLOB,
            szDataDescr: *const u16,
            pOptionalEntropy: *const DATA_BLOB,
            pvReserved: *mut std::ffi::c_void,
            pPromptStruct: *mut std::ffi::c_void,
            dwFlags: u32,
            pDataOut: *mut DATA_BLOB,
        ) -> i32;

        fn CryptUnprotectData(
            pDataIn: *const DATA_BLOB,
            ppszDataDescr: *mut *mut u16,
            pOptionalEntropy: *const DATA_BLOB,
            pvReserved: *mut std::ffi::c_void,
            pPromptStruct: *mut std::ffi::c_void,
            dwFlags: u32,
            pDataOut: *mut DATA_BLOB,
        ) -> i32;

        fn LocalFree(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    pub fn protect(data: &[u8]) -> Result<Vec<u8>, CoreError> {
        let mut in_blob = DATA_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out_blob = DATA_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        let res = unsafe {
            CryptProtectData(
                &mut in_blob,
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };
        if res == 0 {
            let err = std::io::Error::last_os_error();
            return Err(CoreError::Identity(format!("Windows DPAPI CryptProtectData failed: {}", err)));
        }
        let slice = unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) };
        let result = slice.to_vec();
        unsafe { LocalFree(out_blob.pbData as *mut std::ffi::c_void) };
        Ok(result)
    }

    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, CoreError> {
        let mut in_blob = DATA_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out_blob = DATA_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        let res = unsafe {
            CryptUnprotectData(
                &mut in_blob,
                ptr::null_mut(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };
        if res == 0 {
            let err = std::io::Error::last_os_error();
            return Err(CoreError::Identity(format!("Windows DPAPI CryptUnprotectData failed: {}", err)));
        }
        let slice = unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) };
        let result = slice.to_vec();
        unsafe { LocalFree(out_blob.pbData as *mut std::ffi::c_void) };
        Ok(result)
    }
}

/// Protects secret bytes using Windows DPAPI (or a test bypass if STARDEW_SYNC_TEST_MOCK_DPAPI is set)
pub fn protect_secret(data: &[u8]) -> Result<Vec<u8>, CoreError> {
    if std::env::var("STARDEW_SYNC_TEST_MOCK_DPAPI").is_ok() {
        // Deterministic synthetic obfuscation for test fixtures where DPAPI may be isolated
        let mut out = Vec::with_capacity(data.len() + 4);
        out.extend_from_slice(b"MOCK");
        for b in data {
            out.push(b ^ 0x5A);
        }
        return Ok(out);
    }

    #[cfg(target_os = "windows")]
    {
        win_dpapi::protect(data)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(CoreError::Identity("DPAPI is only supported on Windows".to_string()))
    }
}

/// Unprotects secret bytes using Windows DPAPI (or a test bypass if STARDEW_SYNC_TEST_MOCK_DPAPI is set)
pub fn unprotect_secret(data: &[u8]) -> Result<Vec<u8>, CoreError> {
    if std::env::var("STARDEW_SYNC_TEST_MOCK_DPAPI").is_ok() {
        if data.len() < 4 || &data[0..4] != b"MOCK" {
            return Err(CoreError::Identity("Invalid mock encrypted secret payload".to_string()));
        }
        let out: Vec<u8> = data[4..].iter().map(|b| b ^ 0x5A).collect();
        return Ok(out);
    }

    #[cfg(target_os = "windows")]
    {
        win_dpapi::unprotect(data)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(CoreError::Identity("DPAPI is only supported on Windows".to_string()))
    }
}

/// Gets the directory path for storing device identity files
pub fn get_identity_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("STARDEW_SYNC_IDENTITY_DIR") {
        return PathBuf::from(custom);
    }

    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_app_data).join("StardewSync").join("identity")
    } else {
        dirs_fallback().join("identity")
    }
}

fn dirs_fallback() -> PathBuf {
    if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data).join("StardewSync")
    } else {
        PathBuf::from(".").join("StardewSync")
    }
}
