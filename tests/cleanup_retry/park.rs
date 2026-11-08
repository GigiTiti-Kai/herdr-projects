//! Parking a worker that waits for you (`workspace.park`, land/park.rs), in fixtures only: off, `park` is refused and
//! nothing is typed; on, its agent is closed out as a done task's is, its stage, approved commit, question and inbox
//! kept, and it comes back only in its own conversation; every caller that would type into it or start it again.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::rc::Rc;
use sw::json::*;
use sw::memory_queue::{set_fake, Job};
use sw::store::tasks::{update, Change};

/// Approve, reply, review, fix, the board's own turns, a conflict, a restart, and a crash or herdr down.
mod callers;
/// The reviewer's probes.
mod reviewer;

pub const SAFE: &str = "Saved: accepted.\nsafe to Exit";
pub fn world() -> World { let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() }); sw::clock::set(Some(sw::clock::now())); w }
pub fn on() { sw::store::settings::change::set_value("workspace.park", &json!(true), &sw::store::settings::lookup::Scope::you()).unwrap(); }
pub fn pane(w: &World, t: &Obj, f: impl FnOnce(&mut Value)) { let id = t.at("link").t("pane"); w.patch(|s| { let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == id).unwrap(); f(p); }); }
pub fn said(w: &World, t: &Obj, what: &str) -> usize { prompts(w, t.at("link").s("pane")).iter().filter(|p| p.contains(what)).count() }
pub fn closeouts(w: &World, t: &Obj) -> usize { said(w, t, "session-close") }
pub fn exits(w: &World, t: &Obj) -> usize { prompts(w, t.at("link").s("pane")).iter().filter(|p| p.as_str() == "/exit").count() }
pub fn accepted_now(t: &Obj) { let (cwd, at) = (t.s("worktree").to_string(), sw::clock::now() + 1); set_fake(Some(Rc::new(move |_| Ok(vec![Job { id: "j1".into(), status: "queued".into(), accepted: at, cwd: cwd.clone() }])))); }
/// A task waiting for its approval (or, `asks`, for your answer), its agent's process and session as herdr shows them,
/// its input box empty; its next turn (the closeout's) says `reply`.
pub fn waiting(w: &World, n: i64, asks: bool, reply: &str) -> Obj {
    w.script(json!({ "worker": [{ "files": { format!("p{n}.txt"): "x\n" }, "commit": format!("p{n}"), "note": "Did it.", "said": "Did it." }, { "said": reply }] }));
    let id = w.add(&format!("Task {n}"));
    delegate(&id);
    let mut t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    if asks { sw::actions::filing::report(&id, sw::actions::filing::Report { state: "blocked".into(), kind: "question".into(), msg: "Should the old file go".into(), own: "agent", ..Default::default() }).unwrap(); t = settle(&id); assert_eq!(t.s("stage"), "asks"); }
    pane(w, &t, |p| { p["frontPid"] = json!(4000 + n); p["agent"]["session"] = json!(t.s("session")); p["agent"]["screen"] = json!("Done.\n╭──\n│ ❯ \n╰──"); p["screen"] = json!("user@host:/work$ claude --session-id x\nBye.\nuser@host:/work$ "); });
    task(&id)
}
/// One housekeeping pass, and the park looks it started.
pub fn pass(w: &World, id: &str) -> usize {
    let _ = std::fs::remove_dir_all(sw::store::files::data_dir().join("stamps"));
    sw::runner::JOBS.with(|j| j.borrow_mut().clear());
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let jobs: Vec<(String, String, Vec<String>)> = sw::runner::JOBS.with(|j| std::mem::take(&mut *j.borrow_mut())).into_iter().filter(|(job, i, _)| job == "park" && i == id).collect();
    for _ in &jobs { sw::land::park::drive(id); }
    jobs.len()
}
/// Parked, step by step: the closeout, its save, `/exit`, its agent gone.
pub fn parked(w: &World, n: i64, asks: bool) -> Obj {
    let t = waiting(w, n, asks, SAFE);
    sw::land::park::park(t.s("id")).unwrap();
    accepted_now(&task(t.s("id")));
    pass(w, t.s("id"));
    assert_eq!(task(t.s("id")).at("closeout").s("phase"), "closing", "{}", task(t.s("id")).s("msg"));
    pane(w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; });
    pass(w, t.s("id"));
    let c = task(t.s("id"));
    assert!(sw::land::park::parked(&c), "{}", c.s("msg"));
    c
}
/// What a park must leave as it was.
pub fn kept(before: &Obj, after: &Obj) {
    for k in ["stage", "reviewedHead", "requireApproval", "mergeIntent", "kind", "reviewRounds", "session", "armedAt"] { assert_eq!(before.at(k), after.at(k), "{k}"); }
    assert_eq!(before.at("review").at("stale"), after.at("review").at("stale"));
}

