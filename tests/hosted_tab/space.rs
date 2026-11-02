//! Which space a task set to a tab goes to: a space you opened yourself counts when its first tab works in the repo,
//! the repo space herdr opened beside a worktree of the board's never does, and with none open the board opens the
//! repo's space itself.
use super::{pane_there, repo_name, tab_label, tab_world, tabs_in, tag, worker, yours_in};
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::actions::{finish_line, talk};
use sw::herdr::Herdr;
use sw::json::*;

fn ws_of(w: &World, id: &str) -> Option<Value> { w.state().arr("workspaces").iter().find(|x| x.s("workspace_id") == id).cloned() }
fn panes_in(w: &World, ws: &str) -> Vec<String> { w.state().arr("panes").iter().filter(|x| x.s("workspace_id") == ws).map(|x| x.t("pane_id")).collect() }
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
    assert!(tb.at("link").b("hosted"));
    finish_line::approve(&a, None, false).unwrap();
    assert_eq!(settle(&a).s("stage"), "done");
    assert!(ws_of(&w, &parent).is_none(), "closed with its last child, as before");
    assert!(ws_of(&w, tb.at("link").s("workspace")).is_some());
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

#[test]
fn with_no_space_of_the_repo_open_the_board_opens_one_named_after_it_with_the_agent_in_its_first_tab_and_herdr_closes_it_with_the_last_task() {
    let w = tab_world();
    worker(&w, 1);
    let a = w.add("First");
    delegate(&a);
    let ta = settle(&a);
    assert_eq!(ta.s("stage"), "approval", "{}", ta.at("failure"));
    let space = ta.at("link").t("workspace");
    assert!(ta.at("link").b("hosted")); assert!(!ta.at("link").b("ownsWorkspace"));
    assert_eq!(ws_of(&w, &space).unwrap().t("label"), repo_name(&w));
    assert!(ws_of(&w, &space).unwrap().at("worktree").is_null());
    assert_eq!(panes_in(&w, &space), vec![ta.at("link").t("pane")]);
    assert_eq!(tabs_in(&w, &space), vec![ta.at("link").t("tab")]);
    assert_eq!(tab_label(&w, ta.at("link").s("tab")), format!("{} Agent", tag(&a)));
    assert_eq!(made(&w, "worktree create") + made(&w, "worktree open"), 0);
    assert!(!sw::store::files::data_dir().join("spaces.json").exists());
    assert_eq!(w.state().arr("workspaces").len(), 1);
    worker(&w, 2);
    let b = w.add("Second");
    delegate(&b);
    let tb = settle(&b);
    assert_eq!(tb.at("link").s("workspace"), space); assert!(tb.at("link").b("hosted"));
    assert_eq!(tabs_in(&w, &space), vec![ta.at("link").t("tab"), tb.at("link").t("tab")]);
    assert_eq!(w.state().arr("workspaces").len(), 1);
    finish_line::approve(&a, None, false).unwrap();
    assert_eq!(settle(&a).s("stage"), "done");
    assert_eq!(tabs_in(&w, &space), vec![tb.at("link").t("tab")]);
    finish_line::approve(&b, None, false).unwrap();
    assert_eq!(settle(&b).s("stage"), "done");
    assert!(ws_of(&w, &space).is_none());
    assert_eq!(made(&w, "workspace close"), 0);
}

#[test]
fn a_pane_of_yours_in_the_space_the_board_opened_keeps_it_open_after_its_last_task() {
    let w = tab_world();
    worker(&w, 1);
    let a = w.add("Only");
    delegate(&a);
    let t = settle(&a);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    let space = t.at("link").t("workspace");
    let yours = yours_in(&w.fake, t.at("link").s("tab"));
    finish_line::approve(&a, None, false).unwrap();
    assert_eq!(settle(&a).s("stage"), "done");
    assert!(ws_of(&w, &space).is_some()); assert!(pane_there(&w, &yours));
    assert!(!pane_there(&w, t.at("link").s("pane")));
    assert_eq!(made(&w, "workspace close"), 0);
}

