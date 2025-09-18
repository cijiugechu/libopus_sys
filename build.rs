#[cfg(feature = "generate_binding")]
use std::path::PathBuf;
use std::{env, fmt::Display, path::Path};

/// Outputs the library-file's prefix as word usable for actual arguments on
/// commands or paths.
const fn rustc_linking_word(is_static_link: bool) -> &'static str {
    if is_static_link {
        "static"
    } else {
        "dylib"
    }
}

/// Generates a new binding at `src/lib.rs` using `src/wrapper.h`.
#[cfg(feature = "generate_binding")]
fn generate_binding() {
    const ALLOW_UNCONVENTIONALS: &'static str = "#![allow(non_upper_case_globals)]\n\
                                                 #![allow(non_camel_case_types)]\n\
                                                 #![allow(non_snake_case)]\n";

    let bindings = bindgen::Builder::default()
        .header("src/wrapper.h")
        .raw_line(ALLOW_UNCONVENTIONALS)
        .generate()
        .expect("Unable to generate binding");

    let binding_target_path = PathBuf::new().join("src").join("lib.rs");

    bindings
        .write_to_file(binding_target_path)
        .expect("Could not write binding to the file at `src/lib.rs`");

    println!("cargo:info=Successfully generated binding.");
}

fn build_opus(is_static: bool) {
    let opus_path = Path::new("opus");

    if !opus_path.exists() {
        panic!(
            "'opus/' source directory not found. To build without a system lib, enable the 'bundled' feature or set OPUS_LIB_DIR/LIBOPUS_LIB_DIR.\n\
             - Enable feature: cargo build --features bundled\n\
             - Or install libopus and set OPUS_LIB_DIR/LIBOPUS_LIB_DIR to its prefix (containing 'lib')."
        );
    }

    let display_path = opus_path
        .canonicalize()
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| opus_path.display().to_string());

    println!("cargo:info=Opus source path used: {}.", display_path);
    println!("cargo:info=Building Opus via CMake.");

    let mut cfg = cmake::Config::new(opus_path);

    // Prefer Release artifacts for FFI libs regardless of Rust profile
    cfg.profile("Release");

    if is_static {
        cfg.define("BUILD_SHARED_LIBS", "OFF")
            .define("OPUS_BUILD_SHARED_LIBRARY", "OFF")
            .define("OPUS_BUILD_STATIC_LIBRARY", "ON");
    } else {
        cfg.define("BUILD_SHARED_LIBS", "ON")
            .define("OPUS_BUILD_SHARED_LIBRARY", "ON")
            .define("OPUS_BUILD_STATIC_LIBRARY", "OFF");
    }

    // Inject PACKAGE_VERSION so opus reports a proper version string.
    // Prefer env override, then parse opus/package_version if present.
    if let Ok(explicit_version) = env::var("OPUS_PACKAGE_VERSION") {
        if !explicit_version.trim().is_empty() {
            cfg.define("OPUS_PACKAGE_VERSION", &explicit_version);
            cfg.define("PACKAGE_VERSION", &explicit_version);
        }
    } else if let Ok(contents) = std::fs::read_to_string(opus_path.join("package_version"))
        .or_else(|_| std::fs::read_to_string(opus_path.join("cmake").join("package_version")))
    {
        if let Some(v) = contents.lines().find_map(|l| {
            let l = l.trim();
            if l.starts_with("PACKAGE_VERSION=") {
                Some(
                    l.trim_start_matches("PACKAGE_VERSION=")
                        .trim_matches('"')
                        .to_string(),
                )
            } else {
                None
            }
        }) {
            if !v.is_empty() {
                cfg.define("OPUS_PACKAGE_VERSION", &v);
                cfg.define("PACKAGE_VERSION", &v);
            }
        }
    }

    // Best-effort disables; ignored by CMake if unsupported.
    cfg.define("OPUS_BUILD_TESTING", "OFF")
        .define("OPUS_ENABLE_DOC", "OFF")
        .define("OPUS_INSTALL_PKG_CONFIG_MODULE", "OFF");

    let opus_build_dir = cfg.build();
    link_opus(is_static, opus_build_dir.display())
}

fn link_opus(is_static: bool, opus_build_dir: impl Display) {
    let is_static_text = rustc_linking_word(is_static);

    println!(
        "cargo:info=Linking Opus as {} lib: {}",
        is_static_text, opus_build_dir
    );
    println!("cargo:rustc-link-lib={}=opus", is_static_text);
    println!("cargo:rustc-link-search=native={}/lib", opus_build_dir);

    // On Unix-like systems, static libopus may require libm
    if is_static && (cfg!(unix) || cfg!(target_env = "gnu")) {
        println!("cargo:rustc-link-lib=m");
    }
}

