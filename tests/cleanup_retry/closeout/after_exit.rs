//! After `/exit` went in: what keeps the space all the same (the work moved, words typed at its shell and not sent,
//! a save accepted only after the wait was over). Reviewer probes of rounds 11 and 12, kept as they were written.
use super::*;

#[test]
fn round11_branch_tip_changed_after_exit_must_keep_the_space() {
    let w = world();
    on();
    let t = merged(&w, 70);
    accepted_now(&t);
    pass(&w, t.s("id"));
    assert_eq!(task(t.s("id")).at("closeout").s("phase"), "closing");
    let wt = Path::new(t.s("worktree"));
    std::fs::write(wt.join("later.txt"), "work after the accepted snapshot\n").unwrap();
    git(wt, &["add", "later.txt"]);
    git(wt, &["commit", "-qm", "new work after exit"]);
    let new_tip = git(wt, &["rev-parse", "HEAD"]);
    assert_ne!(new_tip, task(t.s("id")).at("closeout").s("tip"));
    pane(&w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; });
    pass(&w, t.s("id"));
    let c = task(t.s("id"));
    assert!(open(&w, &t), "F15: branch changed after /exit but space closed; phase={}, new work kept={}, msg={}", c.at("closeout").s("phase"), wt.join("later.txt").exists(), c.s("msg"));
    set_fake(None);
}

#[test]
fn round11_unsent_shell_input_after_agent_exit_must_keep_the_space() {
    let w = world();
    on();
    let t = merged(&w, 71);
    accepted_now(&t);
    pass(&w, t.s("id"));
    assert_eq!(task(t.s("id")).at("closeout").s("phase"), "closing");
    pane(&w, &t, |p| {
        p["agent"] = Value::Null;
        p["frontPid"] = Value::Null;
        p["screen"] = json!("user@host:/work$ printf 'UNSENT SHELL DRAFT'");
    });
    let api = sw::herdr::Herdr::new(None);
    assert!(api.pane_idle(t.at("link").s("pane")), "no running command in this fixture");
    assert!(api.pane_read(t.at("link").s("pane"), "visible", None).unwrap().contains("UNSENT SHELL DRAFT"));
    pass(&w, t.s("id"));
    let c = task(t.s("id"));
    assert!(open(&w, &t), "F16: unsent shell input lost; cleaned={}, phase={}, msg={}", c.b("cleaned"), c.at("closeout").s("phase"), c.s("msg"));
    set_fake(None);
}

#[test]
fn round11_receipt_accepted_after_deadline_must_not_send_exit() {
    let w = world();
    on();
    fixed(Ok(vec![]));
    let t = merged(&w, 72);
    assert_eq!(task(t.s("id")).at("closeout").s("phase"), "checkpointing");
    sw::clock::advance(LATE);
    fixed(Ok(vec![job("late", t.s("worktree"), sw::clock::now() - 1)]));
    pass(&w, t.s("id"));
    let c = task(t.s("id"));
    assert_eq!(exits(&w, &t), 0, "F17: receipt first accepted after closeoutWait still sent /exit; phase={}, msg={}", c.at("closeout").s("phase"), c.s("msg"));
    set_fake(None);
}

#[test]
fn round11_a_shell_is_empty_only_at_a_bare_prompt() {
    for (screen, empty) in [("user@host:/w$ claude x\nuser@host:/w$ ", true), ("~/w ❯ claude\n~/w ❯ ", true), ("user@host:/w$ ", false), ("user@host:/w$ claude x\nuser@host:/w$ printf 'UNSENT'", false), ("user@host:/w$ echo UNSENT$ ok\nuser@host:/w$ echo UNSENT$", false), ("> ", false), ("", false)] {
        assert_eq!(sw::land::closeout::shell_at_prompt(screen, "claude"), empty, "{screen:?}");
    }
}


fn shell_draft(screen: &str, n: i64) {
    let w = world();
    on();
    let t = merged(&w, n);
    accepted_now(&t);
    pass(&w, t.s("id"));
    assert_eq!(task(t.s("id")).at("closeout").s("phase"), "closing");
    pane(&w, &t, |p| {
        p["agent"] = Value::Null;
        p["frontPid"] = Value::Null;
        p["screen"] = json!(screen);
    });
    let api = sw::herdr::Herdr::new(None);
    assert!(api.pane_idle(t.at("link").s("pane")));
    assert_eq!(api.pane_read(t.at("link").s("pane"), "visible", None).unwrap(), screen);
    pass(&w, t.s("id"));
    let c = task(t.s("id"));
    let space_open = open(&w, &t);
    let checkout_exists = Path::new(t.s("worktree")).exists();
    set_fake(None);
    assert!(space_open, "F18: draft={screen:?}; shell_empty={}; space_open={space_open}; checkout_exists={checkout_exists}; cleaned={}; phase={}; msg={}", sw::land::closeout::shell_empty(screen), c.b("cleaned"), c.at("closeout").s("phase"), c.s("msg"));
}
#[test]
fn round12_unsent_shell_hash_suffix_keeps_space() { shell_draft("user@host:/work$ echo UNSENT#", 80); }
#[test]
fn round12_unsent_shell_dollar_suffix_keeps_space() { shell_draft("user@host:/work$ echo UNSENT$", 81); }
#[test]
fn round12_unsent_shell_percent_suffix_keeps_space() { shell_draft("user@host:/work$ printf 50%", 82); }
#[test]
fn round12_unsent_shell_chevron_suffix_keeps_space() { shell_draft("user@host:/work$ echo ❯", 83); }

