use std::ffi::CStr;

pub fn smoke_check() -> (String, String) {
    // `opus_get_version_string` and `opus_strerror` return pointers to
    // static, null-terminated strings owned by libopus. `OPUS_OK` is a valid
    // status code, and we only read the strings immediately without taking
    // ownership of the underlying storage.
    unsafe {
        let version = libopus_sys::opus_get_version_string();
        if version.is_null() {
            panic!("opus_get_version_string returned null");
        }

        let ok = libopus_sys::opus_strerror(libopus_sys::OPUS_OK as i32);
        if ok.is_null() {
            panic!("opus_strerror returned null for OPUS_OK");
        }

        let version = CStr::from_ptr(version).to_string_lossy().into_owned();
        let ok = CStr::from_ptr(ok).to_string_lossy().into_owned();

        (version, ok)
    }
}

#[cfg(test)]
mod tests {
    use super::smoke_check;

    #[test]
    fn bundled_build_exports_basic_symbols() {
        let (version, ok) = smoke_check();

        assert!(
            version.to_ascii_lowercase().contains("opus"),
            "unexpected version string: {version}"
        );
        assert!(!ok.trim().is_empty(), "unexpected OPUS_OK error string");
    }
}
