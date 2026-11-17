//! A row no agent of the board's works: said to be at work with `report --state working`, its ○ where an agent's row
//! has a moon; at work or waiting for you, its line 2 that starts with エージェントなし; and, a day after it began to be
//! worked by hand (not after it was added), its place under 運用中 after `working`, out of the Working filter; on the
//! web board the same (shown/ongoing.rs).
use crate::common;
use common::board::{open, web, Board};
use common::world;
use serde_json::json;
use sw::actions::filing::{report, Report};
use sw::json::*;
use sw::shown::ongoing::{section_of, AFTER, NO_AGENT, ONGOING};

const NOW: i64 = 1_800_000_000_000;
fn say(id: &str, state: &str, msg: &str) { report(id, Report { state: state.into(), msg: msg.into(), ..Default::default() }).unwrap(); }
fn at_work(id: &str, msg: &str) { say(id, "working", msg); }
fn task(id: &str) -> Obj { sw::store::tasks::find(id).unwrap() }
/// The list alone, without the task panel beside it.
fn list(s: &mut Board) -> String { s.screen(120).iter().map(|l| l.split('│').next().unwrap_or("").to_string()).collect::<Vec<_>>().join("\n") }
fn line(text: &str, p: &str) -> usize { text.lines().position(|l| l.contains(p)).unwrap_or_else(|| panic!("{p}: {text}")) }
/// Line 2 of the row whose name is `name`.
fn under(text: &str, name: &str) -> String { text.lines().nth(line(text, name) + 1).unwrap().trim_matches(['▌', ' ']).to_string() }

#[test]
fn only_a_row_no_agent_works_at_work_by_hand_over_a_day_stands_under_運用中() {
    let row = |v: serde_json::Value| obj(v);
    let old = NOW - AFTER - 1;
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "progress", "progressFrom": old, "created": old })), NOW), ONGOING);
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "progress", "progressFrom": NOW - AFTER })), NOW), "working", "a day to the minute: not yet");
    // Begun today, however long ago it was added.
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "progress", "progressFrom": NOW - 60_000, "created": NOW - 7 * AFTER })), NOW), "working");
    // One at work since before the board kept when it began: from when it was added; with neither, where it was.
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "progress", "created": old })), NOW), ONGOING);
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "progress" })), NOW), "working");
    // An agent's row stays in working however long it runs, and nothing else moves.
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "working", "agent": "claude", "created": old })), NOW), "working");
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "progress", "progressFrom": old, "archivedAt": NOW })), NOW), "working");
    assert_eq!(section_of(&row(json!({ "state": "blocked", "stage": "asks", "progressFrom": old })), NOW), "needs you");
    assert_eq!(section_of(&row(json!({ "state": "done", "stage": "done", "progressFrom": old })), NOW), "done");
}

#[test]
fn when_it_began_is_kept_across_its_reports_and_waits_and_begins_again_from_to_do() {
    let w = world();
    sw::clock::set(Some(NOW));
    let id = w.add("Keep the rota");
    sw::store::tasks::patch(&id, |t| { t.put("created", NOW - 7 * AFTER); }).unwrap();
    at_work(&id, "");
    assert_eq!(task(&id).i("progressFrom"), NOW, "it began now, a week after it was added");
    sw::clock::set(Some(NOW + 3_600_000)); at_work(&id, "Monday done");
    sw::clock::set(Some(NOW + 7_200_000)); say(&id, "blocked", "Which week next?"); at_work(&id, "Week two");
    assert_eq!(task(&id).i("progressFrom"), NOW, "its reports and a wait for you keep when it began");
    say(&id, "idle", ""); sw::clock::set(Some(NOW + 9_000_000)); at_work(&id, "Again");
    assert_eq!(task(&id).i("progressFrom"), NOW + 9_000_000, "put back in to do and begun again: from then");
}