#[cfg(target_os = "macos")]
fn add_homebrew_opus_search_path() {
    use std::process::Command;

    if let Ok(output) = Command::new("brew").args(["--prefix", "opus"]).output() {
        if output.status.success() {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !prefix.is_empty() {
                let lib_dir = format!("{}/lib", prefix);
                println!("cargo:info=Adding Homebrew Opus search path: {}", lib_dir);
                println!("cargo:rustc-link-search=native={}", lib_dir);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn add_homebrew_opus_search_path() {}

#[cfg(any(unix, target_env = "gnu"))]
fn find_via_pkg_config(is_static: bool) -> bool {
    pkg_config::Config::new()
        .statik(is_static)
        .probe("opus")
        .is_ok()
}

/// Based on the OS or target environment we are building for,
/// this function will return an expected default library linking method.
///
/// If we build for Windows, MacOS, or Linux with musl, we will link statically.
/// However, if you build for Linux without musl, we will link dynamically.
///
/// **Info**:
/// This is a helper-function and may not be called if
/// if the `static`-feature is enabled, the environment variable
/// `LIBOPUS_STATIC` or `OPUS_STATIC` is set.
fn default_library_linking() -> bool {
    #[cfg(any(windows, target_os = "macos", target_env = "musl"))]
    {
        true
    }
    #[cfg(any(target_os = "freebsd", all(unix, target_env = "gnu")))]
    {
        false
    }
}

fn find_installed_opus() -> Option<String> {
    if let Ok(lib_directory) = env::var("LIBOPUS_LIB_DIR") {
        Some(lib_directory)
    } else if let Ok(lib_directory) = env::var("OPUS_LIB_DIR") {
        Some(lib_directory)
    } else {
        None
    }
}

fn is_static_build() -> bool {
    if cfg!(feature = "static") && cfg!(feature = "dynamic") {
        default_library_linking()
    } else if cfg!(feature = "static")
        || env::var("LIBOPUS_STATIC").is_ok()
        || env::var("OPUS_STATIC").is_ok()
    {
        println!("cargo:info=Static feature or environment variable found.");

        true
    } else if cfg!(feature = "dynamic") {
        println!("cargo:info=Dynamic feature enabled.");

        false
    } else {
        println!("cargo:info=No feature or environment variable found, linking by default.");

        default_library_linking()
    }
}

fn bundled_enabled() -> bool {
    cfg!(feature = "bundled")
        || env::var("LIBOPUS_BUNDLED").is_ok()
        || env::var("OPUS_BUNDLED").is_ok()
}

fn main() {
    // Rebuild if vendored sources or relevant env vars change
    println!("cargo:rerun-if-changed=opus");
    println!("cargo:rerun-if-env-changed=OPUS_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LIBOPUS_LIB_DIR");
    println!("cargo:rerun-if-env-changed=OPUS_BUNDLED");
    println!("cargo:rerun-if-env-changed=LIBOPUS_BUNDLED");
    println!("cargo:rerun-if-env-changed=OPUS_STATIC");
    println!("cargo:rerun-if-env-changed=LIBOPUS_STATIC");
    #[cfg(feature = "generate_binding")]
    generate_binding();

    add_homebrew_opus_search_path();

    let is_static = is_static_build();

    // If explicitly requested, always build the vendored source
    if bundled_enabled() {
        build_opus(is_static);
        return;
    }

    #[cfg(any(unix, target_env = "gnu"))]
    {
        if std::env::var("LIBOPUS_NO_PKG").is_ok() || std::env::var("OPUS_NO_PKG").is_ok() {
            println!("cargo:info=Bypassed `pkg-config`.");
        } else if find_via_pkg_config(is_static) {
            println!("cargo:info=Found `Opus` via `pkg_config`.");

            return;
        } else {
            println!("cargo:info=`pkg_config` could not find `Opus`.");
        }
    }

    if let Some(installed_opus) = find_installed_opus() {
        link_opus(is_static, installed_opus);
    } else if Path::new("opus").exists() {
        // Building from a repository checkout that contains the vendored sources
        build_opus(is_static);
    } else {
        panic!(
            "Could not locate a system libopus and no vendored 'opus/' was found.\n\
             Options to resolve:\n\
             1) Enable the 'bundled' feature to build the vendored lib (cargo build --features bundled)\n\
             2) Install libopus and set OPUS_LIB_DIR/LIBOPUS_LIB_DIR to its prefix (containing 'lib')\n\
             3) On Unix (gnu), ensure pkg-config can find 'opus'"
        );
    }
}
