//! herdr's folded sidebar rows: a task's pane carries its number under the token of the board's section
//! (`task_needs_you`, `task_ready`, `task`) and its title (`task_title`), cut like the name; the clear takes off all four; a pane with no room for them (herdr keeps 32) still gets the rest.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::flow::sidebar_entry::{coordinator_entry, entry_of, reviewer_entry, task_name, task_tokens, Entry, CAP, TASK_TOKENS};
use sw::json::*;
use sw::store::settings::lookup::Scope;

fn with(base: &Obj, more: Value) -> Obj { let mut o = base.clone(); for (k, v) in more.as_object().unwrap() { o.insert(k.clone(), v.clone()); } o }
fn entry(t: &Obj) -> Entry { entry_of(t, "", sw::clock::now()) }
fn base(w: &World) -> Obj {
    let id = w.add("Show automations in the main list");
    sw::store::settings::change::set_value("repo.key", &json!("PRJ"), &Scope::repo("proj")).unwrap();
    with(&task(&id), json!({ "agent": "claude", "app": "claude", "link": { "pane": "w1:p1", "socket": "x" } }))
}
fn approval(pass: bool) -> Value {
    let review = json!({ "round": 1, "gate": "major", "meets": true, "pass": pass, "findings": if pass { json!([]) } else { json!([{ "id": "F1", "severity": "critical", "where": "a.js", "what": "Broken" }]) } });
    json!({ "stage": "approval", "state": "blocked", "kind": "permission", "reviewRound": 1, "review": review, "reviews": [review] })
}

#[test]
fn task_tokens_put_the_number_under_the_boards_section_and_clear_the_other_two() {
    let w = world();
    let b = base(&w);
    let ready = with(&b, approval(true));
    let failed = with(&b, approval(false));
    let asks = with(&b, json!({ "stage": "asks", "state": "blocked", "kind": "question" }));
    let working = with(&b, json!({ "stage": "working", "state": "working" }));
    let done = with(&b, json!({ "stage": "done", "state": "done" }));
    assert_eq!(task_tokens(&entry(&ready)), [("task_needs_you", "".into()), ("task_ready", "PRJ-1".into()), ("task", "".into()), ("task_title", "Show automations in the main list".into())]);
    assert_eq!(task_tokens(&entry(&failed)).map(|(_, v)| v)[..3], ["PRJ-1".to_string(), "".into(), "".into()]);
    assert_eq!(task_tokens(&entry(&asks)).map(|(_, v)| v)[..3], ["PRJ-1".to_string(), "".into(), "".into()]);
    assert_eq!(task_tokens(&entry(&working)).map(|(_, v)| v)[..3], ["".to_string(), "".into(), "PRJ-1".into()]);
    assert_eq!(task_tokens(&entry(&done)).map(|(_, v)| v)[..3], ["".to_string(), "".into(), "PRJ-1".into()]);
    // The reviewer's pane carries the same task; the Coordinator's carries none.
    assert_eq!(task_tokens(&reviewer_entry(&failed, sw::clock::now())), task_tokens(&entry(&failed)));
    assert!(task_tokens(&coordinator_entry("idle", 0, "")).iter().all(|(_, v)| v.is_empty()));
}

#[test]
fn the_section_is_part_of_what_is_written() {
    // sidebar.rs writes again only when json!(entry) changes.
    let w = world();
    let b = base(&w);
    assert_eq!(json!(entry(&with(&b, approval(true))))["section"], "ready to approve");
    assert_eq!(json!(entry(&with(&b, json!({ "stage": "asks", "state": "blocked", "kind": "question" }))))["section"], "needs you");
}

#[test]
fn task_tokens_cover_every_name_the_clear_removes() {
    assert_eq!(task_tokens(&Entry::default()).map(|(k, _)| k), TASK_TOKENS);
}

#[test]
fn task_title_is_cut_like_the_name_and_the_number_stays_whole() {
    let w = world();
    let long = with(&base(&w), json!({ "title": format!("{}\n\x1b[31mred\x1b[0m", "T".repeat(100)), "stage": "working", "state": "working" }));
    let e = entry(&long);
    assert_eq!(e.title.chars().count(), CAP); assert!(e.title.ends_with('…')); assert!(!e.title.contains('\x1b'));
    assert_eq!(e.number, "PRJ-1");
    assert!(task_name(&long).starts_with(&format!("{} ", e.number)));
}

#[test]
fn a_pane_with_no_room_for_the_task_tokens_still_gets_the_rest_of_its_entry() {
    // herdr keeps 32 tokens on a pane, every source's together: other plugins hold all but one here.
    let w = world();
    let t = task(&to_approval(&w, "Add a greeting"));
    let pane = t.at("link").t("pane");
    w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["tokenRoom"] = json!(1); } } });
    sw::flow::sidebar::sync_sidebar(&[with(&t, json!({ "stage": "working", "state": "working", "progress": 40, "live": { "status": "working" } }))], None, sw::clock::now());
    let last = w.state().arr("metadata").iter().filter(|m| m.s("pane") == pane).map(|m| m.arr("args").iter().map(text).collect::<Vec<_>>().join(" ")).last().unwrap();
    assert!(last.contains("--display-agent agent · working") && last.contains("--token progress=40%"), "{last}");
    assert!(last.ends_with("--clear-token task_needs_you --clear-token task_ready --clear-token task --clear-token task_title"), "{last}");
}
