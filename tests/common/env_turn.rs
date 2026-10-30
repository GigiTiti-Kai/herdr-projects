//! The tests' turns at this process's environment, which every test file shares (they are one test program): a world,
//! or a test that sets the environment, takes it alone (`one_at_a_time`) and leaves it as the turn found it; a test that
//! only reads it, or starts a real program that does, takes a shared turn (`env_kept`), so no world moves HOME, PATH or
//! the rest under it. Between the turns it is the environment the tests started in, without herdr's session.
use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

static ONE: RwLock<()> = RwLock::new(());
static KEPT: OnceLock<HashMap<OsString, OsString>> = OnceLock::new();
// The turn this thread holds: a second one would wait for itself ("": none).
thread_local! { static HELD: Cell<&'static str> = const { Cell::new("") }; }

/// What the environment is between turns, made once (under the turn alone): the one the tests started in, less what
/// would reach the herdr, the board or the agents they run in, and the waits the tests set themselves (every
/// `HERDR_PROJECTS_…_MS`): one given from outside would hold in a test before it sets its own.
fn kept() -> &'static HashMap<OsString, OsString> {
    KEPT.get_or_init(|| {
        for k in ["HERDR_PANE_ID", "HERDR_TAB_ID", "HERDR_WORKSPACE_ID", "HERDR_PLUGIN_ID", "HERDR_PLUGIN_CONTEXT_JSON", "HERDR_ENV", "HERDR_SESSION", "TMUX", "NO_COLOR", "XDG_DATA_HOME", "HERDR_PROJECTS_DATA_DIR", "HERDR_PROJECTS_CONFIG_DIR", "HERDR_PROJECTS_REPORTS_DIR", "HERDR_PROJECTS_WEBUI", "HERDR_PROJECTS_PROGRESS", "HERDR_PROJECTS_SANDBOX", "HERDR_PROJECTS_BOXED", "HERDR_PROJECTS_COORDINATOR_DIR", "ELEVENLABS_API_KEY", "CLAUDE_CODE_TMPDIR", "HERDR_PROJECTS_SIZE"] { unsafe { std::env::remove_var(k) } }
        for (k, _) in std::env::vars_os() { if k.to_str().is_some_and(|k| k.starts_with("HERDR_PROJECTS_") && k.ends_with("_MS")) { unsafe { std::env::remove_var(&k) } } }
        // Not the real herdr, and a board of its own when nothing (scripts/run-tests.sh) gave one: a plain `cargo test` too.
        for (k, v) in [("HERDR_SOCKET_PATH", "/nonexistent"), ("HERDR_BIN_PATH", "/bin/false"), ("HERDR_PROJECTS_TESTING", "1")] { unsafe { std::env::set_var(k, v) } }
        if std::env::var_os("HERDR_PROJECTS_HOME").is_none() { unsafe { std::env::set_var("HERDR_PROJECTS_HOME", std::env::temp_dir().join(format!("sw-test-{}-board", std::process::id()))) } }
        std::env::vars_os().collect()
    })
}
/// The environment alone: it is as the turn found it again when the turn ends, a failed test's too.
pub struct Turn(RwLockWriteGuard<'static, ()>);
impl Drop for Turn {
    fn drop(&mut self) {
        HELD.set("");
        let kept = kept();
        for (k, _) in std::env::vars_os() { if !kept.contains_key(&k) { unsafe { std::env::remove_var(&k) } } }
        for (k, v) in kept { if std::env::var_os(k).as_ref() != Some(v) { unsafe { std::env::set_var(k, v) } } }
    }
}
pub fn one_at_a_time() -> Turn {
    assert_eq!(HELD.get(), "", "this test already holds a turn at the environment: a world or one_at_a_time takes none of its own");
    let g = ONE.write().unwrap_or_else(|e| e.into_inner());
    kept();
    HELD.set("alone");
    Turn(g)
}
/// A shared turn: the environment stays as it is between turns while it lasts.
pub struct Shared(RwLockReadGuard<'static, ()>);
impl Drop for Shared { fn drop(&mut self) { HELD.set(""); } }
/// For a test with no world (one with a world has the environment to itself already).
pub fn env_kept() -> Shared {
    assert_eq!(HELD.get(), "", "this test already holds a turn at the environment: env_kept is for a test with no world");
    if KEPT.get().is_none() { drop(one_at_a_time()); }
    let g = ONE.read().unwrap_or_else(|e| e.into_inner());
    HELD.set("shared");
    Shared(g)
}
