use std::ffi::CStr;

use libopus_sys::{opus_get_version_string, opus_strerror, OPUS_OK};

fn main() {
    unsafe {
        let ver = CStr::from_ptr(opus_get_version_string())
            .to_string_lossy()
            .into_owned();
        let ok = CStr::from_ptr(opus_strerror(OPUS_OK as i32))
            .to_string_lossy()
            .into_owned();

        println!("libopus version: {}", ver);
        println!("OPUS_OK strerror: {}", ok);
    }
}


