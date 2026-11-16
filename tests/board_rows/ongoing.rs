//! A row no agent of the board's works (said to be at work with `report --state working`): its ○ where an agent's row
//! has a moon, its line 2 that starts with エージェントなし, and, a day after it was added, its place under 運用中 after
//! `working`, out of the Working filter; on the web board the same (shown/ongoing.rs).
use crate::common;
use common::board::{open, web};
use common::world;
use serde_json::json;
use sw::actions::filing::{report, Report};
use sw::json::*;
use sw::shown::ongoing::{section_of, AFTER, NO_AGENT, ONGOING};

const NOW: i64 = 1_800_000_000_000;
fn at_work(id: &str, msg: &str) { report(id, Report { state: "working".into(), msg: msg.into(), ..Default::default() }).unwrap(); }
fn added(id: &str, at: i64) { sw::store::tasks::patch(id, |t| { t.put("created", at); }).unwrap(); }

#[test]
fn only_a_row_no_agent_works_added_over_a_day_ago_stands_under_運用中() {
    let row = |v: serde_json::Value| obj(v);
    let old = NOW - AFTER - 1;
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "progress", "created": old })), NOW), ONGOING);
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "progress", "created": NOW - AFTER })), NOW), "working", "a day to the minute: not yet");
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "progress" })), NOW), "working", "no time it was added: where it was");
    // An agent's row stays in working however long it runs, and nothing else moves.
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "working", "agent": "claude", "created": old })), NOW), "working");
    assert_eq!(section_of(&row(json!({ "state": "working", "stage": "progress", "created": old, "archivedAt": NOW })), NOW), "working");
    assert_eq!(section_of(&row(json!({ "state": "blocked", "stage": "asks", "created": old })), NOW), "needs you");
    assert_eq!(section_of(&row(json!({ "state": "done", "stage": "done", "created": old })), NOW), "done");
}

#[test]
fn on_the_board_a_pass_that_runs_on_moves_under_運用中_and_says_no_agent_works_it() {
    let w = world();
    sw::clock::set(Some(NOW));
    let round = w.add("Read the shared inbox each morning");
    let ledger = w.add("Note the new rota");
    w.add("Not begun");
    at_work(&round, "Morning pass: 4 mails, 1 answered");
    at_work(&ledger, "");
    added(&round, NOW - 2 * AFTER);
    let mut s = open(&w.root, 40);
    // The list alone, without the task panel beside it.
    let list = |s: &mut common::board::Board| s.screen(120).iter().map(|l| l.split('│').next().unwrap_or("").to_string()).collect::<Vec<_>>().join("\n");
    let text = list(&mut s);
    let at = |p: &str| text.lines().position(|l| l.contains(p)).unwrap_or_else(|| panic!("{p}: {text}"));
    // working, then 運用中, then to do; each row under its own heading.
    assert!(at("▾ working") < at("] Note the new rota") && at("] Note the new rota") < at(&format!("▾ {ONGOING}")) && at(&format!("▾ {ONGOING}")) < at("] Read the shared inbox") && at("] Read the shared inbox") < at("▾ to do"), "{text}");
    // No moon: nobody the board runs works them, and line 2 says so first.
    for name in ["] Read the shared inbox", "] Note the new rota"] { assert!(text.lines().nth(at(name)).unwrap().trim_start_matches(['▌', ' ']).starts_with("○ PRJ-"), "{name}: {text}"); }
    assert_eq!(text.lines().nth(at("] Read the shared inbox") + 1).unwrap().trim_matches(['▌', ' ']), "エージェントなし · Morning pass: 4 mails, 1 answered", "{text}");
    assert_eq!(text.lines().nth(at("] Note the new rota") + 1).unwrap().trim_matches(['▌', ' ']), "エージェントなし", "{text}");
    // The Working filter counts and shows working only.
    assert!(text.contains("Working 1 "), "{text}");
    while s.s.show != 3 { s.press(&["tab"]); }
    let text = list(&mut s);
    assert!(text.contains("Note the new rota") && !text.contains("Read the shared inbox"), "{text}");
    // The web board has the same heading, in the brighter grey, and the same line 2.
    let page = web(&w, "board", json!({}));
    let groups = page.at("list").arr("groups");
    let ongoing = groups.iter().find(|g| g.s("name") == ONGOING).unwrap_or_else(|| panic!("{page}"));
    assert_eq!(ongoing.s("color"), "subtext0");
    let row = &ongoing.arr("rows")[0];
    let said: String = row.arr("says").iter().map(|p| p.t("t")).collect();
    assert_eq!((row.s("mark"), said.as_str()), ("○", "エージェントなし · Morning pass: 4 mails, 1 answered"));
}

#[test]
fn the_menu_says_what_the_ring_and_運用中_mean() {
    let w = world();
    w.add("Fix the page");
    let mut s = open(&w.root, 60);
    s.rows = 80; s.press(&["?"]);
    let text = s.screen(160).join("\n");
    assert!(text.contains(&format!("said to be at work, no agent on it ({NO_AGENT}); a day after it was added, under {ONGOING}")), "{text}");
}
