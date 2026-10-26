//! `board.memoryFree`: with less memory available than it (`MemAvailable` of a meminfo file the test writes), a task
//! about to start waits in the queue, never fails, says "waiting for memory" with a plain line, and starts by itself
//! once there is room; a reviewer waits where it is, then starts; what already works is never stopped.
use crate::common;
use common::*;
use serde_json::json;
use std::{cell::RefCell, rc::Rc};
use sw::json::*;

/// This machine has `gb` GB of memory available, as /proc/meminfo would say it.
fn memory_is(gb: f64) {
    let file = std::env::var("HERDR_PROJECTS_MEMINFO").unwrap();
    std::fs::write(file, format!("MemTotal:       8000000 kB\nMemFree:         100000 kB\nMemAvailable:   {} kB\n", (gb * 1e9 / 1024.0) as u64)).unwrap();
}
fn free_in_memory(n: i64) { sw::store::settings::change::set_value("board.memoryFree", &json!(n), &sw::store::settings::lookup::Scope::you()).unwrap(); }

#[test]
fn with_too_little_memory_a_start_waits_in_the_queue_with_a_plain_line_never_fails_and_starts_once_there_is_room() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }] }));
    // One already at work is never stopped when memory runs low.
    let a = w.add("Already at work");
    delegate(&a);
    assert_eq!(settle(&a).s("stage"), "working");
    memory_is(1.8);
    let id = w.add("Big agent");
    sw::clock::advance(1000);
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "queued", "{}", t.at("failure"));
    assert!(t.b("memWait") && t.at("failure").is_null());
    let host = sw::remote::check::hostname();
    assert!(t.s("msg").starts_with(&format!("Memory nearly full on {host}: 1.8 GB free < 3.0 GB (board.memoryFree)")), "{}", t.s("msg"));
    assert_eq!(starts(&w).len(), 1, "no agent was started for it: {:?}", starts(&w));
    assert_eq!(task(&a).s("stage"), "working");
    // Every place says what it waits for, and its place in the queue.
    let r = sw::shown::states::as_row(&row(&id));
    assert_eq!(sw::shown::states::standing(&r), "waiting for memory, Q#1");
    let (_, said) = sw::cli::tasks::command("list", &sw::cli::flags::parse(&["list".to_string()])).unwrap();
    assert!(said.lines().any(|l| l.contains("Big agent") && l.contains("· waiting for memory, Q#1")), "{said}");
    let e = sw::flow::sidebar_entry::entry_of(&row(&id), "", sw::clock::now());
    assert_eq!((e.place.as_str(), e.progress.as_str()), ("waiting for memory", "Q#1"));
    // Still too little: the keeper's pass starts no runner for it, and it still waits, not failed.
    assert!(sw::actions::start::start_queued(&w.socket).is_empty());
    assert_eq!(task(&id).s("stage"), "queued");
    // Room again: it starts by itself.
    memory_is(6.0);
    assert_eq!(sw::actions::start::start_queued(&w.socket), vec![id.clone()]);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working");
    assert!(!t.b("memWait"));
    assert_eq!(starts(&w).len(), 2);
}

#[test]
fn board_memory_free_is_read_from_the_settings_and_0_never_waits() {
    let _w = world();
    // No meminfo to read (not Linux): never waits.
    assert!(sw::memory::available().is_none() && sw::memory_wait::low().is_none());
    assert_eq!(sw::store::settings::lookup::setting("board.memoryFree", None), json!(3));
    memory_is(2.5);
    let l = sw::memory_wait::low().unwrap();
    assert!((2_499_000_000..=2_500_000_000).contains(&l.free) && l.need == 3_000_000_000, "{l:?}");
    // Above the line it starts; under it it waits.
    free_in_memory(2);
    assert!(sw::memory_wait::low().is_none());
    free_in_memory(4);
    assert_eq!(sw::memory_wait::low().map(|l| l.need), Some(4_000_000_000));
    free_in_memory(0);
    memory_is(0.1);
    assert!(sw::memory_wait::low().is_none());
    // `show` names it with what it means.
    let (_, said) = sw::cli::tasks::command("show", &sw::cli::flags::parse(&["show".to_string()])).unwrap();
    assert!(said.contains("board.memoryFree"), "{said}");
}

