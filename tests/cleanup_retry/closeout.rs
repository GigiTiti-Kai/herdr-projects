//! The record-only closeout before a done task's space is closed (`workspace.closeout`, land/closeout.rs): off, nothing
//! is typed; on, a `/session-close` that ships nothing goes in once, the task stays done, and the space closes only once
//! Agent Memory's queue (a fake here) holds a job accepted for that session and worktree; what cannot be proved keeps it.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use std::rc::Rc;
use sw::json::*;
use sw::memory_queue::{set_fake, Job};
use sw::store::tasks::{update, Change};

fn pass(w: &World, id: &str) -> usize {
    let _ = std::fs::remove_dir_all(sw::store::files::data_dir().join("stamps"));
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let jobs: Vec<(String, String, Vec<String>)> = sw::runner::JOBS.with(|j| std::mem::take(&mut *j.borrow_mut())).into_iter().filter(|(job, i, _)| job == "cleanup" && i == id).collect();
    for (_, _, extra) in &jobs { sw::land::cleanup_job(id, if extra.iter().any(|e| e == "--merged") { "merged" } else { "always" }); }
    jobs.len()
}
/// A task through its approval, with its agent's session known, and merged: the clean-up that follows is the test's.
fn merged(w: &World, n: i64) -> Obj {
    w.script(json!({ "worker": [{ "files": { format!("c{n}.txt"): "x\n" }, "commit": format!("c{n}"), "note": "Did it." }] }));
    let id = w.add(&format!("Task {n}"));
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    update(&id, |t| { t.put("session", format!("s-{n}")); Ok(Change::Write) }).unwrap();
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    t
}
fn closeouts(w: &World, t: &Obj) -> usize { prompts(w, t.at("link").s("pane")).iter().filter(|p| p.contains("session-close")).count() }
fn open(w: &World, t: &Obj) -> bool { w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == t.at("link").s("workspace")) }
fn on() { sw::store::settings::change::set_value("workspace.closeout", &json!(true), &sw::store::settings::lookup::Scope::you()).unwrap(); }
fn queue(jobs: Rc<std::cell::RefCell<Result<Vec<Job>, String>>>) { set_fake(Some(Rc::new(move |_| jobs.borrow().clone()))); }
fn job(id: &str, cwd: &str, at: i64) -> Job { Job { id: id.into(), status: "queued".into(), accepted: at, cwd: cwd.into() } }

#[test]
fn off_by_default_nothing_is_typed_and_the_space_closes_as_ever() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let t = merged(&w, 1);
    assert_eq!(closeouts(&w, &t), 0);
    assert!(task(t.s("id")).b("cleaned") && !open(&w, &t));
}

fn pane(w: &World, t: &Obj, f: impl FnOnce(&mut serde_json::Value)) { let id = t.at("link").t("pane"); w.patch(|s| { let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == id).unwrap(); f(p); }); }
fn exits(w: &World, t: &Obj) -> usize { prompts(w, t.at("link").s("pane")).iter().filter(|p| p.as_str() == "/exit").count() }
fn held(c: &Obj, why: &str) { assert!(c.s("msg").contains(&format!("closeout 未確認: {why}")), "{}", c.s("msg")); assert!(!c.b("tidy") && !c.b("cleaned") && c.arr("kept").iter().any(|k| text(k).contains("closeout 未確認")) && Path::new(c.s("worktree")).exists(), "{c:?}"); }
fn fixed(r: Result<Vec<Job>, String>) { queue(Rc::new(std::cell::RefCell::new(r))); }
const LATE: i64 = 11 * 60 * 1000;

