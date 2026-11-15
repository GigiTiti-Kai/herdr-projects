//! A message an agent took in the middle of a long turn: delivered, even found only when it rests, and never retried
//! as "not started" while its session lives; one only queued is not delivered; and an agent that exits with typed text
//! not taken is not started with the text kept, whatever late report of its old turn comes in; a message taken in a
//! new conversation is read from that conversation, even when herdr named it before the start was done (PRJ-25).
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use sw::herdr::cli::{Fake, Ran};
use sw::json::*;
use sw::submit::{NOT_STARTED, TRIES};

fn waits(ready: i64, submit: i64) { unsafe { std::env::set_var("HERDR_PROJECTS_READY_MS", ready.to_string()); std::env::set_var("HERDR_PROJECTS_SUBMIT_MS", submit.to_string()); } }
fn pass(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
fn count(w: &World, re: &str) -> usize { let r = regex::Regex::new(re).unwrap(); calls(w).iter().filter(|c| r.is_match(c)).count() }
fn the_agent(w: &World, id: &str) -> Value { agent_of(w, task(id).at("link").s("pane")) }
fn said_times(w: &World, id: &str, words: &str) -> usize { the_agent(w, id).arr("prompts").iter().filter(|x| text(x).contains(words)).count() }
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn tell(id: &str, words: &str) { sw::actions::talk::tell(id, words, false).unwrap(); }
fn first_agent(w: &World, f: impl FnOnce(&mut Value)) { w.patch(|s| { if let Some(p) = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.at("agent").is_object()) { f(&mut p["agent"]); } }); }
fn rests(w: &World) { w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); if let Some(p) = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.at("agent").is_object()) { p["agent"]["status"] = json!("idle"); p["agent"]["seq"] = json!(seq); p["agent"]["completion"] = json!(seq); } }); }
/// The agent's note, written after its turn was armed (as `flush` writes notes: never in the same moment).
fn note(id: &str, words: &str) {
    let f = sw::flow::task::note_file(&task(id)); std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(&f, words).unwrap();
    let at = std::time::UNIX_EPOCH + std::time::Duration::from_millis((sw::clock::now().max(task(id).i("armedAt")) + 2000) as u64);
    std::fs::File::options().write(true).open(&f).unwrap().set_modified(at).unwrap();
}
/// A task whose agent is at work on its first turn, in a world that reviews nothing.
fn long_turn(title: &str) -> (World, String) {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    waits(60000, 0);
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    let id = w.add(title);
    delegate(&id);
    settle(&id);
    (w, id)
}
/// What Claude Code writes when it takes, in the middle of a turn, a message that was typed while it worked: not a
/// user entry but an attachment (seen on claude 2.1.296, task 175). Then more than a megabyte of the turn's own work.
fn took_mid_turn(w: &World, id: &str, said: &str) -> PathBuf {
    let file = w.base.join(format!("conversation-{}.jsonl", task(id).i("n")));
    let at = sw::clock::iso(sw::clock::now());
    let work = json!({ "type": "user", "message": { "role": "user", "content": [{ "type": "tool_result", "content": "x".repeat(64 * 1024) }] }, "timestamp": at }).to_string();
    let mut lines = vec![json!({ "type": "queue-operation", "operation": "enqueue", "timestamp": at, "content": said }).to_string(), json!({ "type": "queue-operation", "operation": "remove", "timestamp": at, "content": said }).to_string(),
        json!({ "type": "attachment", "attachment": { "type": "queued_command", "prompt": said }, "timestamp": at }).to_string()];
    lines.extend(std::iter::repeat_n(work.clone(), (1200 * 1024usize).div_ceil(work.len())));
    lines.push(json!({ "type": "assistant", "message": { "id": "msg_done", "role": "assistant", "content": [{ "type": "text", "text": "All of it is done." }], "stop_reason": "end_turn" }, "timestamp": at }).to_string());
    std::fs::write(&file, lines.join("\n") + "\n").unwrap();
    sw::store::tasks::patch(id, |t| { t.put("transcript", file.display().to_string()); }).unwrap();
    first_agent(w, |a| a["box"] = Value::Null); // the program took it out of its box
    file
}

