//! Independent round 16: hidden tracked edits and tracked executable permissions invalidate the accepted version.
use super::*;
use std::os::unix::fs::PermissionsExt;

fn hidden_tracked_edit(after_exit: bool, mode_only: bool) {
    let w = world(); on(); fixed(Ok(vec![]));
    let t = merged_with(&w, 120, SAFE, |t| pane(&w, t, |p| p["agent"]["status"] = json!("working")));
    let wt = Path::new(t.s("worktree"));
    let file = wt.join("c120.txt");
    if mode_only { git(wt, &["config", "core.filemode", "false"]); }
    else { git(wt, &["update-index", "--assume-unchanged", "c120.txt"]); }
    let before = sw::land::closeout::changes_of(wt).unwrap();
    pane(&w, &t, |p| p["agent"]["status"] = json!("idle")); pass(&w, t.s("id"));
    assert_eq!(closeouts(&w, &t), 1); accepted_now(&t);
    if after_exit { pass(&w, t.s("id")); assert_eq!(exits(&w, &t), 1); }
    if mode_only {
        let old = std::fs::metadata(&file).unwrap().permissions().mode();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(old ^ 0o100)).unwrap();
    } else { std::fs::write(&file, "NEW UNSAVED USER CONTENT\n").unwrap(); }
    let after = sw::land::closeout::changes_of(wt).unwrap();
    // The reviewer's probe asserted the two equal (git diff hides the change); read from disk, they differ now.
    assert_ne!(before, after, "the artifact differs, and its version says so though git diff would not");
    let closeouts_before_cleanup = closeouts(&w, &t);
    let exits_before_cleanup = exits(&w, &t);
    if after_exit { pane(&w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; }); }
    pass(&w, t.s("id"));
    let c = task(t.s("id"));
    let space_open = open(&w, &t); let exit_count = exits(&w, &t);
    let kept = std::fs::read_to_string(&file).ok();
    eprintln!("round16 hidden_tracked after_exit={after_exit} mode_only={mode_only}: fingerprint_same={}; closeouts_before_cleanup={closeouts_before_cleanup}; exits_before_cleanup={exits_before_cleanup}; exits_after_cleanup={exit_count}; space_open={space_open}; artifact={kept:?}; cleaned={}; phase={}; msg={}", before == after, c.b("cleaned"), c.at("closeout").s("phase"), c.s("msg"));
    set_fake(None);
    if after_exit { assert!(space_open, "changed tracked artifact must keep the space and worktree"); }
    else { assert_eq!(exit_count, 0, "changed tracked artifact must invalidate the accepted Exit decision"); }
}
#[test]
fn round16_assume_unchanged_before_exit() { hidden_tracked_edit(false, false); }
#[test]
fn round16_assume_unchanged_after_exit() { hidden_tracked_edit(true, false); }
#[test]
fn round16_tracked_mode_before_exit() { hidden_tracked_edit(false, true); }
#[test]
fn round16_tracked_mode_after_exit() { hidden_tracked_edit(true, true); }
