use std::{
    env,
    ffi::{OsStr, OsString},
    fmt::Display,
    path::{Path, PathBuf},
};

const LIB_DIR_ENV_VARS: [&str; 2] = ["LIBOPUS_LIB_DIR", "OPUS_LIB_DIR"];
const BUNDLED_ENV_VARS: [&str; 2] = ["LIBOPUS_BUNDLED", "OPUS_BUNDLED"];
const STATIC_ENV_VARS: [&str; 2] = ["LIBOPUS_STATIC", "OPUS_STATIC"];
const NO_PKG_ENV_VARS: [&str; 2] = ["LIBOPUS_NO_PKG", "OPUS_NO_PKG"];
const PACKAGE_VERSION_ENV_VARS: [&str; 3] = [
    "LIBOPUS_PACKAGE_VERSION",
    "OPUS_PACKAGE_VERSION",
    "PACKAGE_VERSION",
];
const ANDROID_ABI_ENV_VARS: [&str; 3] = [
    "LIBOPUS_ANDROID_ABI",
    "ANDROID_ABI",
    "CMAKE_ANDROID_ARCH_ABI",
];
const ANDROID_PLATFORM_ENV_VARS: [&str; 5] = [
    "LIBOPUS_ANDROID_PLATFORM",
    "ANDROID_PLATFORM",
    "CARGO_NDK_ANDROID_PLATFORM",
    "CARGO_NDK_PLATFORM",
    "CMAKE_SYSTEM_VERSION",
];
const ANDROID_NDK_ENV_VARS: [&str; 7] = [
    "LIBOPUS_ANDROID_NDK",
    "CMAKE_ANDROID_NDK",
    "ANDROID_NDK_HOME",
    "ANDROID_NDK_ROOT",
    "ANDROID_NDK",
    "ANDROID_NDK_PATH",
    "NDK_HOME",
];
const ANDROID_SYSROOT_ENV_VARS: [&str; 1] = ["CARGO_NDK_SYSROOT_PATH"];
const CMAKE_TOOLCHAIN_ENV_VARS: [&str; 2] =
    ["CMAKE_TOOLCHAIN_FILE", "CARGO_NDK_CMAKE_TOOLCHAIN_PATH"];

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HostOs {
    Linux,
    MacOs,
    Windows,
    Other,
}

impl HostOs {
    fn from_triple(triple: &str) -> Self {
        if triple.contains("apple-darwin") {
            Self::MacOs
        } else if triple.contains("windows") {
            Self::Windows
        } else if triple.contains("linux") {
            Self::Linux
        } else {
            Self::Other
        }
    }
}

#[derive(Clone, Debug)]
struct TargetInfo {
    triple: String,
    os: String,
    env: Option<String>,
    arch: String,
    family: Option<String>,
}

impl TargetInfo {
    fn from_env() -> Self {
        Self {
            triple: env::var("TARGET").expect("Cargo did not provide TARGET"),
            os: env::var("CARGO_CFG_TARGET_OS").expect("Cargo did not provide CARGO_CFG_TARGET_OS"),
            env: env::var("CARGO_CFG_TARGET_ENV")
                .ok()
                .filter(|value| !value.is_empty()),
            arch: env::var("CARGO_CFG_TARGET_ARCH")
                .expect("Cargo did not provide CARGO_CFG_TARGET_ARCH"),
            family: env::var("CARGO_CFG_TARGET_FAMILY")
                .ok()
                .filter(|value| !value.is_empty()),
        }
    }

    fn is_android(&self) -> bool {
        self.os == "android"
    }

    fn is_freebsd(&self) -> bool {
        self.os == "freebsd"
    }

    fn is_gnu(&self) -> bool {
        self.env.as_deref() == Some("gnu")
    }

    fn is_macos(&self) -> bool {
        self.os == "macos"
    }

    fn is_musl(&self) -> bool {
        self.env.as_deref() == Some("musl")
    }

    fn is_unix_like(&self) -> bool {
        self.family.as_deref() == Some("unix") || self.is_android()
    }

    fn is_windows(&self) -> bool {
        self.os == "windows"
    }

    fn allows_pkg_config(&self) -> bool {
        !self.is_android() && !self.is_windows() && (self.is_unix_like() || self.is_gnu())
    }

    fn defaults_to_static_linking(&self) -> bool {
        self.is_windows() || self.is_macos() || self.is_android() || self.is_musl()
    }

    fn needs_libm_when_static(&self) -> bool {
        self.is_unix_like() || self.is_gnu()
    }
}

#[derive(Clone, Debug)]
struct BuildContext {
    host: String,
    host_os: HostOs,
    target: TargetInfo,
}