#[test]
fn off_by_default_park_is_refused_and_nothing_is_typed() {
    let w = world();
    let t = waiting(&w, 1, false, SAFE);
    let e = sw::land::park::park(t.s("id")).unwrap_err();
    assert!(e.msg.contains("Parking is off"), "{}", e.msg);
    assert_eq!(closeouts(&w, &t), 0);
    assert!(!task(t.s("id")).at("closeout").is_object());
}

#[test]
fn an_approval_is_closed_out_and_parked_with_its_stage_commit_and_review_kept() {
    let w = world();
    on();
    let t = waiting(&w, 2, false, SAFE);
    let id = t.t("id");
    sw::land::park::park(&id).unwrap();
    let c = task(&id);
    // typed once, as the board's own turn; the reply's turn took nothing from the approval
    assert_eq!(closeouts(&w, &t), 1);
    assert_eq!(c.at("closeout").s("phase"), "checkpointing");
    kept(&t, &c);
    on_pane(&w, t.at("link").s("pane"));
    settle(&id);
    kept(&t, &task(&id));
    accepted_now(&c);
    pass(&w, &id);
    assert_eq!((task(&id).at("closeout").t("phase"), exits(&w, &t)), ("closing".to_string(), 1));
    pane(&w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; });
    pass(&w, &id);
    let c = task(&id);
    assert!(sw::land::park::parked(&c) && c.s("msg").contains("Parked"), "{}", c.s("msg"));
    kept(&t, &c);
    assert_eq!(row(&id).s("state"), "blocked", "still waits for you, not an exited agent");
    // passes after: nothing typed, nothing started, no job of its own
    let before = calls(&w).len();
    for _ in 0..3 { assert_eq!(pass(&w, &id), 0); }
    assert!(calls(&w)[before..].iter().all(|c| !c.starts_with("agent start") && !c.starts_with("agent prompt")), "{:?}", &calls(&w)[before..]);
    set_fake(None);
}

#[test]
fn a_question_is_parked_with_its_question_kept_and_comes_back_in_its_own_conversation() {
    let w = world();
    on();
    let t = parked(&w, 3, true);
    let id = t.t("id");
    assert_eq!(t.s("stage"), "asks");
    let was = t.at("closeout").t("was");
    assert!(was.contains("Should the old file go"), "{was}");
    // going into the task: the same conversation, nothing sent to it, the question as it was
    let (_, has, _) = sw::delegate::reopen(&id, true).unwrap();
    assert!(has);
    let c = task(&id);
    assert!(calls(&w).iter().any(|c| c.starts_with("agent start") && c.contains(&format!("--resume {}", t.s("session")))), "{:?}", calls(&w));
    assert_eq!((c.s("stage"), c.s("msg"), c.s("session")), ("asks", was.as_str(), t.s("session")));
    assert!(!c.at("closeout").is_object(), "the park is over");
    set_fake(None);
}

