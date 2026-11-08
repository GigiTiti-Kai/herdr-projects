//! The record-only closeout before a done task's space is closed (`workspace.closeout`, land/closeout.rs): off, nothing
//! is typed; on, a `/session-close` that ships nothing goes in once, the task stays done, and each step is proved apart
//! (the save in Agent Memory's queue, a fake here; the agent's own reply with its Exit decision; `/exit`; its process
//! ended and its pane free) before the space closes. What cannot be proved keeps it, with the reason.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use std::rc::Rc;
use sw::json::*;
use sw::memory_queue::{set_fake, Job};
use sw::store::tasks::{update, Change};

/// What it never takes for leave to close.
mod guards;

const SAFE: &str = "Saved: accepted.\nsafe to Exit";
const LATE: i64 = 11 * 60 * 1000;
fn pass(w: &World, id: &str) -> usize {
    let _ = std::fs::remove_dir_all(sw::store::files::data_dir().join("stamps"));
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let jobs: Vec<(String, String, Vec<String>)> = sw::runner::JOBS.with(|j| std::mem::take(&mut *j.borrow_mut())).into_iter().filter(|(job, i, _)| job == "cleanup" && i == id).collect();
    for (_, _, extra) in &jobs { sw::land::cleanup_job(id, if extra.iter().any(|e| e == "--merged") { "merged" } else { "always" }); }
    jobs.len()
}
fn pane(w: &World, t: &Obj, f: impl FnOnce(&mut Value)) { let id = t.at("link").t("pane"); w.patch(|s| { let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == id).unwrap(); f(p); }); }
/// A task through its approval, its agent's process known, and merged; the agent's next turn (the closeout's) says `reply`.
fn merged_with(w: &World, n: i64, reply: &str, before: impl FnOnce(&Obj)) -> Obj {
    w.script(json!({ "worker": [{ "files": { format!("c{n}.txt"): "x\n" }, "commit": format!("c{n}"), "note": "Did it." }, { "said": reply }] }));
    let id = w.add(&format!("Task {n}"));
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(t.b("session"), "its agent's session is known");
    // its agent's process and session as herdr shows them, its input box empty
    pane(w, &t, |p| { p["frontPid"] = json!(4000 + n); p["agent"]["session"] = json!(t.s("session")); p["agent"]["screen"] = json!("Done.\n╭──\n│ ❯ \n╰──"); });
    before(&t);
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    t
}
fn merged(w: &World, n: i64) -> Obj { merged_with(w, n, SAFE, |_| {}) }
fn said(w: &World, t: &Obj, what: &str) -> usize { prompts(w, t.at("link").s("pane")).iter().filter(|p| p.contains(what)).count() }
fn closeouts(w: &World, t: &Obj) -> usize { said(w, t, "session-close") }
fn exits(w: &World, t: &Obj) -> usize { prompts(w, t.at("link").s("pane")).iter().filter(|p| p.as_str() == "/exit").count() }
fn open(w: &World, t: &Obj) -> bool { w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == t.at("link").s("workspace")) }
fn on() { sw::store::settings::change::set_value("workspace.closeout", &json!(true), &sw::store::settings::lookup::Scope::you()).unwrap(); }
fn queue(jobs: Rc<std::cell::RefCell<Result<Vec<Job>, String>>>) { set_fake(Some(Rc::new(move |_| jobs.borrow().clone()))); }
fn fixed(r: Result<Vec<Job>, String>) { queue(Rc::new(std::cell::RefCell::new(r))); }
fn job(id: &str, cwd: &str, at: i64) -> Job { Job { id: id.into(), status: "queued".into(), accepted: at, cwd: cwd.into() } }
fn accepted_now(t: &Obj) { fixed(Ok(vec![job("ok", t.s("worktree"), sw::clock::now() + 1)])); }
fn held(c: &Obj, why: &str) { assert!(c.s("msg").contains(&format!("closeout 未確認: {why}")), "{}", c.s("msg")); assert!(!c.b("tidy") && !c.b("cleaned") && c.arr("kept").iter().any(|k| text(k).contains("closeout 未確認")) && Path::new(c.s("worktree")).exists(), "{c:?}"); }
/// A turn of yours in the agent's own terminal, kept in its saved conversation as every turn is.
fn yours(w: &World, t: &Obj) { w.script(json!({ "worker": [{ "said": "Did one more thing." }] })); sw::herdr::Herdr::new(None).agent_prompt(t.at("link").s("pane"), "one more thing").unwrap(); }
/// The clock stands still unless a test moves it: a wall clock that steps back (WSL under load) would make a reply look
/// older than the closeout it answers.
fn world() -> World { let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() }); sw::clock::set(Some(sw::clock::now())); w }