#[test]
fn a_reviewer_waits_for_memory_where_it_is_then_starts_and_the_review_goes_on() {
    let w = world();
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }], "reviewer": [{ "verdict": { "meets": true, "findings": [], "text": "Fine." } }] }));
    let id = w.add("Reviewed later");
    delegate(&id);
    // Memory runs low just as the agent's turn ends; on the board's fixed clock each of its waits is a look at it.
    sw::clock::set(Some(sw::clock::now()));
    let seen: Rc<RefCell<Vec<(String, String, usize)>>> = Rc::default();
    let reviewers = |w: &World| calls(w).iter().filter(|c| regex::Regex::new(r"^agent start \S+-r ").unwrap().is_match(c)).count();
    { let (seen, fake) = (seen.clone(), w.fake.clone()); let id = id.clone(); sw::clock::meanwhile(move || {
        let t = task(&id);
        let n = fake.st.borrow().arr("calls").iter().filter(|c| regex::Regex::new(r"^agent start \S+-r ").unwrap().is_match(c.as_str().unwrap_or(""))).count();
        seen.borrow_mut().push((t.t("stage"), t.t("msg"), n));
        if seen.borrow().len() >= 2 { memory_is(6.0); }
    }); }
    let low = std::cell::Cell::new(false);
    for _ in 0..12 {
        flush();
        sw::runner::drain_jobs(&|job, jid, extra| { if job == "finish" && !low.get() { memory_is(1.2); low.set(true); } sw::jobs::run_job(job, jid, extra); flush(); });
        if low.get() { break; }
        let t = task(&id);
        if t.at("link").b("pane") { sw::hook::on_pane(&sw::herdr::cli::current_socket(), t.at("link").s("pane")); }
    }
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    let seen = seen.borrow();
    assert!(seen.len() >= 2, "{seen:?}");
    for (stage, msg, n) in seen.iter() {
        assert_ne!(stage, "failed");
        assert!(msg.starts_with("Memory nearly full on ") && msg.contains("1.2 GB free < 3.0 GB"), "{msg}");
        assert_eq!(*n, 0, "no reviewer started while it waited");
    }
    assert_eq!(reviewers(&w), 1);
}

#[test]
fn memory_that_runs_low_during_the_setup_sends_the_start_back_to_the_queue_before_its_agent() {
    let w = world_with(Opts { repo: true, settings: json!({ "workspace.setup": "printf 'MemAvailable: 1000000 kB\\n' > \"$HERDR_PROJECTS_MEMINFO\"" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }] }));
    memory_is(6.0);
    let id = w.add("Heavy setup");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "queued", "{}", t.at("failure"));
    assert!(t.b("memWait") && t.s("msg").contains("1.0 GB free < 3.0 GB"), "{}", t.s("msg"));
    assert!(starts(&w).is_empty(), "no agent was started: {:?}", starts(&w));
    // Room again: it starts in the worktree it has, its setup not run again.
    memory_is(6.0);
    assert_eq!(sw::actions::start::start_queued(&w.socket), vec![id.clone()]);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert_eq!(starts(&w).len(), 1);
    assert!(sw::memory_wait::low().is_none(), "the setup ran once");
}

#[test]
fn an_exited_agent_started_again_shows_its_wait_for_memory_not_that_it_exited() {
    let w = world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Exits");
    delegate(&id);
    let pane = settle(&id).at("link").t("pane");
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"] = serde_json::Value::Null; } } });
    memory_is(1.1);
    sw::clock::set(Some(sw::clock::now()));
    let seen: Rc<RefCell<Vec<(String, String)>>> = Rc::default();
    { let (seen, id) = (seen.clone(), id.clone()); sw::clock::meanwhile(move || { let r = row(&id); seen.borrow_mut().push((r.t("state"), r.t("note"))); memory_is(6.0); }); }
    sw::delegate::jobs::restart_job(&id);
    let seen = seen.borrow();
    assert!(!seen.is_empty(), "it waited");
    for (state, note) in seen.iter() { assert!(state == "working" && note.starts_with("Memory nearly full on ") && note.contains("1.1 GB free < 3.0 GB"), "{seen:?}"); }
    let t = task(&id);
    assert!(!t.b("memHold") && t.s("stage") == "working", "{t:?}");
    assert_eq!(starts(&w).len(), 2, "started again once there was room");
}

