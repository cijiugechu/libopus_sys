fn main() {
    let (version, ok) = libopus_sys_bundled::smoke_check();

    println!("libopus version: {}", version);
    println!("OPUS_OK strerror: {}", ok);
}
