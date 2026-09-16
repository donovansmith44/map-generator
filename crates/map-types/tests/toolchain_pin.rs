//! TOOLCHAIN-1 (atlas spec 2026-09-14-relational-artifact-design §3.2):
//! map-generator path-depends on atlas-graph-types from eight crates and
//! pins the atlas version root (C6). The two repos must build with the
//! same compiler, or the root the atlas computes and the root this repo
//! recomputes could disagree with ZERO data change.

use std::path::{Path, PathBuf};
use std::process::Command;

const PINNED: &str = "1.97.1";

fn repo_root() -> PathBuf {
    // crates/map-types/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn channel_from(toml: &str) -> Option<String> {
    toml.lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("channel"))
        .and_then(|rest| rest.trim().strip_prefix('='))
        .map(|v| v.trim().trim_matches('"').to_string())
}

#[test]
fn rust_toolchain_toml_pins_the_exact_version() {
    let path = repo_root().join("rust-toolchain.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must exist at the repo root: {e}", path.display()));
    let channel = channel_from(&text).expect("rust-toolchain.toml must declare `channel = \"...\"`");
    assert_eq!(channel, PINNED, "channel must be the exact version {PINNED:?}, never \"stable\"");
}

#[test]
fn the_compiler_that_built_this_test_is_the_pinned_version() {
    let rustc = Path::new(env!("CARGO")).with_file_name(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    let out = Command::new(&rustc).arg("--version").output()
        .unwrap_or_else(|e| panic!("could not run {}: {e}", rustc.display()));
    let v = String::from_utf8_lossy(&out.stdout);
    assert!(v.starts_with(&format!("rustc {PINNED} ")), "built by {v:?}; the pin requires rustc {PINNED}");
}

#[test]
fn rustup_resolves_the_pin_from_the_repo_root() {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).expect("a home dir");
    let proxy = Path::new(&home).join(".cargo").join("bin")
        .join(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    let out = Command::new(&proxy).arg("--version").current_dir(repo_root()).output()
        .unwrap_or_else(|e| panic!("could not run the rustup proxy {}: {e}", proxy.display()));
    let v = String::from_utf8_lossy(&out.stdout);
    assert!(v.starts_with(&format!("rustc {PINNED} ")), "resolved {v:?} from the repo root; expected {PINNED}");
}