#[test]
fn a_park_asked_twice_or_with_words_waiting_or_not_waiting_for_you_is_refused() {
    let w = world();
    on();
    let t = waiting(&w, 4, false, SAFE);
    let id = t.t("id");
    sw::store::inbox::queue_text(&id, "later", Obj::new()).unwrap();
    assert!(sw::land::park::park(&id).unwrap_err().msg.contains("Words wait"));
    sw::store::inbox::take_text(&id).unwrap();
    sw::land::park::park(&id).unwrap();
    assert!(sw::land::park::park(&id).unwrap_err().msg.contains("closeout record already"));
    assert_eq!(closeouts(&w, &t), 1);
    update(&id, |t| { t.put("stage", "working"); t.put("closeout", Value::Null); Ok(Change::Write) }).unwrap();
    assert!(sw::land::park::park(&id).unwrap_err().msg.contains("waits for nothing of yours"));
    set_fake(None);
}

#[test]
fn a_park_held_before_anything_went_in_is_no_park_and_one_held_later_leaves_the_approval_as_it_was() {
    let w = world();
    on();
    // words wait unsent in its input box: nothing is typed, no record is left, and it can be asked again
    let t = waiting(&w, 5, false, SAFE);
    let id = t.t("id");
    pane(&w, &t, |p| p["agent"]["screen"] = json!("Done.\n╭──\n│ ❯ half a thought\n╰──"));
    let e = sw::land::park::park(&id).unwrap_err();
    assert!(e.msg.contains("not parked, and nothing was typed") && e.msg.contains("words wait unsent"), "{}", e.msg);
    assert!(!task(&id).at("closeout").is_object() && closeouts(&w, &t) == 0);
    assert_eq!(task(&id).s("msg"), t.s("msg"));
    pane(&w, &t, |p| p["agent"]["screen"] = json!("Done.\n╭──\n│ ❯ \n╰──"));
    sw::land::park::park(&id).unwrap();
    // no save accepted within the wait: held with its agent still there, and the hook's next look finds no turn of yours
    set_fake(Some(Rc::new(|_| Ok(vec![]))));
    on_pane(&w, t.at("link").s("pane"));
    settle(&id);
    sw::clock::advance(11 * 60 * 1000);
    pass(&w, &id);
    let c = task(&id);
    assert!(c.at("closeout").b("held") && c.s("msg").contains("closeout 未確認"), "{}", c.s("msg"));
    on_pane(&w, t.at("link").s("pane"));
    settle(&id);
    kept(&t, &task(&id));
    assert_eq!(exits(&w, &t), 0);
    set_fake(None);
}

#[test]
fn resumed_its_next_looks_keep_what_it_waits_on_and_its_clean_up_is_a_closeout_of_its_own() {
    let w = world();
    on();
    sw::store::settings::change::set_value("workspace.closeout", &json!(true), &sw::store::settings::lookup::Scope::you()).unwrap();
    let t = parked(&w, 6, false);
    let id = t.t("id");
    w.script(json!({ "worker": [{ "said": SAFE }] }));
    let (_, has, _) = sw::delegate::reopen(&id, true).unwrap();
    assert!(has);
    let p = t.at("link").t("pane");
    pane(&w, &t, |p| { p["frontPid"] = json!(5006); p["agent"]["session"] = json!(t.s("session")); p["agent"]["screen"] = json!("Done.\n╭──\n│ ❯ \n╰──"); });
    on_pane(&w, &p);
    settle(&id);
    let c = task(&id);
    kept(&obj(json!({ "stage": t.at("stage"), "reviewedHead": t.at("reviewedHead"), "requireApproval": t.at("requireApproval"), "mergeIntent": t.at("mergeIntent"), "kind": t.at("kind"), "reviewRounds": t.at("reviewRounds"), "session": t.at("session"), "armedAt": c.at("armedAt"), "review": t.at("review") })), &c);
    assert_eq!(c.s("msg"), t.at("closeout").s("was"));
    // approved and merged: the clean-up's closeout is a new one, typed once, into the agent that came back
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let c = settle(&id);
    assert_eq!(c.s("stage"), "done", "{}", c.at("failure"));
    assert_eq!(closeouts(&w, &c), 1, "{:?}", prompts(&w, &p));
    assert!(!sw::land::park::unfinished(&c));
    set_fake(None);
}
