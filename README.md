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
- This crate ships with pre-generated bindings. To **regenerate bindings** you need
  [`Clang`](https://rust-lang.github.io/rust-bindgen/requirements.html#clang) and `LIBCLANG_PATH`.

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

## Linking

`libopus_sys` targets Opus 1.5 and supports Windows, Linux, and macOS.

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