impl BuildContext {
    fn from_env() -> Self {
        let host = env::var("HOST").expect("Cargo did not provide HOST");

        Self {
            host_os: HostOs::from_triple(&host),
            host,
            target: TargetInfo::from_env(),
        }
    }

    fn emit_rerun_directives(&self) {
        println!("cargo:rerun-if-changed=opus");

        for bases in [
            &LIB_DIR_ENV_VARS[..],
            &BUNDLED_ENV_VARS[..],
            &STATIC_ENV_VARS[..],
            &NO_PKG_ENV_VARS[..],
            &PACKAGE_VERSION_ENV_VARS[..],
            &ANDROID_ABI_ENV_VARS[..],
            &ANDROID_PLATFORM_ENV_VARS[..],
            &ANDROID_NDK_ENV_VARS[..],
            &ANDROID_SYSROOT_ENV_VARS[..],
            &CMAKE_TOOLCHAIN_ENV_VARS[..],
        ] {
            emit_rerun_for_target_bases(&self.target.triple, bases);
        }
    }

    fn env_flag(&self, bases: &[&str]) -> bool {
        self.env_os(bases).is_some()
    }

    fn env_os(&self, bases: &[&str]) -> Option<OsString> {
        target_env_os_first(&self.target.triple, bases)
    }

    fn env_path(&self, bases: &[&str]) -> Option<PathBuf> {
        self.env_os(bases).map(PathBuf::from)
    }

    fn env_string(&self, bases: &[&str]) -> Option<String> {
        target_env_string(&self.target.triple, bases)
    }

