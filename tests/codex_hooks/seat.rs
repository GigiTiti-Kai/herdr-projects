//! A seat's Codex stopped on "Hooks need review" at its start: the board presses "Trust all and continue" when every
//! hook passes and the work goes on, or presses nothing and leaves it for you, the reason on the row and in the log.
use super::{clean, listed, plugin, screen, FROM, PLUGIN};
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::json::*;

fn log() -> String { read(&sw::store::files::data_dir().join("hooks-trust.log")) }
/// The world with a Codex seat whose start shows the hooks dialog, and Codex's list for it answered.
fn seated(role: &str, allow: &str, listed_as: &str) -> (World, String) {
    let key = if role == "worker" { "worker.agent" } else { "reviewer.agent" };
    let w = world_with(Opts { repo: true, settings: json!({ key: "codex", "reviewer.fallback": "none", "review.when": if role == "worker" { "never" } else { "always" }, "review.rounds": 0, "review.timeout": "1s", "board.trustHookPlugins": allow }), ..Default::default() });
    let root = plugin(&w.base, &clean());
    let v = listed(&root, listed_as);
    sw::agents::codex_hooks::list::set_fake(Some(std::rc::Rc::new(move |_: &Path| Ok(v.clone()))));
    w.patch(|s| { s["dialogs"]["codex"] = json!(1); s["dialogScreen"]["codex"] = json!(screen(1)); });
    let pass = json!({ "verdict": { "meets": true, "findings": [], "text": "Fine." } });
    w.script(json!({ "worker": [step(1)], "reviewer": [{ "status": "working", "then": pass }] }));
    let id = w.add("Behind the hooks dialog");
    (w, id)
}
fn typed(w: &World) -> Vec<String> { w.state().arr("typed").iter().map(|t| t.t("text")).collect() }

#[test]
fn a_workers_dialog_of_hooks_you_allowed_is_answered_and_its_brief_goes_in() {
    let (w, id) = seated("worker", &format!("{PLUGIN}={FROM}"), "modified");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(typed(&w), vec!["2"], "only the number of \"Trust all and continue\" is typed");
    let pane = t.at("link").t("pane");
    assert_eq!(agent_of(&w, &pane).arr("keys").iter().map(text).collect::<Vec<_>>(), vec!["enter"]);
    assert_eq!(prompts(&w, &pane).len(), 1);
    let l = log();
    assert!(l.contains(&format!("{id} worker pane {pane}")) && l.contains(&format!("trusted 1: sessionStart {PLUGIN}:hooks/hooks.json:session_start:0:0: plugin {PLUGIN} from {FROM}")), "{l}");
}

#[test]
fn a_workers_dialog_of_hooks_you_did_not_allow_gets_no_key_and_waits_for_you() {
    let (w, id) = seated("worker", "none", "modified");
    delegate(&id);
    let t = settle(&id);
    assert!(typed(&w).is_empty(), "{:?}", typed(&w));
    let pane = t.at("link").t("pane");
    assert!(agent_of(&w, &pane).arr("keys").is_empty());
    assert_eq!(row(&id).s("state"), "blocked");
    assert!(log().contains("not trusted: sessionStart hook pt@pt:hooks/hooks.json:session_start:0:0"), "{}", log());
}

#[test]
fn a_reviewers_dialog_of_hooks_you_allowed_is_answered_and_the_review_goes_on() {
    let (w, id) = seated("reviewer", &format!("{PLUGIN}={FROM}"), "untrusted");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(typed(&w), vec!["2"]);
    assert!(log().contains(&format!("{id} reviewer pane")) && log().contains("trusted 1"), "{}", log());
}

#[test]
fn a_reviewers_dialog_whose_count_does_not_match_gets_no_key_and_says_why() {
    let (w, id) = seated("reviewer", &format!("{PLUGIN}={FROM}"), "untrusted");
    w.patch(|s| s["dialogScreen"]["codex"] = json!(screen(2)));
    delegate(&id);
    let t = settle(&id);
    assert!(typed(&w).is_empty(), "{:?}", typed(&w));
    assert_eq!(t.s("stage"), "failed", "the review timed out with nobody answering: {}", t.at("failure"));
    let l = log();
    assert!(l.contains("not trusted: the screen counts 2 new or changed hooks, Codex's own list 1 (sessionStart)"), "{l}");
}
