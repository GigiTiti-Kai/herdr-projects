//! The build cache every Rust repo on the board shares: a task whose folder has a `Cargo.toml` is started with
//! sccache's variables, so a new worktree does not compile its libraries again, whatever its repo's own scripts do. The
//! one place that knows about sccache is `scripts/build-cache.sh`, carried in the program, as a task's repo need not
//! be this one. docs/build-cache.md.
use std::path::Path;
use std::process::{Command, Stdio};

const SCRIPT: &str = include_str!("../../../scripts/build-cache.sh");

/// The variables for a build in `folder`: none where it is no Rust repo, or where the script gives none (the cache
/// is off, cannot be installed here, or you set a `RUSTC_WRAPPER` of your own). A build then goes on alone.
pub fn env(folder: &Path) -> Vec<(String, String)> {
    if !folder.join("Cargo.toml").is_file() { return vec![]; }
    let out = Command::new("sh").args(["-c", SCRIPT, "build-cache.sh", "--env"]).stdin(Stdio::null()).stderr(Stdio::null()).output();
    out.map(|o| parse(&String::from_utf8_lossy(&o.stdout))).unwrap_or_default()
}
/// `NAME=value` lines.
pub fn parse(text: &str) -> Vec<(String, String)> {
    text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.to_string(), v.to_string())).collect()
}