    fn is_native_build(&self) -> bool {
        self.host == self.target.triple
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AndroidAbi {
    Arm64V8a,
    ArmeabiV7a,
    X86,
    X86_64,
}

impl AndroidAbi {
    fn as_str(self) -> &'static str {
        match self {
            Self::Arm64V8a => "arm64-v8a",
            Self::ArmeabiV7a => "armeabi-v7a",
            Self::X86 => "x86",
            Self::X86_64 => "x86_64",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value.trim() {
            "arm64-v8a" => Some(Self::Arm64V8a),
            "armeabi-v7a" => Some(Self::ArmeabiV7a),
            "x86" => Some(Self::X86),
            "x86_64" => Some(Self::X86_64),
            _ => None,
        }
    }

    fn infer(target: &TargetInfo) -> Option<Self> {
        match target.triple.as_str() {
            "aarch64-linux-android" => Some(Self::Arm64V8a),
            "arm-linux-androideabi"
            | "armv7-linux-androideabi"
            | "thumbv7neon-linux-androideabi" => Some(Self::ArmeabiV7a),
            "i686-linux-android" => Some(Self::X86),
            "x86_64-linux-android" => Some(Self::X86_64),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
struct AndroidConfig {
    abi: AndroidAbi,
    platform: String,
    ndk_home: Option<PathBuf>,
    toolchain_file: PathBuf,
}

impl AndroidConfig {
    fn detect(context: &BuildContext) -> Self {
        let expected_abi = AndroidAbi::infer(&context.target).unwrap_or_else(|| {
            panic!(
                "Unsupported Android Rust target '{}' for bundled Opus builds.",
                context.target.triple
            )
        });

        let abi = match context.env_string(&ANDROID_ABI_ENV_VARS) {
            Some(value) => {
                let parsed = AndroidAbi::from_str(&value).unwrap_or_else(|| {
                    panic!(
                        "Unsupported Android ABI '{}' for target '{}'. Expected one of: arm64-v8a, armeabi-v7a, x86, x86_64.",
                        value,
                        context.target.triple
                    )
                });

                if parsed != expected_abi {
                    panic!(
                        "Android ABI '{}' does not match Rust target '{}' (expected '{}').",
                        value,
                        context.target.triple,
                        expected_abi.as_str()
                    );
                }

                parsed
            }
            None => expected_abi,
        };

        let platform = context
            .env_string(&ANDROID_PLATFORM_ENV_VARS)
            .and_then(|value| normalize_android_platform(&value))
            .unwrap_or_else(|| {
                panic!(
                    "Android target '{}' requires an API level. Use `cargo ndk -p <api>` or set ANDROID_PLATFORM / LIBOPUS_ANDROID_PLATFORM.",
                    context.target.triple
                )
            });

        let toolchain_file_from_env = context.env_path(&CMAKE_TOOLCHAIN_ENV_VARS);
        let ndk_home = context
            .env_path(&ANDROID_NDK_ENV_VARS)
            .or_else(|| {
                context
                    .env_path(&ANDROID_SYSROOT_ENV_VARS)
                    .and_then(|sysroot| infer_ndk_home_from_sysroot_path(&sysroot))
            })
            .or_else(|| {
                toolchain_file_from_env
                    .as_deref()
                    .and_then(infer_ndk_home_from_toolchain_file)
            });

        let toolchain_file = toolchain_file_from_env
            .or_else(|| {
                ndk_home
                    .as_ref()
                    .map(|ndk_home| ndk_home.join("build/cmake/android.toolchain.cmake"))
            })
            .unwrap_or_else(|| {
                panic!(
                    "Android target '{}' requires an NDK toolchain file. Set CMAKE_TOOLCHAIN_FILE or ANDROID_NDK_HOME / ANDROID_NDK_ROOT, or build via cargo-ndk.",
                    context.target.triple
                )
            });

        if toolchain_file.file_name() != Some(OsStr::new("android.toolchain.cmake")) {
            panic!(
                "CMAKE_TOOLCHAIN_FILE for Android target '{}' must point to android.toolchain.cmake, got '{}'.",
                context.target.triple,
                toolchain_file.display()
            );
        }

        if !toolchain_file.exists() {
            panic!(
                "Android toolchain file '{}' does not exist.",
                toolchain_file.display()
            );
        }

        Self {
            abi,
            platform,
            ndk_home,
            toolchain_file,
        }
    }

    fn apply_to(&self, cfg: &mut cmake::Config) {
        cfg.define("ANDROID_ABI", self.abi.as_str())
            .define("CMAKE_ANDROID_ARCH_ABI", self.abi.as_str())
            .define("ANDROID_PLATFORM", &self.platform)
            .define("CMAKE_SYSTEM_VERSION", &self.platform)
            .define("CMAKE_TOOLCHAIN_FILE", &self.toolchain_file);

        if let Some(ndk_home) = &self.ndk_home {
            cfg.define("CMAKE_ANDROID_NDK", ndk_home);
        }
    }
}

fn env_keys_for_target(base: &str, target: &str) -> Vec<String> {
    let target_u = target.replace('-', "_");
    let mut keys = Vec::with_capacity(4);

    for key in [
        format!("{}_{}", base, target),
        format!("{}_{}", base, target_u),
        format!("TARGET_{}", base),
        base.to_string(),
    ] {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }

    keys
}

fn target_env_os(target: &str, base: &str) -> Option<OsString> {
    env_keys_for_target(base, target)
        .into_iter()
        .find_map(env::var_os)
}

fn target_env_os_first(target: &str, bases: &[&str]) -> Option<OsString> {
    for base in bases {
        if let Some(value) = target_env_os(target, base) {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    None
}

fn target_env_string(target: &str, bases: &[&str]) -> Option<String> {
    target_env_os_first(target, bases)
        .and_then(|value| value.into_string().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn emit_rerun_for_target_bases(target: &str, bases: &[&str]) {
    for base in bases {
        for key in env_keys_for_target(base, target) {
            println!("cargo:rerun-if-env-changed={key}");
        }
    }
}

fn normalize_android_platform(value: &str) -> Option<String> {
    let normalized = value
        .trim()
        .strip_prefix("android-")
        .unwrap_or(value.trim());

    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn infer_ndk_home_from_sysroot_path(sysroot_path: &Path) -> Option<PathBuf> {
    let mut ndk_home = sysroot_path.to_path_buf();

    for _ in 0..5 {
        if !ndk_home.pop() {
            return None;
        }
    }

    Some(ndk_home)
}

fn infer_ndk_home_from_toolchain_file(toolchain_file: &Path) -> Option<PathBuf> {
    if toolchain_file.file_name() != Some(OsStr::new("android.toolchain.cmake")) {
        return None;
    }

    let cmake_dir = toolchain_file.parent()?;
    if cmake_dir.file_name() != Some(OsStr::new("cmake")) {
        return None;
    }

    let build_dir = cmake_dir.parent()?;
    if build_dir.file_name() != Some(OsStr::new("build")) {
        return None;
    }

    Some(build_dir.parent()?.to_path_buf())
}

fn build_opus(context: &BuildContext, is_static: bool) {
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

    println!("cargo:info=Build host: {}.", context.host);
    println!(
        "cargo:info=Build target: {} (os={}, arch={}).",
        context.target.triple, context.target.os, context.target.arch
    );
    println!("cargo:info=Opus source path used: {}.", display_path);
    println!("cargo:info=Building Opus via CMake.");

    let mut cfg = cmake::Config::new(opus_path);

    // Prefer Release artifacts for FFI libs regardless of Rust profile.
    cfg.profile("Release");
    // Force CMake to use 'lib' instead of 'lib64' on x86_64 Linux.
    cfg.define("CMAKE_INSTALL_LIBDIR", "lib");

    if context.target.is_android() {
        let android = AndroidConfig::detect(context);

        println!(
            "cargo:info=Configuring Android build with ABI '{}' and API '{}'.",
            android.abi.as_str(),
            android.platform
        );

        android.apply_to(&mut cfg);
    }

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
    if let Some(explicit_version) = context.env_string(&PACKAGE_VERSION_ENV_VARS) {
        cfg.define("OPUS_PACKAGE_VERSION", &explicit_version)
            .define("PACKAGE_VERSION", &explicit_version);
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
                cfg.define("OPUS_PACKAGE_VERSION", &v)
                    .define("PACKAGE_VERSION", &v);
            }
        }
    }

    // Best-effort disables; ignored by CMake if unsupported.
    cfg.define("OPUS_BUILD_TESTING", "OFF")
        .define("OPUS_ENABLE_DOC", "OFF")
        .define("OPUS_INSTALL_PKG_CONFIG_MODULE", "OFF");

    let opus_build_dir = cfg.build();
    link_opus(context, is_static, opus_build_dir.display())
}

fn link_opus(context: &BuildContext, is_static: bool, opus_build_dir: impl Display) {
    let is_static_text = rustc_linking_word(is_static);

    println!(
        "cargo:info=Linking Opus as {} lib: {}",
        is_static_text, opus_build_dir
    );
    println!("cargo:rustc-link-lib={}=opus", is_static_text);
    println!("cargo:rustc-link-search=native={}/lib", opus_build_dir);

    // On Unix-like targets, static libopus may require libm.
    if is_static && context.target.needs_libm_when_static() {
        println!("cargo:rustc-link-lib=m");
    }
}

fn add_homebrew_opus_search_path(context: &BuildContext) {
    use std::process::Command;

    if context.host_os != HostOs::MacOs || !context.is_native_build() || !context.target.is_macos()
    {
        return;
    }

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

fn default_library_linking(context: &BuildContext) -> bool {
    if context.target.defaults_to_static_linking() {
        true
    } else if context.target.is_freebsd() || context.target.is_gnu() {
        false
    } else {
        false
    }
}

fn find_installed_opus(context: &BuildContext) -> Option<PathBuf> {
    context.env_path(&LIB_DIR_ENV_VARS)
}

fn find_via_pkg_config(context: &BuildContext, is_static: bool) -> bool {
    if !context.target.allows_pkg_config() {
        return false;
    }

    pkg_config::Config::new()
        .statik(is_static)
        .probe("opus")
        .is_ok()
}

fn is_static_build(context: &BuildContext) -> bool {
    if cfg!(feature = "static") && cfg!(feature = "dynamic") {
        default_library_linking(context)
    } else if cfg!(feature = "static") || context.env_flag(&STATIC_ENV_VARS) {
        println!("cargo:info=Static feature or environment variable found.");

        true
    } else if cfg!(feature = "dynamic") {
        println!("cargo:info=Dynamic feature enabled.");

        false
    } else {
        println!("cargo:info=No feature or environment variable found, linking by default.");

        default_library_linking(context)
    }
}

fn bundled_enabled(context: &BuildContext) -> bool {
    cfg!(feature = "bundled") || context.env_flag(&BUNDLED_ENV_VARS)
}

fn main() {
    let context = BuildContext::from_env();
    context.emit_rerun_directives();

    #[cfg(feature = "generate_binding")]
    generate_binding();

    add_homebrew_opus_search_path(&context);

    let is_static = is_static_build(&context);

    // If explicitly requested, always build the vendored source.
    if bundled_enabled(&context) {
        build_opus(&context, is_static);
        return;
    }

    if context.env_flag(&NO_PKG_ENV_VARS) {
        println!("cargo:info=Bypassed `pkg-config`.");
    } else if find_via_pkg_config(&context, is_static) {
        println!("cargo:info=Found `Opus` via `pkg_config`.");

        return;
    } else if context.target.allows_pkg_config() {
        println!("cargo:info=`pkg_config` could not find `Opus`.");
    }

    if let Some(installed_opus) = find_installed_opus(&context) {
        link_opus(&context, is_static, installed_opus.display());
    } else if Path::new("opus").exists() {
        // Building from a repository checkout that contains the vendored sources.
        build_opus(&context, is_static);
    } else {
        panic!(
            "Could not locate a system libopus and no vendored 'opus/' was found.\n\
             Options to resolve:\n\
             1) Enable the 'bundled' feature to build the vendored lib (cargo build --features bundled)\n\
             2) Install libopus and set OPUS_LIB_DIR/LIBOPUS_LIB_DIR to its prefix (containing 'lib')\n\
             3) On supported non-Android Unix targets, ensure pkg-config can find 'opus'"
        );
    }
}