#[test]
fn a_space_herdr_closes_as_the_tab_is_asked_for_gives_way_to_a_new_space_of_the_repo() {
    let w = tab_world();
    plain_space(&w, "going", &repo_name(&w), &w.root_str(), true);
    w.patch(|s| { s["tabGone"] = json!("going"); });
    worker(&w, 1);
    let a = w.add("Raced");
    delegate(&a);
    let t = settle(&a);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_ne!(t.at("link").s("workspace"), "going"); assert!(t.at("link").b("hosted"));
    assert_eq!(ws_of(&w, t.at("link").s("workspace")).unwrap().t("label"), repo_name(&w));
}

#[test]
fn restarted_or_gone_into_with_its_space_gone_it_opens_the_repos_space_again_and_never_reopens_a_worktree_space() {
    let w = tab_world();
    let api = Herdr::new(Some(w.socket.clone()));
    worker(&w, 1);
    let a = w.add("Restarted");
    delegate(&a);
    let t = settle(&a);
    let first = t.at("link").t("workspace");
    api.pane_close(t.at("link").s("pane")); // its only pane: herdr closes the space with it
    assert!(ws_of(&w, &first).is_none());
    let said = talk::tell(&a, "go on", false).unwrap().msg;
    assert!(said.contains("starting it again"), "{said}");
    let t = settle(&a);
    assert!(t.at("link").b("hosted")); assert!(!t.at("link").b("ownsWorkspace"));
    assert_eq!(ws_of(&w, t.at("link").s("workspace")).unwrap().t("label"), repo_name(&w));
    assert_eq!(tab_label(&w, t.at("link").s("tab")), format!("{} Agent", tag(&a)));
    assert_eq!(made(&w, "worktree open"), 0);
    // gone again, and you go into the task: the same
    api.pane_close(t.at("link").s("pane"));
    let (pane, _, t) = sw::delegate::reopen(&a, false).unwrap();
    assert!(t.at("link").b("hosted"));
    assert_eq!(ws_of(&w, pane.s("workspace_id")).unwrap().t("label"), repo_name(&w));
    assert_eq!(made(&w, "worktree open"), 0);
    // another task opens its tab there; its pane gone, a reply makes it a new tab in the same space
    worker(&w, 2);
    let b = w.add("Beside");
    delegate(&b);
    let tb = settle(&b);
    assert_eq!(tb.at("link").s("workspace"), pane.s("workspace_id"));
    api.pane_close(tb.at("link").s("pane"));
    talk::tell(&b, "go on", false).unwrap();
    let tb = settle(&b);
    assert_eq!(tb.at("link").s("workspace"), pane.s("workspace_id"));
    assert_eq!(made(&w, "worktree open"), 0);
}

#[test]
fn delegated_again_after_a_stop_with_its_space_gone_it_opens_the_repos_space_again() {
    let w = tab_world();
    let api = Herdr::new(Some(w.socket.clone()));
    worker(&w, 1);
    let a = w.add("Again");
    delegate(&a);
    let t = settle(&a);
    talk::stop(&a).unwrap();
    api.pane_close(t.at("link").s("pane"));
    delegate(&a);
    let t = settle(&a);
    assert_ne!(t.s("stage"), "failed", "{}", t.at("failure"));
    assert!(t.at("link").b("hosted"));
    assert_eq!(ws_of(&w, t.at("link").s("workspace")).unwrap().t("label"), repo_name(&w));
    assert_eq!(made(&w, "worktree open") + made(&w, "worktree create"), 0);
}

#[test]
fn a_planner_leaves_the_space_the_board_opened_with_the_agents_first_tab_in_place() {
    let w = world_with(Opts { repo: true, settings: json!({ "workspace.place": "tab", "plan.approve": "you", "plan.agent": "codex/gpt-5.5", "worker.agent": "claude", "review.when": "never" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    w.script(json!({ "worker": [{ "plan": "# Plan\n\n1. Add h.txt.\n", "note": "Planned." }, { "files": { "h.txt": "x\n" }, "commit": "h", "note": "Did it." }] }));
    let a = w.add("Planned");
    delegate(&a);
    let t = settle(&a);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    let space = t.at("link").t("workspace");
    assert!(t.at("link").b("hosted"));
    assert_eq!(tabs_in(&w, &space).len(), 2); // the agent's first tab and the planner's
    finish_line::approve(&a, None, false).unwrap();
    let t = settle(&a);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.at("link").s("workspace"), space); assert!(ws_of(&w, &space).is_some());
    assert_eq!(tabs_in(&w, &space), vec![t.at("link").t("tab")]);
}