#[test]
fn a_message_taken_in_the_middle_of_a_long_turn_is_delivered_and_no_enter_is_pressed() {
    let (w, id) = long_turn("Long turn");
    w.patch(|s| s["swallow"] = json!(1)); // herdr's counters do not move for a message a busy program queues
    tell(&id, "ONE MORE THING while you are at it");
    pass(&w);
    assert!(task(&id).at("sent").b("busy"));
    took_mid_turn(&w, &id, &task(&id).at("sent").t("text"));
    // Counted as delivered the moment the agent takes it, while it still works.
    for _ in 0..2 { pass(&w); }
    assert_eq!((task(&id).t("stage"), task(&id).at("sent").is_null()), ("working".into(), true));
    // Its turn ends, far later in its conversation than the board reads for an ending: that turn is the answer.
    note(&id, "All of it is done.");
    rests(&w);
    for _ in 0..(TRIES + 3) { pass(&w); settle(&id); }
    let c = task(&id);
    assert_ne!(row(&id).s("note"), NOT_STARTED);
    assert!(!c.b("unsent") && c.at("failure").s("stage") != "send", "{}", c.at("failure"));
    assert_eq!((count(&w, r"^agent send-keys .* enter$"), count(&w, r"^agent start")), (0, 1));
    assert_eq!(c.s("stage"), "done", "{}", c.s("msg"));
}

#[test]
fn a_message_taken_mid_turn_and_found_only_when_the_agent_rests_is_delivered_one_only_queued_is_not() {
    let (w, id) = long_turn("Long turn, board closed");
    w.patch(|s| s["swallow"] = json!(1));
    tell(&id, "SAID WITH NOBODY LOOKING");
    let said = task(&id).at("sent").t("text");
    let mark = task(&id).at("sent").clone();
    let cwd = sw::store::repos::cwd_of(&task(&id));
    // Only queued so far: the program has the words, but has not read them. That is not delivered.
    let file = w.base.join("queued.jsonl");
    std::fs::write(&file, json!({ "type": "queue-operation", "operation": "enqueue", "timestamp": sw::clock::iso(sw::clock::now()), "content": said }).to_string() + "\n").unwrap();
    assert_eq!(sw::submit::shown("claude", Some(&file), "", &cwd, &mark, &said), Some(false));
    let conv = took_mid_turn(&w, &id, &said);
    assert_eq!(sw::submit::shown("claude", Some(Path::new(&conv)), "", &cwd, &mark, &said), Some(true), "more than a megabyte back from the end, past what one look at an ending reads");
    note(&id, "Done, with the addition.");
    rests(&w);
    for _ in 0..(TRIES + 3) { pass(&w); settle(&id); }
    assert_eq!(task(&id).s("stage"), "done", "{}", task(&id).s("msg"));
    assert_eq!(count(&w, r"^agent send-keys .* enter$"), 0);
}

#[test]
fn retry_on_not_started_never_restarts_a_session_that_is_alive_and_took_the_text() {
    let (w, id) = long_turn("Wrongly not started");
    // As an older board left it: the text was taken, the turn's work is done and saved, and the task says not started.
    w.patch(|s| s["swallow"] = json!(1));
    tell(&id, "TAKEN ALL ALONG");
    let said = task(&id).at("sent").t("text");
    took_mid_turn(&w, &id, &said);
    note(&id, "Done long ago.");
    rests(&w);
    // The record is an older board's: it holds the moment the agent came to rest, a minute after it took the words.
    let later = sw::clock::now() + 60000;
    let patch = sw::store::failures::failure_patch(&obj(json!({ "stage": "send", "title": "The agent has not started on its instructions", "detail": "", "next": "" })));
    sw::store::tasks::patch(&id, |c| { c.put("unsent", json!({ "text": said, "first": false, "at": later })); c.put("sent", Value::Null); c.put("stage", "failed"); c.put("msg", NOT_STARTED); for (k, v) in patch { c.insert(k, v); } }).unwrap();
    let pid = the_agent(&w, &id).i("pid");
    let quits = count(&w, r"^pane quit");
    let out = sw::actions::finish_line::retry(&id).unwrap().msg;
    assert!(out.contains("its agent had taken the text"), "{out}");
    let t = task(&id);
    assert_eq!((t.t("stage"), t.at("unsent").is_null(), t.at("failure").is_null()), ("working".into(), true, true));
    assert!(sw::runner::JOBS.with(|j| j.borrow().is_empty()), "no restart was asked for");
    assert_eq!((the_agent(&w, &id).i("pid"), count(&w, r"^pane quit")), (pid, quits));
    settle(&id);
    assert_eq!(count(&w, r"^agent start"), 1, "the one program, start to end");
    assert_eq!(task(&id).s("stage"), "done", "{}", task(&id).s("msg"));
    // What this board writes down is when the text was typed: only words standing since then count, and words the
    // program never took still get it started again.
    let t = task(&id);
    let with = |unsent: Value, started: Option<i64>| { let mut x = obj(json!({ "agent": "claude", "transcript": t.at("transcript"), "unsent": unsent })); if let Some(s) = started { x.put("startedAgentAt", s); } sw::submit::took_after_all(&x) };
    assert_eq!(with(json!({ "text": said, "at": sw::clock::now() + 60000, "typed": true }), None), Some(false));
    assert_eq!(with(json!({ "text": said, "at": sw::clock::now() - 60000, "typed": true }), None), Some(true));
    assert_eq!(with(json!({ "text": "NEVER SAID TO IT AT ALL", "at": sw::clock::now() }), Some(1)), Some(false));
}

