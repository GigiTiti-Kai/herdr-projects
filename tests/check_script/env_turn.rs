//! The tests' turns at the environment (tests/common/env_turn.rs): a world leaves it as it found it, what a test set or
//! took out in it too, so the next test, a world or a real program, never starts in what the last one left behind.
use crate::common;
use std::collections::BTreeMap;

fn now() -> BTreeMap<std::ffi::OsString, std::ffi::OsString> { std::env::vars_os().collect() }

#[test]
fn a_world_leaves_the_environment_as_it_found_it_and_herdrs_session_is_never_in_it() {
    let before = { let _env = common::env_kept(); now() };
    assert_eq!(before.get(std::ffi::OsStr::new("HERDR_SOCKET_PATH")).and_then(|v| v.to_str()), Some("/nonexistent"), "between turns no test reaches the real herdr");
    for k in ["HERDR_ENV", "HERDR_PANE_ID", "TMUX"] { assert!(!before.contains_key(std::ffi::OsStr::new(k)), "{k} is herdr's session's"); }
    let waits: Vec<_> = before.keys().filter_map(|k| k.to_str()).filter(|k| k.starts_with("HERDR_PROJECTS_") && k.ends_with("_MS")).collect();
    assert!(waits.is_empty(), "the waits are each test's own, none from outside: {waits:?}");
    {
        let w = common::world();
        assert_eq!(std::env::var("HOME").unwrap(), w.base.join("home").display().to_string());
        unsafe { std::env::set_var("HERDR_PROJECTS_REOPEN_MS", "0"); std::env::set_var("PATH", w.base.join("bin")); std::env::remove_var("USER"); }
    }
    let _env = common::env_kept();
    // By name only: the values can be secrets of the one who runs the tests.
    let after = now();
    let moved: Vec<_> = before.keys().chain(after.keys()).filter(|k| before.get(*k) != after.get(*k)).collect();
    assert!(moved.is_empty(), "left behind by the world: {moved:?}");
}
