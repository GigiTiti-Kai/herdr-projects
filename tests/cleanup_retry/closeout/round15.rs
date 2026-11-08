//! Independent round 15: old dangling-link replay, unreadable directories and artifact type changes.
use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::process::Command;

fn dangling_replay(n: i64, after_exit: bool) {
    let w = world(); on(); fixed(Ok(vec![]));
    let t = merged_with(&w, n, SAFE, |t| pane(&w, t, |p| p["agent"]["status"] = json!("working")));
    let wt = Path::new(t.s("worktree"));
    let file = wt.join(format!("c{n}.txt"));
    std::fs::write(&file, "saved version\n").unwrap();
    symlink("missing-target", wt.join("unreadable-link")).unwrap();
    let before = sw::land::closeout::changes_of(wt).expect("dangling link is readable by its target");
    pane(&w, &t, |p| p["agent"]["status"] = json!("idle")); pass(&w, t.s("id"));
    assert_eq!(closeouts(&w, &t), 1); accepted_now(&t);
    if after_exit { pass(&w, t.s("id")); assert_eq!(exits(&w, &t), 1); }
    std::fs::write(&file, "NEW UNSAVED VERSION\n").unwrap();
    assert_ne!(Some(before), sw::land::closeout::changes_of(wt));
    if after_exit { pane(&w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; }); }
    pass(&w, t.s("id"));
    let c = task(t.s("id"));
    let ok = if after_exit { open(&w, &t) } else { exits(&w, &t) == 0 };
    assert!(ok, "dangling replay after_exit={after_exit}; phase={}; msg={}", c.at("closeout").s("phase"), c.s("msg"));
    held(&c, "its branch moved since it was sent"); set_fake(None);
}
#[test]
fn round15_dangling_link_before_exit_original_replay() { dangling_replay(110, false); }
#[test]
fn round15_dangling_link_after_exit_original_replay() { dangling_replay(111, true); }

fn unreadable_directory(n: i64, after_exit: bool) {
    let w = world(); on(); fixed(Ok(vec![]));
    let t = merged_with(&w, n, SAFE, |t| pane(&w, t, |p| p["agent"]["status"] = json!("working")));
    let wt = Path::new(t.s("worktree"));
    let dir = wt.join("locked"); std::fs::create_dir(&dir).unwrap();
    let file = dir.join("work.txt"); std::fs::write(&file, "saved version\n").unwrap();
    // Locked before anything is sent: nothing is (the version is unknown). After /exit (adapted from the reviewer's
    // probe, which locked it first: that now holds before the closeout, so there would be no /exit to look after):
    // it is locked once /exit went in.
    if !after_exit { std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0)).unwrap(); }
    let list = Command::new("git").args(["ls-files", "--others", "--exclude-standard", "-z"]).current_dir(wt).output().unwrap();
    let before = sw::land::closeout::changes_of(wt);
    if !after_exit { assert_eq!(before, None, "git skips the locked folder with a warning only: the version is unknown"); }
    pane(&w, &t, |p| p["agent"]["status"] = json!("idle")); pass(&w, t.s("id"));
    accepted_now(&t);
    if after_exit { pass(&w, t.s("id")); assert_eq!(exits(&w, &t), 1); std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0)).unwrap(); }
    else { assert_eq!((closeouts(&w, &t), exits(&w, &t)), (0, 0)); held(&task(t.s("id")), "its worktree cannot be read"); }
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(&file, "NEW UNSAVED VERSION\n").unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0)).unwrap();
    let after = sw::land::closeout::changes_of(wt);
    if after_exit { pane(&w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; }); }
    pass(&w, t.s("id"));
    let c = task(t.s("id")); let space_open = open(&w, &t); let exit_count = exits(&w, &t);
    let restore = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    let preserved = std::fs::read_to_string(&file).unwrap_or_default();
    eprintln!("round15 unreadable_directory after_exit={after_exit}: git_exit={:?}; git_stderr={:?}; before={before:?}; after={after:?}; closeouts={}; exits={exit_count}; space_open={space_open}; phase={}; restore={restore:?}; preserved={preserved:?}; msg={}", list.status.code(), String::from_utf8_lossy(&list.stderr), closeouts(&w, &t), c.at("closeout").s("phase"), c.s("msg"));
    set_fake(None);
    if after_exit { assert!(space_open, "F21: unreadable directory and changed unsaved work must keep the space"); }
    else { assert_eq!(exit_count, 0, "F21: unreadable directory and changed unsaved work must prevent /exit"); }
}
#[test]
fn round15_unreadable_directory_before_exit_must_hold() { unreadable_directory(112, false); }
#[test]
fn round15_unreadable_directory_after_exit_must_keep_space() { unreadable_directory(113, true); }

fn artifact_type(n: i64, after_exit: bool) {
    let w = world(); on(); fixed(Ok(vec![]));
    let t = merged_with(&w, n, SAFE, |t| pane(&w, t, |p| p["agent"]["status"] = json!("working")));
    let wt = Path::new(t.s("worktree")); let file = wt.join("draft");
    symlink("missing-target", &file).unwrap();
    let before = sw::land::closeout::changes_of(wt);
    pane(&w, &t, |p| p["agent"]["status"] = json!("idle")); pass(&w, t.s("id"));
    accepted_now(&t);
    if after_exit { pass(&w, t.s("id")); assert_eq!(exits(&w, &t), 1); }
    std::fs::remove_file(&file).unwrap(); std::fs::write(&file, "missing-target").unwrap();
    let after = sw::land::closeout::changes_of(wt);
    assert!(!std::fs::symlink_metadata(&file).unwrap().file_type().is_symlink());
    if after_exit { pane(&w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; }); }
    pass(&w, t.s("id"));
    let c = task(t.s("id")); let space_open = open(&w, &t); let exit_count = exits(&w, &t);
    eprintln!("round15 artifact_type after_exit={after_exit}: before={before:?}; after={after:?}; closeouts={}; exits={exit_count}; space_open={space_open}; file_preserved={}; phase={}; msg={}", closeouts(&w, &t), file.exists(), c.at("closeout").s("phase"), c.s("msg"));
    set_fake(None);
    if after_exit { assert!(space_open, "changed artifact type after /exit must keep the space"); }
    else { assert_eq!(exit_count, 0, "changed artifact type must invalidate saved Exit decision"); }
}
#[test]
fn round15_symlink_replaced_with_regular_file_before_exit_must_hold() { artifact_type(114, false); }
#[test]
fn round15_symlink_replaced_with_regular_file_after_exit_must_keep_space() { artifact_type(115, true); }