#[test]
fn an_agent_that_exits_with_typed_text_not_taken_is_not_started_and_a_late_report_finishes_nothing() {
    let (w, id) = long_turn("Exits");
    // Its own report that a turn ended, as its hook leaves it: arriving late, after everything below.
    let report = || {
        sw::store::tasks::patch(&id, |c| { let n = c.i("eventSeq") + 1; c.put("eventSeq", n); let mut ev = sw::store::marks::pending_events(c); ev.push(json!({ "at": sw::clock::now(), "role": "agent", "round": 0, "session": c.t("session"), "file": "", "error": "", "side": false, "n": n })); c.put("events", ev); }).unwrap();
        sw::runner::spawn_runner("finish", &id, &[]);
        drain();
    };
    let exits = |w: &World| w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { p["agent"] = Value::Null; } });
    // Told not started (every enter refused), then its program exits, then the old turn's report comes in.
    w.patch(|s| { s["swallow"] = json!(1); s["deaf"] = json!(99); });
    tell(&id, "LEFT IN THE BOX");
    rests(&w);
    for _ in 0..(TRIES + 3) { pass(&w); settle(&id); }
    assert_eq!(task(&id).s("stage"), "failed");
    exits(&w);
    report();
    for _ in 0..3 { pass(&w); settle(&id); }
    assert_eq!((task(&id).t("stage"), row(&id).t("note")), ("failed".into(), NOT_STARTED.to_string()));
    assert!(task(&id).at("unsent").s("text").contains("LEFT IN THE BOX"));
    // Retry starts a program again and hands it the text; a message typed into that one, which then exits before
    // taking it, ends the same way without a single enter having been refused.
    w.patch(|s| s["deaf"] = json!(0));
    sw::actions::finish_line::retry(&id).unwrap();
    settle(&id);
    assert_eq!(said_times(&w, &id, "LEFT IN THE BOX"), 1);
    w.patch(|s| s["swallow"] = json!(1));
    tell(&id, "GONE WITH IT");
    exits(&w);
    report();
    for _ in 0..3 { pass(&w); settle(&id); }
    assert_eq!((task(&id).t("stage"), row(&id).t("note")), ("failed".into(), NOT_STARTED.to_string()));
    assert!(task(&id).at("unsent").s("text").contains("GONE WITH IT"));
    assert!(task(&id).at("failure").s("detail").contains("program ended before it started on it"), "{}", task(&id).at("failure"));
    sw::actions::finish_line::retry(&id).unwrap();
    settle(&id);
    assert_eq!(said_times(&w, &id, "GONE WITH IT"), 1);
}

/// The world's fake herdr, with a look at every call first (as cleanup_late.rs): `on` acts in the middle of a step.
struct Hooked { inner: Rc<herdr::FakeHerdr>, on: RefCell<Option<Box<dyn Fn(&[String])>>>, busy: Cell<bool> }
impl Fake for Hooked {
    fn run(&self, args: &[String], input: &str) -> Ran {
        if !self.busy.get() { if let Some(f) = self.on.borrow().as_ref() { self.busy.set(true); f(args); self.busy.set(false); } }
        self.inner.run(args, input)
    }
    fn kids(&self, pid: i64) -> Vec<i64> { self.inner.kids(pid) }
    fn wired_to(&self, pid: i64, group: &[i64]) -> Option<bool> { self.inner.wired_to(pid, group) }
    fn kill(&self, pid: i64) { self.inner.kill(pid) }
}

