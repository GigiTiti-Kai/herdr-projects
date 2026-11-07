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

#[test]
fn on_the_closeout_goes_in_once_and_the_space_closes_only_once_its_record_was_accepted() {
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
    assert!(c.s("msg").contains("Closeout sent"), "{}", c.s("msg"));
    assert!(open(&w, &c) && Path::new(c.s("worktree")).exists() && c.b("tidy"));
    assert_eq!(c.i("reviewRounds"), t.i("reviewRounds")); assert_eq!(c.at("mergedAt"), t.at("mergedAt"));
    assert_ne!(c.at("last").at("comp"), c.at("closeout").at("comp0"), "the closeout turn's end was seen by the board, and changed nothing");
    // no job yet, a job from another folder, one from before it was sent: not sent again, nothing closed
    assert_eq!(pass(&w, &id), 1);
    *jobs.borrow_mut() = Ok(vec![job("else", "/elsewhere", sw::clock::now()), job("old", c.s("worktree"), c.at("closeout").i("sentAt") - 1)]);
    assert_eq!(pass(&w, &id), 1);
    assert_eq!(closeouts(&w, &t), 1, "never typed twice for the same session and tip");
    assert!(open(&w, &task(&id)) && task(&id).s("stage") == "done");
    // accepted: closed, its line as it was
    *jobs.borrow_mut() = Ok(vec![job("j1", c.s("worktree"), sw::clock::now())]);
    assert_eq!(pass(&w, &id), 1);
    let c = task(&id);
    assert!(c.b("cleaned") && !open(&w, &c) && !Path::new(t.s("worktree")).exists(), "{}", c.s("msg"));
    assert_eq!(c.at("closeout").s("phase"), "saved"); assert_eq!(c.at("closeout").s("job"), "j1");
    assert_eq!(c.s("msg"), "Merged into main.");
    set_fake(None);
}

#[test]
fn what_cannot_be_proved_keeps_the_space_with_the_reason_and_types_nothing_more() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    on();
    queue(Rc::new(std::cell::RefCell::new(Err("no Agent Memory queue".into()))));
    // the queue cannot be read: waits, then after closeoutWait is held
    let t = merged(&w, 3);
    let id = t.t("id");
    assert_eq!(pass(&w, &id), 1);
    sw::clock::advance(11 * 60 * 1000);
    assert_eq!(pass(&w, &id), 1);
    let c = task(&id);
    assert!(c.s("msg").contains("closeout 未確認: no Agent Memory queue"), "{}", c.s("msg"));
    assert!(!c.b("tidy") && c.arr("kept").iter().any(|k| text(k).contains("closeout 未確認")) && open(&w, &c) && Path::new(c.s("worktree")).exists());
    assert_eq!(pass(&w, &id), 0, "held: not tried again by itself");
    assert_eq!(closeouts(&w, &t), 1);
    // a dialog in front of the agent: nothing is typed at all
    w.script(json!({ "worker": [{ "files": { "d.txt": "x\n" }, "commit": "d", "note": "Did it." }] }));
    let d = w.add("Dialog");
    delegate(&d);
    assert_eq!(settle(&d).s("stage"), "approval");
    update(&d, |t| { t.put("session", "s-d"); Ok(Change::Write) }).unwrap();
    let p = task(&d).at("link").t("pane");
    w.patch(|s| { let x = s["panes"].as_array_mut().unwrap().iter_mut().find(|x| x.s("pane_id") == p).unwrap(); x["agent"]["status"] = json!("blocked"); });
    let before = prompts(&w, &p).len();
    sw::actions::finish_line::approve(&d, None, false).unwrap();
    let c = settle(&d);
    assert_eq!(c.s("stage"), "done");
    assert_eq!(prompts(&w, &p).len(), before);
    assert!(c.s("msg").contains("closeout 未確認: its agent is blocked"), "{}", c.s("msg"));
    assert!(open(&w, &c) && !c.b("tidy"));
    // a turn of yours after the closeout's own, before its record came: what is accepted proves nothing, it goes in again
    let y = merged(&w, 5);
    let yid = y.t("id");
    assert_eq!(pass(&w, &yid), 1);
    assert!(task(&yid).at("closeout").has("comp1"));
    sw::herdr::Herdr::new(None).agent_prompt(y.at("link").s("pane"), "one more thing").unwrap();
    settle(&yid);
    queue(Rc::new(std::cell::RefCell::new(Ok(vec![job("y", y.s("worktree"), sw::clock::now())]))));
    assert_eq!(pass(&w, &yid), 1);
    assert_eq!(closeouts(&w, &y), 2);
    assert!(open(&w, &task(&yid)) && task(&yid).s("stage") == "done");
    queue(Rc::new(std::cell::RefCell::new(Ok(vec![job("y2", y.s("worktree"), sw::clock::now())]))));
    assert_eq!(pass(&w, &yid), 1);
    assert!(task(&yid).b("cleaned"));
    assert_eq!(task(&yid).s("msg"), "Merged into main.", "sent twice, its line as it was all the same");
    queue(Rc::new(std::cell::RefCell::new(Err("no Agent Memory queue".into()))));
    // the session changed since it was sent: that record proves nothing, and the closeout goes in again
    let n = merged(&w, 4);
    let nid = n.t("id");
    update(&nid, |t| { t.put("session", "s-other"); Ok(Change::Write) }).unwrap();
    assert_eq!(pass(&w, &nid), 1);
    assert_eq!(closeouts(&w, &n), 2);
    assert_eq!(task(&nid).at("closeout").s("session"), "s-other");
    set_fake(None);
}