#[test]
fn on_each_step_is_proved_apart_save_then_exit_then_the_agent_gone_then_the_space_closed() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    on();
    let jobs = Rc::new(std::cell::RefCell::new(Ok(vec![])));
    queue(jobs.clone());
    let t = merged(&w, 2);
    let id = t.t("id");
    let c = task(&id);
    // typed once, as the board's own turn: the task stays done, nothing reviewed, merged or pushed again
    assert_eq!(closeouts(&w, &t), 1);
    let last = prompts(&w, t.at("link").s("pane")).last().cloned().unwrap();
    assert!(last.starts_with("/session-close 記録だけの session-close。ship・push・merge・PR・再 review・削除はしない。"), "{last}");
    assert_eq!(c.s("stage"), "done"); assert_eq!(c.at("closeout").s("phase"), "checkpointing"); assert_eq!(c.at("closeout").s("session"), "s-2");
    assert!(c.s("msg").contains("Closeout "), "{}", c.s("msg"));
    assert!(open(&w, &c) && Path::new(c.s("worktree")).exists() && c.b("tidy"));
    assert_eq!(c.i("reviewRounds"), t.i("reviewRounds")); assert_eq!(c.at("mergedAt"), t.at("mergedAt"));
    assert_ne!(c.at("last").at("comp"), c.at("closeout").at("comp0"), "the closeout turn's end was seen by the board, and changed nothing");
    // no job yet, one from another folder, one accepted a moment before it was sent: nothing typed, nothing closed
    let sent = c.at("closeout").i("sentAt");
    assert_eq!(pass(&w, &id), 1);
    *jobs.borrow_mut() = Ok(vec![job("else", "/elsewhere", sw::clock::now()), job("old", c.s("worktree"), sent - 1), job("older", c.s("worktree"), sent - 4000)]);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!((closeouts(&w, &t), exits(&w, &t)), (1, 0));
    // accepted, but a command still runs under the agent: saved, and no exit yet; the job is looked up again each pass
    pane(&w, &t, |p| p["tool"] = json!(true));
    *jobs.borrow_mut() = Ok(vec![job("j1", c.s("worktree"), sent)]);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!(task(&id).at("closeout").s("phase"), "saved"); assert_eq!(exits(&w, &t), 0);
    pane(&w, &t, |p| p["tool"] = json!(false));
    *jobs.borrow_mut() = Ok(vec![Job { status: "discarded".into(), ..job("j1", c.s("worktree"), sent) }]);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!(exits(&w, &t), 0, "a save given up on since proves nothing");
    // the exit, decided now: typed once; the space stays until herdr sees the agent gone
    *jobs.borrow_mut() = Ok(vec![job("j1", c.s("worktree"), sent)]);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!(task(&id).at("closeout").s("phase"), "closing"); assert_eq!(exits(&w, &t), 1);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!(exits(&w, &t), 1, "never typed twice");
    assert!(open(&w, &task(&id)));
    // gone, but its shell is busy: still kept; then free: closed, its line as it was
    pane(&w, &t, |p| { p["agent"] = serde_json::Value::Null; p["busy"] = json!(true); });
    assert_eq!(pass(&w, &id), 1);
    assert_eq!(task(&id).at("closeout").s("phase"), "closing"); assert!(open(&w, &task(&id)));
    pane(&w, &t, |p| p["busy"] = json!(false));
    assert_eq!(pass(&w, &id), 1);
    let c = task(&id);
    assert!(c.b("cleaned") && !open(&w, &c) && !Path::new(t.s("worktree")).exists(), "{}", c.s("msg"));
    assert_eq!(c.at("closeout").s("phase"), "pane_closed"); assert_eq!(c.at("closeout").s("job"), "j1"); assert!(c.at("closeout").b("exitedAt"));
    assert_eq!(c.s("msg"), "Merged into main.");
    set_fake(None);
}

