//! Probestack Shim for Linux GNU Linker
//!
//! WORKAROUND: Provides a stub implementation of `__rust_probestack` to satisfy
//! the linker when building on Linux with GNU toolchain. This symbol is sometimes
//! required by LLVM-compiled dependencies (e.g., wasmer_compiler) but may not be
//! provided by the Rust standard library in certain release build configurations.
//!
//! This is a known issue with stack probe generation in cross-compilation scenarios
//! and certain LLVM versions. The function is intentionally empty because stack
//! probing is already handled by the Rust runtime on Linux.
//!
//! References:
//! - https://github.com/rust-lang/rust/issues/46592
//! - https://github.com/rust-lang/rust/issues/59127
//!
//! This shim is only compiled on Linux GNU targets and has no effect on other platforms.

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[no_mangle]
pub extern "C" fn __rust_probestack() {
    // Intentionally empty - stack probing is handled by Rust runtime
}