#[test]
fn a_message_taken_mid_turn_in_a_new_conversation_is_read_there_not_in_the_one_before() {
    // As effortless/5 on 2026-11-15: its first conversation's file is on record; its program exits, and a reply starts
    // a new conversation, which herdr names (the hook takes that name) while the board still waits for its screen.
    let (w, id) = long_turn("New conversation");
    let before = w.base.join("conversation-before.jsonl");
    std::fs::write(&before, json!({ "type": "user", "message": { "role": "user", "content": "the first brief" }, "timestamp": sw::clock::iso(sw::clock::now()) }).to_string() + "\n").unwrap();
    sw::store::tasks::patch(&id, |t| { t.put("transcript", before.display().to_string()); }).unwrap();
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { p["agent"] = Value::Null; } });
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    let h = Rc::new(Hooked { inner: w.fake.clone(), on: RefCell::new(None), busy: Cell::new(false) });
    sw::herdr::cli::set_fake(Some(h.clone()));
    let started = Rc::new(Cell::new(0));
    { let (w2, started, socket) = (w.fake.clone(), started.clone(), w.socket.clone()); *h.on.borrow_mut() = Some(Box::new(move |args| {
        if args.len() > 1 && args[0] == "agent" && args[1] == "start" { started.set(1); return; }
        if started.get() != 1 { return; }
        let mut pane = String::new();
        { let s = &mut *w2.st.borrow_mut(); for p in s["panes"].as_array_mut().unwrap() { let a = &mut p["agent"]; if !a.is_object() { continue; } let args: Vec<String> = a.arr("args").iter().map(text).collect(); if let Some(i) = args.iter().position(|x| x == "--session-id") { a["session"] = json!(args[i + 1]); pane = p.t("pane_id"); } } }
        if !pane.is_empty() { started.set(2); sw::hook::on_pane(&socket, &pane); }
    })); }
    tell(&id, "START AGAIN WITH THIS");
    drain();
    settle(&id);
    *h.on.borrow_mut() = None;
    sw::herdr::cli::set_fake(Some(w.fake.clone()));
    assert_eq!(started.get(), 2, "herdr named the new conversation during the start");
    let t = task(&id);
    assert_ne!(t.s("transcript"), before.display().to_string(), "the conversation before is not this one's");
    // A message typed while it works, taken in the middle of its turn: in the new conversation's own file.
    w.patch(|s| s["swallow"] = json!(1));
    tell(&id, "TAKEN IN THE NEW ONE");
    assert!(task(&id).at("sent").b("busy"));
    let said = task(&id).at("sent").t("text");
    let file = sw::agents::adapter::transcript("claude", t.s("session"), &sw::store::repos::cwd_of(&t)).unwrap_or_else(|| {
        let dir = Path::new(&std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join("projects").join(sw::store::repos::cwd_of(&t).canonicalize().unwrap().display().to_string().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>());
        std::fs::create_dir_all(&dir).unwrap(); dir.join(format!("{}.jsonl", t.s("session")))
    });
    let at = sw::clock::iso(sw::clock::now());
    let lines = [json!({ "type": "attachment", "attachment": { "type": "queued_command", "commandMode": "prompt", "origin": { "kind": "human" }, "prompt": said }, "timestamp": at }),
        json!({ "type": "assistant", "message": { "id": "msg_new", "role": "assistant", "content": [{ "type": "text", "text": "Done, with it." }], "stop_reason": "end_turn" }, "timestamp": at })];
    use std::io::Write; std::fs::OpenOptions::new().create(true).append(true).open(&file).unwrap().write_all(lines.iter().map(|l| l.to_string() + "\n").collect::<String>().as_bytes()).unwrap();
    first_agent(&w, |a| a["box"] = Value::Null);
    note(&id, "Done, with it.");
    rests(&w);
    for _ in 0..(TRIES + 3) { pass(&w); settle(&id); }
    let c = task(&id);
    assert_ne!(row(&id).s("note"), NOT_STARTED, "{}", c.at("failure"));
    assert!(!c.b("unsent") && c.at("failure").s("stage") != "send", "{}", c.at("failure"));
}