#[test]
fn what_cannot_be_proved_keeps_the_space_with_the_reason_and_types_nothing_more() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    on();
    fixed(Err("no Agent Memory queue".into()));
    // the queue cannot be read: waits, then after closeoutWait is held, and not tried again by itself
    let t = merged(&w, 3);
    let id = t.t("id");
    assert_eq!(pass(&w, &id), 1);
    sw::clock::advance(LATE);
    assert_eq!(pass(&w, &id), 1);
    held(&task(&id), "no Agent Memory queue");
    assert_eq!(pass(&w, &id), 0);
    assert_eq!((closeouts(&w, &t), exits(&w, &t)), (1, 0));
    // a turn of yours after the closeout's own: an accepted job proves nothing, and nothing more is typed
    let y = merged(&w, 5);
    let yid = y.t("id");
    assert_eq!(pass(&w, &yid), 1);
    assert!(task(&yid).at("closeout").has("comp1"));
    sw::herdr::Herdr::new(None).agent_prompt(y.at("link").s("pane"), "one more thing").unwrap();
    settle(&yid);
    fixed(Ok(vec![job("y", y.s("worktree"), sw::clock::now())]));
    assert_eq!(pass(&w, &yid), 1);
    held(&task(&yid), "new input reached its agent");
    assert_eq!((closeouts(&w, &y), exits(&w, &y)), (1, 0));
    // the session changed since it was sent
    let n = merged(&w, 4);
    update(n.s("id"), |t| { t.put("session", "s-other"); Ok(Change::Write) }).unwrap();
    assert_eq!(pass(&w, n.s("id")), 1);
    held(&task(n.s("id")), "its agent's session changed");
    assert_eq!(closeouts(&w, &n), 1);
    // the agent left its pane before it was told to exit (a crash): unknown, never a finished closeout
    let g = merged(&w, 6);
    fixed(Ok(vec![job("g", g.s("worktree"), sw::clock::now())]));
    pane(&w, &g, |p| p["agent"] = serde_json::Value::Null);
    assert_eq!(pass(&w, g.s("id")), 1);
    held(&task(g.s("id")), "its agent left its pane before it was told to exit");
    set_fake(None);
}

#[test]
fn a_dialog_a_turn_that_never_ends_or_herdr_not_answering_never_close_anything() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    on();
    fixed(Ok(vec![]));
    // a dialog in front of the agent: nothing is typed at all
    w.script(json!({ "worker": [{ "files": { "d.txt": "x\n" }, "commit": "d", "note": "Did it." }] }));
    let d = w.add("Dialog");
    delegate(&d);
    assert_eq!(settle(&d).s("stage"), "approval");
    update(&d, |t| { t.put("session", "s-d"); Ok(Change::Write) }).unwrap();
    let dt = task(&d);
    pane(&w, &dt, |p| p["agent"]["status"] = json!("blocked"));
    let before = prompts(&w, dt.at("link").s("pane")).len();
    sw::actions::finish_line::approve(&d, None, false).unwrap();
    let c = settle(&d);
    assert_eq!(c.s("stage"), "done");
    assert_eq!(prompts(&w, dt.at("link").s("pane")).len(), before);
    held(&c, "its agent is blocked");
    // sent, then its turn never ends: held once closeoutWait has passed
    let k = merged(&w, 7);
    pane(&w, &k, |p| p["agent"]["status"] = json!("working"));
    assert_eq!(pass(&w, k.s("id")), 1);
    assert!(!task(k.s("id")).b("kept"), "{:?}", task(k.s("id")).at("kept"));
    sw::clock::advance(LATE);
    assert_eq!(pass(&w, k.s("id")), 1);
    held(&task(k.s("id")), "its agent's turn is under way");
    // sent, then herdr does not answer: an error is not a pane gone; held once closeoutWait has passed
    let e = merged(&w, 8);
    w.patch(|s| s["agentGetFails"] = json!(true));
    sw::land::cleanup_job(e.s("id"), "merged");
    assert!(!task(e.s("id")).b("kept") && open(&w, &task(e.s("id"))));
    sw::clock::advance(LATE);
    sw::land::cleanup_job(e.s("id"), "merged");
    w.patch(|s| s["agentGetFails"] = json!(false));
    held(&task(e.s("id")), "herdr did not answer");
    assert_eq!(closeouts(&w, &e), 1);
    set_fake(None);
}