#[test]
fn round12_detached_new_head_after_exit_keeps_space() {
    let w = world();
    on();
    let t = merged(&w, 84);
    accepted_now(&t);
    pass(&w, t.s("id"));
    assert_eq!(task(t.s("id")).at("closeout").s("phase"), "closing");
    let wt = Path::new(t.s("worktree"));
    let old_tip = task(t.s("id")).at("closeout").t("tip");
    git(wt, &["checkout", "--detach", "-q"]);
    std::fs::write(wt.join("later.txt"), "new post-exit artifact\n").unwrap();
    git(wt, &["add", "later.txt"]);
    git(wt, &["commit", "-qm", "new work detached after exit"]);
    let new_head = git(wt, &["rev-parse", "HEAD"]);
    assert_ne!(new_head, old_tip);
    let task_ref = git(wt, &["rev-parse", &format!("refs/heads/{}", t.s("branch"))]);
    assert_eq!(task_ref, old_tip, "the saved task ref is unchanged but the actual artifact changed");
    pane(&w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; });
    pass(&w, t.s("id"));
    let c = task(t.s("id"));
    let space_open = open(&w, &t);
    let checkout_exists = wt.exists();
    set_fake(None);
    assert!(space_open, "F17: old_tip={old_tip}; new_head={new_head}; space_open={space_open}; checkout_exists={checkout_exists}; cleaned={}; phase={}; msg={}", c.b("cleaned"), c.at("closeout").s("phase"), c.s("msg"));
}


fn dirty_snapshot(n: i64, tracked: bool, after_exit: bool) {
    let w = world();
    on();
    fixed(Ok(vec![]));
    let t = merged_with(&w, n, SAFE, |t| pane(&w, t, |p| p["agent"]["status"] = json!("working")));
    assert_eq!(closeouts(&w, &t), 0);
    let wt = Path::new(t.s("worktree"));
    let file = wt.join(if tracked { format!("c{n}.txt") } else { "draft.txt".into() });
    std::fs::write(&file, "saved version\n").unwrap();
    let before_status = git(wt, &["status", "--porcelain=v1", "-uall"]);
    let before_head = git(wt, &["rev-parse", "HEAD"]);
    pane(&w, &t, |p| p["agent"]["status"] = json!("idle"));
    pass(&w, t.s("id"));
    assert_eq!(closeouts(&w, &t), 1);
    assert_eq!(task(t.s("id")).at("closeout").s("phase"), "checkpointing");
    accepted_now(&t);
    if after_exit {
        pass(&w, t.s("id"));
        assert_eq!(task(t.s("id")).at("closeout").s("phase"), "closing");
    }
    std::fs::write(&file, "NEW UNSAVED VERSION\n").unwrap();
    let after_status = git(wt, &["status", "--porcelain=v1", "-uall"]);
    assert_eq!(before_status, after_status, "porcelain records paths and flags, not file bytes");
    assert_eq!(before_head, git(wt, &["rev-parse", "HEAD"]));
    if after_exit { pane(&w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; }); }
    pass(&w, t.s("id"));
    let c = task(t.s("id"));
    let space_open = open(&w, &t);
    let exit_count = exits(&w, &t);
    eprintln!("round13 tracked={tracked} after_exit={after_exit}: status_before={before_status:?}; status_after={after_status:?}; exit_count={exit_count}; space_open={space_open}; phase={}; file_preserved={}; msg={}", c.at("closeout").s("phase"), file.exists(), c.s("msg"));
    set_fake(None);
    if after_exit { assert!(space_open, "new artifact bytes after /exit must invalidate the old decision and keep the pane"); }
    else { assert_eq!(exit_count, 0, "new artifact bytes before /exit must invalidate the old decision"); }
}
#[test]
fn round13_tracked_dirty_bytes_after_exit_keep_space() { dirty_snapshot(90, true, true); }
#[test]
fn round13_untracked_dirty_bytes_after_exit_keep_space() { dirty_snapshot(91, false, true); }
#[test]
fn round13_tracked_dirty_bytes_before_exit_send_nothing() { dirty_snapshot(92, true, false); }
