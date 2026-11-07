//! The build cache every Rust repo on the board shares: a task whose folder has a `Cargo.toml` is started with
//! sccache's variables, so a new worktree does not compile its libraries again, whatever its repo's own scripts do. The
//! one place that knows about sccache is `scripts/build-cache.sh`, carried in the program, as a task's repo need not
//! be this one. The cache saves time, not disk: every hit is written into the worktree's `target/` all the same. What
//! writes less is a smaller `target/`, so such a task also builds with no incremental notes and only line tables for
//! debugging. docs/build-cache.md.
use std::path::Path;
use std::process::{Command, Stdio};

const SCRIPT: &str = include_str!("../../../scripts/build-cache.sh");
/// Never a value that changes from run to run: the cache keys every compile on the `CARGO_` variables.
const SMALLER: [(&str, &str); 2] = [("CARGO_INCREMENTAL", "0"), ("CARGO_PROFILE_DEV_DEBUG", "line-tables-only")];

/// The variables for a build in `folder`: none where it is no Rust repo, or where the script gives none (the cache
/// is off, cannot be installed here, or you set a `RUSTC_WRAPPER` of your own). A build then goes on alone.
pub fn env(folder: &Path) -> Vec<(String, String)> {
    let Ok(manifest) = std::fs::read_to_string(folder.join("Cargo.toml")) else { return vec![] };
    let out = Command::new("sh").args(["-c", SCRIPT, "build-cache.sh", "--env"]).stdin(Stdio::null()).stderr(Stdio::null()).output();
    out.map(|o| with_smaller(parse(&String::from_utf8_lossy(&o.stdout)), &manifest)).unwrap_or_default()
}
/// The cache's variables with those of a smaller build, unless there are none or the repo chose its own profiles.
/// This one did: its `quick` builds lean on the compiler's incremental notes for a rebuild of seconds (docs/testing.md).
pub fn with_smaller(mut vars: Vec<(String, String)>, manifest: &str) -> Vec<(String, String)> {
    if !vars.is_empty() && !manifest.contains("[profile.") { vars.extend(SMALLER.iter().map(|(k, v)| (k.to_string(), v.to_string()))); }
    vars
}
/// `NAME=value` lines.
pub fn parse(text: &str) -> Vec<(String, String)> {
    text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.to_string(), v.to_string())).collect()
}
