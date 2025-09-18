use std::ffi::CStr;

fn main() {
    unsafe {
        let ver = libopus_sys::opus_get_version_string();
        if ver.is_null() {
            panic!("opus_get_version_string returned null");
        }
        let ver = CStr::from_ptr(ver).to_string_lossy().into_owned();

        let ok = CStr::from_ptr(libopus_sys::opus_strerror(libopus_sys::OPUS_OK as i32))
            .to_string_lossy()
            .into_owned();

        println!("libopus version: {}", ver);
        println!("OPUS_OK strerror: {}", ok);
    }
}