#[test]
fn off_by_default_nothing_is_typed_and_the_space_closes_as_ever() {
    let w = world();
    let t = merged(&w, 1);
    assert_eq!(closeouts(&w, &t), 0);
    assert!(task(t.s("id")).b("cleaned") && !open(&w, &t));
}

#[test]
fn on_each_step_is_proved_apart_save_exit_decision_exit_process_ended_then_the_space_closed() {
    let w = world();
    on();
    let jobs = Rc::new(std::cell::RefCell::new(Ok(vec![])));
    queue(jobs.clone());
    let t = merged(&w, 2);
    let id = t.t("id");
    let c = task(&id);
    // typed once, as the board's own turn: the task stays done, nothing reviewed, merged or pushed again
    assert_eq!(closeouts(&w, &t), 1);
    assert!(said(&w, &t, "/session-close 記録だけの session-close。ship・push・merge・PR・再 review・削除はしない。") == 1);
    assert_eq!(c.s("stage"), "done"); assert_eq!(c.at("closeout").s("phase"), "checkpointing"); assert_eq!(c.at("closeout").s("session"), c.s("session"));
    assert!(c.s("msg").contains("Closeout "), "{}", c.s("msg"));
    assert!(open(&w, &c) && Path::new(c.s("worktree")).exists() && c.b("tidy"));
    assert_eq!(c.i("reviewRounds"), t.i("reviewRounds")); assert_eq!(c.at("mergedAt"), t.at("mergedAt"));
    // no job yet; one from another folder; one accepted before it was sent, or in the same millisecond: nothing more
    let sent = c.at("closeout").i("sentAt");
    assert_eq!(pass(&w, &id), 1);
    *jobs.borrow_mut() = Ok(vec![job("else", "/elsewhere", sent + 9), job("old", c.s("worktree"), sent - 4000), job("same", c.s("worktree"), sent)]);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!((closeouts(&w, &t), exits(&w, &t)), (1, 0));
    assert!(task(&id).at("closeout").b("turn"), "its reply to the closeout is known");
    // accepted, but a command still runs under the agent: saved, no exit; the job is looked up again on every pass
    pane(&w, &t, |p| p["child"] = json!(true));
    *jobs.borrow_mut() = Ok(vec![job("j1", c.s("worktree"), sent + 1)]);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!(task(&id).at("closeout").s("phase"), "saved"); assert_eq!(exits(&w, &t), 0);
    pane(&w, &t, |p| p["child"] = json!(false));
    *jobs.borrow_mut() = Ok(vec![Job { status: "discarded".into(), ..job("j1", c.s("worktree"), sent + 1) }]);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!(exits(&w, &t), 0, "a save given up on since proves nothing");
    // the exit, decided now: typed once, with the agent's process noted
    *jobs.borrow_mut() = Ok(vec![job("j1", c.s("worktree"), sent + 1)]);
    assert_eq!(pass(&w, &id), 1);
    let r = task(&id).at("closeout").clone();
    assert_eq!(r.s("phase"), "closing"); assert_eq!(r.at("pids"), &json!([4002])); assert_eq!(exits(&w, &t), 1);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!(exits(&w, &t), 1, "never typed twice");
    // herdr sees no agent, but its process still runs; then its shell is busy; then free: closed, its line as it was
    pane(&w, &t, |p| p["agent"] = Value::Null);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!(task(&id).at("closeout").s("phase"), "closing"); assert!(task(&id).s("msg").contains("process has not ended"), "{}", task(&id).s("msg"));
    pane(&w, &t, |p| { p["frontPid"] = Value::Null; p["busy"] = json!(true); });
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
fn new_input_another_session_no_exit_decision_or_an_agent_gone_hold_it_and_type_nothing_more() {
    let w = world();
    on();
    // a turn of yours right after the closeout's, before the board's first look (no hook needed to tell): its last
    // reply is not the closeout's, so an accepted job proves nothing
    let y = merged(&w, 3);
    yours(&w, &y);
    settle(y.s("id"));
    update(y.s("id"), |t| { t.obj_mut("closeout").remove("turn"); Ok(Change::Write) }).unwrap(); // as if the board had not looked yet
    accepted_now(&y);
    assert_eq!(pass(&w, y.s("id")), 1);
    held(&task(y.s("id")), "new input reached its agent after it was sent (its last turn answers something else)");
    assert_eq!((closeouts(&w, &y), exits(&w, &y)), (1, 0));
    assert_eq!(pass(&w, y.s("id")), 0, "held: not tried again by itself");
    // and one after the board's first look: another reply than the one it saw
    fixed(Ok(vec![]));
    let z = merged(&w, 4);
    assert_eq!(pass(&w, z.s("id")), 1);
    assert!(task(z.s("id")).at("closeout").b("turn"));
    yours(&w, &z);
    settle(z.s("id"));
    accepted_now(&z);
    assert_eq!(pass(&w, z.s("id")), 1);
    held(&task(z.s("id")), "new input reached its agent");
    assert_eq!(exits(&w, &z), 0);
    // its reply says the Exit is blocked
    let b = merged_with(&w, 5, "Save complete; Exit blocked: a watcher of mine still runs.", |_| {});
    accepted_now(&b);
    pass(&w, b.s("id")); // held at its first look already
    held(&task(b.s("id")), "its reply gives no Exit decision");
    // "unsafe to Exit" is no Exit decision, nor is "not safe to Exit"
    for (n, reply) in [(14, "Saved, but unsafe to Exit: a watcher runs."), (15, "It is not safe to Exit yet.")] {
        let u = merged_with(&w, n, reply, |_| {});
        accepted_now(&u);
        pass(&w, u.s("id"));
        held(&task(u.s("id")), "its reply gives no Exit decision");
        assert_eq!(exits(&w, &u), 0);
    }
    // the session changed since it was sent
    let n = merged(&w, 6);
    update(n.s("id"), |t| { t.put("session", "s-other"); Ok(Change::Write) }).unwrap();
    assert_eq!(pass(&w, n.s("id")), 1);
    held(&task(n.s("id")), "another session than its own is in its pane");
    // the agent left its pane before it was told to exit (a crash): never a finished closeout
    let g = merged(&w, 7);
    accepted_now(&g);
    pane(&w, &g, |p| p["agent"] = Value::Null);
    assert_eq!(pass(&w, g.s("id")), 1);
    held(&task(g.s("id")), "its agent left its pane before it was told to exit");
    // the queue cannot be read: held once closeoutWait has passed
    fixed(Err("no Agent Memory queue".into()));
    let q = merged(&w, 8);
    assert_eq!(pass(&w, q.s("id")), 1);
    sw::clock::advance(LATE);
    assert_eq!(pass(&w, q.s("id")), 1);
    held(&task(q.s("id")), "no Agent Memory queue");
    set_fake(None);
}

#[test]
fn a_dialog_a_turn_that_never_ends_or_herdr_not_answering_never_close_anything_before_or_after_it_was_sent() {
    let w = world();
    on();
    fixed(Ok(vec![]));
    // a dialog in front of the agent: nothing is typed at all
    let d = merged_with(&w, 9, SAFE, |t| pane(&w, t, |p| p["agent"]["status"] = json!("blocked")));
    assert_eq!(closeouts(&w, &d), 0);
    held(&task(d.s("id")), "its agent is blocked");
    // a turn under way from the first look on, never sent: held once closeoutWait has passed since that look
    let k = merged_with(&w, 10, SAFE, |t| pane(&w, t, |p| p["agent"]["status"] = json!("working")));
    assert!(!task(k.s("id")).b("kept"), "{:?}", task(k.s("id")).at("kept"));
    sw::clock::advance(LATE);
    assert_eq!(pass(&w, k.s("id")), 1);
    held(&task(k.s("id")), "its agent's turn is under way");
    assert_eq!(closeouts(&w, &k), 0);
    // sent, then its turn never ends
    let m = merged(&w, 11);
    pane(&w, &m, |p| p["agent"]["status"] = json!("working"));
    assert_eq!(pass(&w, m.s("id")), 1);
    sw::clock::advance(LATE);
    assert_eq!(pass(&w, m.s("id")), 1);
    held(&task(m.s("id")), "its agent's turn is under way");
    // herdr does not answer, before anything was sent and after: an error is not a pane gone
    for (n, sent) in [(12, false), (13, true)] {
        let e = merged_with(&w, n, SAFE, |_| if !sent { w.patch(|s| s["agentGetFails"] = json!(true)); });
        w.patch(|s| s["agentGetFails"] = json!(true));
        sw::land::cleanup_job(e.s("id"), "merged");
        assert!(!task(e.s("id")).b("kept") && open(&w, &task(e.s("id"))));
        sw::clock::advance(LATE);
        sw::land::cleanup_job(e.s("id"), "merged");
        w.patch(|s| s["agentGetFails"] = json!(false));
        held(&task(e.s("id")), "herdr did not answer");
        assert_eq!(closeouts(&w, &e), usize::from(sent));
    }
    set_fake(None);
}

#[test]
fn what_is_typed_is_written_down_first_and_a_record_that_cannot_be_written_types_nothing_or_nothing_again() {
    let w = world();
    on();
    fixed(Ok(vec![]));
    let t = merged_with(&w, 16, SAFE, |t| pane(&w, t, |p| p["agent"]["status"] = json!("working"))); // nothing sent yet
    let c = task(t.s("id"));
    let who = || sw::land::closeout::Who { api: sw::herdr::Herdr::new(None), pane: c.at("link").t("pane"), terminal: c.at("link").t("terminal"), session: c.t("session"), transcript: String::new(), agent: c.t("agent"), cwd: c.s("worktree").into(), tip: String::new(), events: 0, wait_ms: 600000 };
    pane(&w, &t, |p| p["agent"]["status"] = json!("idle"));
    let g = sw::land::closeout::step(&who(), &json!({ "waitFrom": sw::clock::now() }), &|_, _| false);
    assert!(matches!(&g, sw::land::closeout::Gate::Wait(m) if m.contains("could not be written")), "{g:?}");
    assert_eq!(closeouts(&w, &t), 0, "not typed while its record cannot be written");
    for phase in ["sending", "exiting"] {
        let g = sw::land::closeout::step(&who(), &json!({ "phase": phase, "sentAt": 1 }), &|_, _| true);
        assert!(matches!(&g, sw::land::closeout::Gate::Held(m) if m.contains("is not known")), "{g:?}");
    }
    assert_eq!((closeouts(&w, &t), exits(&w, &t)), (0, 0), "never sent again on a guess");
    assert!(sw::land::closeout::safe("Saved.\nsafe to Exit") && !sw::land::closeout::safe("Saved. safe to Exit, said I") && !sw::land::closeout::safe("unsafe to Exit") && !sw::land::closeout::safe("safe to Exiting"));
    set_fake(None);
}
