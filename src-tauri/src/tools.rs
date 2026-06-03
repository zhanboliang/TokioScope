//! Toolchain discovery. A bundled GUI app on macOS launches with a minimal
//! environment that does **not** include the user's login-shell `PATH`, so
//! `cargo` (installed by rustup under `~/.cargo/bin`) is frequently missing.
//! These helpers resolve cargo explicitly and hand every spawned cargo an
//! augmented `PATH` so it can still find `rustc`, the linker, etc.

use std::ffi::OsString;
use std::path::PathBuf;

/// Directories that commonly hold the Rust toolchain but may be absent from a
/// GUI app's inherited `PATH`.
fn toolchain_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(home) = dirs::home_dir() {
        v.push(home.join(".cargo").join("bin"));
    }
    // Homebrew (Apple silicon + Intel) and the common /usr/local prefix.
    v.push(PathBuf::from("/opt/homebrew/bin"));
    v.push(PathBuf::from("/usr/local/bin"));
    v
}

fn cargo_exe_name() -> &'static str {
    if cfg!(windows) {
        "cargo.exe"
    } else {
        "cargo"
    }
}

/// Resolve the `cargo` executable. Honors `$CARGO`, then `~/.cargo/bin`, then
/// falls back to the bare name (relying on whatever `PATH` exists).
pub fn cargo_bin() -> PathBuf {
    if let Some(c) = std::env::var_os("CARGO") {
        let p = PathBuf::from(c);
        if p.exists() {
            return p;
        }
    }
    for d in toolchain_dirs() {
        let c = d.join(cargo_exe_name());
        if c.exists() {
            return c;
        }
    }
    PathBuf::from("cargo")
}

/// `PATH` with the toolchain dirs prepended, so a spawned cargo can locate
/// `rustc` and friends even when the app was launched with a stripped env.
pub fn augmented_path() -> OsString {
    let mut dirs = toolchain_dirs();
    if let Some(existing) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(dirs).unwrap_or_else(|_| std::env::var_os("PATH").unwrap_or_default())
}

/// A `tokio` cargo command pre-wired with the resolved binary + augmented PATH.
pub fn cargo_command() -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(cargo_bin());
    cmd.env("PATH", augmented_path());
    cmd
}
