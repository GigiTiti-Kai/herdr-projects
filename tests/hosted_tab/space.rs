//! Which space a task set to a tab goes to: a space you opened yourself counts when its first tab works in the repo,
//! the repo space herdr opened beside a worktree of the board's never does, and with none open the board opens the
//! repo's space itself.
use super::{repo_name, tab_world, tabs_in, worker};
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;

fn made(w: &World, verb: &str) -> usize { calls(w).iter().filter(|c| c.starts_with(verb)).count() }
/// A space you opened without herdr's worktree commands: no `worktree` in herdr's answer, one tab, one idle pane at `cwd`.
fn plain_space(w: &World, id: &str, label: &str, cwd: &str, focused: bool) {
    w.patch(|s| {
        s["workspaces"].as_array_mut().unwrap().push(json!({ "workspace_id": id, "label": label, "cwd": cwd, "focused": focused, "nextT": 2 }));
        s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": format!("{id}:p1"), "workspace_id": id, "tab_id": format!("{id}:t1"), "terminal_id": format!("term_{id}"), "cwd": cwd, "agent": null }));
        s["tabs"].as_array_mut().unwrap().push(json!({ "tab_id": format!("{id}:t1"), "workspace_id": id, "label": "1" }));
    });
}

#[test]
fn spaces_you_opened_count_when_their_first_tab_works_in_the_repo_and_no_other_folder_does() {
    let w = tab_world();
    let name = repo_name(&w);
    let elsewhere = w.base.join("elsewhere"); std::fs::create_dir_all(&elsewhere).unwrap();
    let inner = w.root.join("inner"); std::fs::create_dir_all(&inner).unwrap(); git(&inner, &["init", "-q"]);
    plain_space(&w, "away", &name, &elsewhere.display().to_string(), true);   // in front and named after the repo, not in it
    plain_space(&w, "nested", &name, &inner.display().to_string(), false);    // a repo of its own inside the repo
    plain_space(&w, "desk", "My desk", &w.root_str(), false);                 // the repo's own folder
    worker(&w, 1);
    let a = w.add("At your desk");
    delegate(&a);
    let t = settle(&a);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.at("link").s("workspace"), "desk"); assert!(t.at("link").b("hosted"));
    assert_eq!(tabs_in(&w, "away").len(), 1); assert_eq!(tabs_in(&w, "nested").len(), 1);
    // a space you opened at a worktree of the repo counts too, and the one in front comes first
    let side = w.base.join("side"); git(&w.root, &["worktree", "add", "-q", "-b", "side", &side.display().to_string()]);
    plain_space(&w, "side", "side", &side.display().to_string(), false);
    w.patch(|s| { for x in s["workspaces"].as_array_mut().unwrap() { x["focused"] = json!(x.s("workspace_id") == "side"); } });
    worker(&w, 2);
    let b = w.add("At the side");
    delegate(&b);
    let t = settle(&b);
    assert_eq!(t.at("link").s("workspace"), "side"); assert!(t.at("link").b("hosted"));
}

#[test]
fn the_repo_space_herdr_opened_beside_a_worktree_of_the_boards_never_takes_a_task_tab_and_still_closes_after_its_last_child() {
    let w = tab_world();
    worker(&w, 1);
    let a = w.add("Own space");
    delegate_with(&a, json!({ "workspace.place": "space" }));   // herdr opens the repo itself beside a's worktree
    let ta = settle(&a);
    assert_eq!(ta.s("stage"), "approval", "{}", ta.at("failure"));
    let parent = w.state().arr("workspaces").iter().find(|x| x.s("cwd") == w.root_str()).map(|x| x.t("workspace_id")).expect("herdr's repo space");
    worker(&w, 2);
    let b = w.add("In a tab");
    delegate(&b);
    let tb = settle(&b);
    assert_eq!(tb.s("stage"), "approval", "{}", tb.at("failure"));
    assert_ne!(tb.at("link").s("workspace"), parent);
    assert_eq!(tabs_in(&w, &parent).len(), 1);
}

#[test]
fn the_start_fails_and_opens_no_space_when_herdr_does_not_say_which_spaces_are_open() {
    let w = tab_world();
    plain_space(&w, "desk", "My desk", &w.root_str(), false);
    w.patch(|s| { s["tabListFails"] = json!(true); });
    worker(&w, 1);
    let a = w.add("Unanswered");
    delegate(&a);
    let t = settle(&a);
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(made(&w, "workspace create") + made(&w, "worktree create"), 0);
    assert_eq!(tabs_in(&w, "desk").len(), 1);
}