#[test]
fn a_reopen_with_too_little_memory_opens_its_space_and_starts_its_agent_by_itself_once_there_is_room() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "copilot" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Reopened");
    delegate(&id);
    let pane = settle(&id).at("link").t("pane");
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"] = serde_json::Value::Null; } } });
    sw::store::tasks::update_wait(&id, 1000, |t| { t.put("stage", "stopped"); t.put("sent", json!({ "text": "Go." })); Ok(sw::store::tasks::Change::Write) }).unwrap();
    assert!(sw::delegate::launch::resumable(&task(&id)), "{:?}", task(&id));
    memory_is(1.1);
    let (_, has, _) = sw::delegate::jobs::reopen(&id, true).unwrap();
    assert!(!has && starts(&w).len() == 1, "not started now");
    let asked: Vec<(String, String, Vec<String>)> = sw::runner::JOBS.with(|j| j.borrow().clone());
    assert!(asked.iter().any(|j| j.0 == "reopen" && j.1 == id), "a runner waits for room: {asked:?}");
    // The runner waits, its row saying so; room again, and its agent is started in the same conversation.
    sw::clock::set(Some(sw::clock::now()));
    let seen: Rc<RefCell<Vec<(String, String)>>> = Rc::default();
    { let (seen, id) = (seen.clone(), id.clone()); sw::clock::meanwhile(move || { let r = row(&id); seen.borrow_mut().push((r.t("state"), r.t("note"))); memory_is(6.0); }); }
    sw::runner::JOBS.with(|j| j.borrow_mut().retain(|j| j.0 != "reopen"));
    sw::delegate::jobs::reopen_job(&id);
    let seen = seen.borrow();
    assert!(!seen.is_empty() && seen.iter().all(|(s, n)| s == "working" && n.contains("1.1 GB free < 3.0 GB")), "{seen:?}");
    assert_eq!(starts(&w).len(), 2, "started again once there was room");
    let t = task(&id);
    assert!(t.s("stage") == "stopped" && !t.b("memHold") && t.at("runner").is_null(), "{t:?}");
}

#[test]
fn an_archived_task_reopened_with_too_little_memory_starts_its_agent_too_once_there_is_room() {
    let w = world_with(Opts { repo: true, settings: json!({ "worker.agent": "copilot" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Reopened");
    delegate(&id);
    let pane = settle(&id).at("link").t("pane");
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"] = serde_json::Value::Null; } } });
    sw::store::tasks::update_wait(&id, 1000, |t| { t.put("stage", "stopped"); t.put("archivedAt", sw::clock::now()); t.put("sent", json!({ "text": "Go." })); Ok(sw::store::tasks::Change::Write) }).unwrap();
    assert!(sw::delegate::launch::resumable(&task(&id)), "{:?}", task(&id));
    memory_is(1.1);
    let (_, has, _) = sw::delegate::jobs::reopen(&id, true).unwrap();
    assert!(!has && starts(&w).len() == 1, "not started now");
    let asked: Vec<(String, String, Vec<String>)> = sw::runner::JOBS.with(|j| j.borrow().clone());
    assert!(asked.iter().any(|j| j.0 == "reopen" && j.1 == id), "a runner waits for room: {asked:?}");
    // The runner waits, its row saying so; room again, and its agent is started in the same conversation.
    sw::clock::set(Some(sw::clock::now()));
    let seen: Rc<RefCell<Vec<(String, String)>>> = Rc::default();
    { let (seen, id) = (seen.clone(), id.clone()); sw::clock::meanwhile(move || { let r = row(&id); seen.borrow_mut().push((r.t("state"), r.t("note"))); memory_is(6.0); }); }
    sw::runner::JOBS.with(|j| j.borrow_mut().retain(|j| j.0 != "reopen"));
    sw::delegate::jobs::reopen_job(&id);
    let seen = seen.borrow();
    assert!(!seen.is_empty() && seen.iter().all(|(s, n)| s == "working" && n.contains("1.1 GB free < 3.0 GB")), "{seen:?}");
    assert_eq!(starts(&w).len(), 2, "started again once there was room");
    let t = task(&id);
    assert!(t.s("stage") == "stopped" && !t.b("memHold") && t.at("runner").is_null(), "{t:?}");
}
