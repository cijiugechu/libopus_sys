# `libopus_sys`

`libopus_sys` is an FFI-Rust-binding to `Opus` version 1.5.

Originally, this sys-crate was made to empower the `serenity`-crate to build audio features on Windows, Linux, and macOS. However, it's not limited to that.

Everyone is welcome to contribute,
check out the [`CONTRIBUTING.md`](CONTRIBUTING.md) for further guidance.

# Building & Linking

This crate can either:

- link against a **system-installed** `libopus`, or
- build Opus from **vendored sources** (no system `libopus` required) via the `bundled` feature.

## Features

- `bundled`: build Opus from the vendored `opus/` sources using CMake (**does not require a system `libopus`**)
- `static`: prefer static linking
- `dynamic`: prefer dynamic linking
- `generate_binding`: regenerate `src/lib.rs` from `src/wrapper.h` (requires libclang)

## Requirements

- To **build Opus from source** (e.g. `--features bundled`): you need `cmake`.
- To **link a system libopus via pkg-config**: you need `pkg-config` (Unix / GNU targets only).
- To **cross-build for Android**: use the Android NDK and `cargo-ndk`.
- This crate ships with pre-generated bindings. To **regenerate bindings** you need
  [`Clang`](https://rust-lang.github.io/rust-bindgen/requirements.html#clang) and `LIBCLANG_PATH`.

## Platform Support

| Platform | Supported path | Default linking |
|----------|----------------|-----------------|
| Windows | Native build or `bundled` | Static |
| macOS | Native build or `bundled` | Static |
| Linux `gnu` | Native build, `pkg-config`, or `bundled` | Dynamic |
| Linux `musl` | Native build or `bundled` | Static |
| Android | `cargo-ndk` with `bundled` | Static |

Android support is target-aware inside `build.rs` and is validated via dedicated Android workflows.
For Android, the build script consumes the NDK/toolchain environment exported by `cargo-ndk` rather
than trying to guess compiler flags itself.

## Recommended: build from vendored sources (no system dependency)

If you want a build that does not depend on any system-installed `libopus`, enable `bundled`:

```bash
cargo build --features bundled
```

You can also force this via environment variables:

- `OPUS_BUNDLED=1`
- `LIBOPUS_BUNDLED=1`

When `bundled` is enabled, the build script uses CMake to compile the vendored `opus/` sources and
links the resulting library.

## Android

The supported Android path is `cargo-ndk` plus the vendored `opus/` sources.

Install the Android Rust targets you intend to build:

```bash
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  i686-linux-android \
  x86_64-linux-android
```

Typical build commands:

```bash
cargo ndk -t arm64-v8a -P 21 build --features bundled,static --release
cargo ndk -t x86_64 -P 21 test --package libopus_sys_bundled --features bundled,static --release --no-run
```

The repository's Android runtime smoke test is based on `cargo ndk-test`:

```bash
cargo ndk-test -t x86_64 -P 29 --package libopus_sys_bundled --features bundled,static --release
```

Android notes:

- The build script validates that the Android ABI matches the Rust target triple.
- The build script passes `ANDROID_ABI`, `CMAKE_ANDROID_ARCH_ABI`, `ANDROID_PLATFORM`,
  `CMAKE_SYSTEM_VERSION`, `CMAKE_ANDROID_NDK`, and `CMAKE_TOOLCHAIN_FILE` to CMake when targeting
  Android.
- If you use `cargo-ndk`, those values are discovered from its environment automatically.
- If you are not using `cargo-ndk`, you can override Android detection with:
  `LIBOPUS_ANDROID_ABI`, `LIBOPUS_ANDROID_PLATFORM`, `LIBOPUS_ANDROID_NDK`, and
  `CMAKE_TOOLCHAIN_FILE`.
- The build script accepts target-specific environment-variable overrides in the same order as the
  `cc` and `cmake` crates: `<VAR>_<target>`, `<VAR>_<target_with_underscores>`, `TARGET_<VAR>`,
  then `<VAR>`.
- Android builds skip `pkg-config` and do not use the Homebrew search-path logic.
- `dynamic` builds are available, but this repository's validation focuses on `bundled,static`.

## Linking

`libopus_sys` targets Opus 1.5 and supports Windows, Linux, macOS, and Android.

### Static vs dynamic

- By default, we link **statically** on Windows, macOS, and `musl` targets.
- By default, we link **dynamically** on Linux `gnu` targets.

You can override this with features:

- `--features static`
- `--features dynamic`

If both are enabled, we pick the default for your target (as described above).

Environment variables `LIBOPUS_STATIC` or `OPUS_STATIC` take precedence over features: if either
is set, static linking is selected (the value does not matter).

### How libopus is located (when `bundled` is not enabled)

On Unix / GNU targets, the build script will:

- try `pkg-config` for `opus` (unless `LIBOPUS_NO_PKG` or `OPUS_NO_PKG` is set)
- otherwise, if `LIBOPUS_LIB_DIR` or `OPUS_LIB_DIR` is set, link from that prefix
- otherwise, if vendored sources (`opus/`) are present, build Opus via CMake

If none of these work, the build fails with instructions on how to proceed.

## Pkg-Config

On Unix / GNU targets (and when `bundled` is not enabled), `libopus_sys` will try `pkg-config`
first. Set `LIBOPUS_NO_PKG=1` or `OPUS_NO_PKG=1` to bypass it.

## System libopus (pre-installed)

If you prefer to link an existing `libopus` installation (or you already ship one with your
application), you can point the build script at it:

- `LIBOPUS_LIB_DIR=/path/to/prefix`
- `OPUS_LIB_DIR=/path/to/prefix`

Where `/path/to/prefix` contains `lib/` (e.g. `/usr/local`, a Homebrew prefix, etc.).

Be aware that using an Opus other than version 1.5 may not work.

# Generating The Binding
If you want to generate the binding yourself, you can use the
`generate_binding`-feature.

Be aware, `bindgen` requires Clang and its `LIBCLANG_PATH`
environment variable to be specified.

# Installation
Add this to your `Cargo.toml`:

```toml
[dependencies]
libopus_sys = "0.3"
```