#[test]
fn on_the_board_a_pass_that_runs_on_moves_under_運用中_a_day_after_it_began_and_says_no_agent_works_it() {
    let w = world();
    sw::clock::set(Some(NOW - 2 * AFTER));
    let round = w.add("Read the shared inbox each morning");
    at_work(&round, "Morning pass: 4 mails, 1 answered");
    sw::clock::set(Some(NOW));
    let ledger = w.add("Note the new rota");
    at_work(&ledger, "");
    // Added a week ago and begun just now: on its way, in working.
    let late = w.add("Fix the hall light");
    sw::store::tasks::patch(&late, |t| { t.put("created", NOW - 7 * AFTER); }).unwrap();
    at_work(&late, "Bought the bulb");
    w.add("Not begun");
    let mut s = open(&w.root, 40);
    let text = list(&mut s);
    let at = |p: &str| line(&text, p);
    // working, then 運用中, then to do; each row under its own heading.
    assert!(at("▾ working") < at("] Note the new rota") && at("] Fix the hall light") < at(&format!("▾ {ONGOING}")) && at(&format!("▾ {ONGOING}")) < at("] Read the shared inbox") && at("] Read the shared inbox") < at("▾ to do"), "{text}");
    // No moon: nobody the board runs works them, and line 2 says so first.
    for name in ["] Read the shared inbox", "] Note the new rota", "] Fix the hall light"] { assert!(text.lines().nth(at(name)).unwrap().trim_start_matches(['▌', ' ']).starts_with("○ PRJ-"), "{name}: {text}"); }
    // What it was told after how long ago it was told (view/measured.rs).
    assert!(under(&text, "] Read the shared inbox").starts_with("エージェントなし · 報告2d前: Morning pass: 4 mails"), "{text}");
    assert_eq!(under(&text, "] Note the new rota"), "エージェントなし", "{text}");
    // The Working filter counts and shows working only.
    assert!(text.contains("Working 2 "), "{text}");
    while s.s.show != 3 { s.press(&["tab"]); }
    let text = list(&mut s);
    assert!(text.contains("Note the new rota") && text.contains("Fix the hall light") && !text.contains("Read the shared inbox"), "{text}");
    // The web board has the same heading, in the brighter grey, and the same line 2.
    let page = web(&w, "board", json!({}));
    let groups = page.at("list").arr("groups");
    let ongoing = groups.iter().find(|g| g.s("name") == ONGOING).unwrap_or_else(|| panic!("{page}"));
    assert_eq!(ongoing.s("color"), "subtext0");
    let row = &ongoing.arr("rows")[0];
    let said: String = row.arr("says").iter().map(|p| p.t("t")).collect();
    assert_eq!((row.s("mark"), said.as_str()), ("○", "エージェントなし · 報告2d前: Morning pass: 4 mails, 1 answered"));
}

#[test]
fn a_row_that_waits_for_you_says_whether_an_agent_is_on_it() {
    let w = world();
    sw::clock::set(Some(NOW));
    let alone = w.add("Look into the noise");
    let with = w.add("Look into the leak");
    sw::store::tasks::patch(&with, |t| { t.put("agent", "claude"); }).unwrap();
    // Its agent's pane linked, and no agent named: one is on it all the same (flow/task.rs: is_agent_task).
    let linked = w.add("Look into the draft");
    sw::store::tasks::patch(&linked, |t| { t.put("link", json!({ "pane": "w1:p9" })); }).unwrap();
    for id in [&alone, &with, &linked] { say(id, "blocked", "Which floor first?"); }
    let mut s = open(&w.root, 40);
    let text = list(&mut s);
    assert_eq!(under(&text, "] Look into the noise"), "エージェントなし · Which floor first?", "{text}");
    assert_eq!(under(&text, "] Look into the leak"), "Which floor first?", "an agent is on it: what it asks, alone");
    assert_eq!(under(&text, "] Look into the draft"), "Which floor first?", "{text}");
    let page = web(&w, "board", json!({}));
    let says = |name: &str| page.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").iter().cloned()).find(|r| r.s("title") == name).map(|r| r.arr("says").iter().map(|p| p.t("t")).collect::<String>()).unwrap_or_else(|| panic!("{name}: {page}"));
    assert_eq!((says("Look into the noise").as_str(), says("Look into the leak").as_str()), ("エージェントなし · Which floor first?", "Which floor first?"));
}

#[test]
fn the_menu_says_what_the_ring_and_運用中_mean() {
    let w = world();
    w.add("Fix the page");
    let mut s = open(&w.root, 60);
    s.rows = 80; s.press(&["?"]);
    let text = s.screen(160).join("\n");
    assert!(text.contains(&format!("said to be at work, no agent on it (any row with none says {NO_AGENT}); a day on, under {ONGOING}")), "{text}");
}
