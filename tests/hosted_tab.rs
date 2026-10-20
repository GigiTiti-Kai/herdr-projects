//! `workspace.place` = `tab`: a task works in a tab of your own space of its repo, in a worktree of its own. Your panes
//! are never split, typed into or closed and your space keeps its name, at the start, a restart, going into it, clean-up
//! and archive; only the task's own panes close. No space of yours open: a space of its own, as ever.
use crate::common;
use common::*;
use serde_json::json;
use std::path::Path;
use sw::actions::{archive, finish_line, talk};
use sw::herdr::Herdr;
use sw::json::*;

fn tab_world() -> World { world_with(Opts { repo: true, settings: json!({ "review.when": "never", "workspace.place": "tab" }), ..Default::default() }) }
fn repo_name(w: &World) -> String { w.root.file_name().unwrap().to_string_lossy().into_owned() }
/// Your own space of the repo, open before any task: one tab, one pane at the repo with something running in it.
fn your_space(w: &World, id: &str, label: &str, focused: bool) {
    let root = w.root_str();
    w.patch(|s| {
        s["workspaces"].as_array_mut().unwrap().push(json!({ "workspace_id": id, "label": label, "cwd": root, "focused": focused, "nextT": 2, "worktree": { "checkout_path": root, "repo_root": root, "is_linked_worktree": false } }));
        s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": format!("{id}:p1"), "workspace_id": id, "tab_id": format!("{id}:t1"), "terminal_id": format!("term_{id}"), "cwd": root, "agent": null, "busy": true }));
        s["tabs"].as_array_mut().unwrap().push(json!({ "tab_id": format!("{id}:t1"), "workspace_id": id, "label": "1" }));
    });
}
fn worker(w: &World, n: i64) { w.script(json!({ "worker": [{ "files": { format!("h{n}.txt"): "x\n" }, "commit": format!("h{n}"), "note": "Did it." }, { "note": "Went on." }] })); }
fn label_of(w: &World, ws: &str) -> String { w.state().arr("workspaces").iter().find(|x| x.s("workspace_id") == ws).map(|x| x.t("label")).unwrap_or_default() }
fn tab_label(w: &World, tab: &str) -> String { w.state().arr("tabs").iter().find(|x| x.s("tab_id") == tab).map(|x| x.t("label")).unwrap_or_default() }
fn tabs_in(w: &World, ws: &str) -> Vec<String> { w.state().arr("tabs").iter().filter(|x| x.s("workspace_id") == ws).map(|x| x.t("tab_id")).collect() }
fn pane_there(w: &World, pane: &str) -> bool { w.state().arr("panes").iter().any(|p| p.s("pane_id") == pane) }
fn tag(id: &str) -> String { let g = sw::store::keys::tag_of(id, &sw::store::keys::repo_keys()); if g.is_empty() { id.to_string() } else { g } }
/// Every call that names one of `ids` and is no look: what was done to them, beyond opening a tab of the task's own.
fn touched(w: &World, ids: &[&str]) -> Vec<String> {
    let looks = ["workspace list", "workspace get", "tab list", "tab get", "tab create", "pane list", "pane get", "pane process-info", "pane layout", "pane read", "agent get", "agent list"];
    calls(w).into_iter().filter(|c| ids.iter().any(|id| c.split(' ').any(|x| x == *id)) && !looks.iter().any(|l| c.starts_with(l))).collect()
}
fn splits(w: &World) -> usize { calls(w).iter().filter(|c| c.starts_with("pane split")).count() }

#[test]
fn a_task_set_to_a_tab_works_in_a_tab_of_your_repo_space_never_a_tasks_own_and_leaves_with_its_own_tab_only() {
    let w = tab_world();
    let name = repo_name(&w);
    your_space(&w, "mine", &name, false);
    // a task asked for a space of its own gets one; that space, even in front, is never another task's host
    worker(&w, 1);
    let a = w.add("Own space");
    delegate_with(&a, json!({ "workspace.place": "space" }));
    let ta = settle(&a);
    assert_eq!(ta.s("stage"), "approval", "{}", ta.at("failure"));
    assert!(ta.at("link").b("ownsWorkspace")); assert!(!ta.at("link").b("hosted")); assert_ne!(ta.at("link").s("workspace"), "mine");
    let theirs = ta.at("link").t("workspace");
    w.patch(|s| { for x in s["workspaces"].as_array_mut().unwrap() { if x.s("workspace_id") == theirs { x["focused"] = json!(true); } } });
    // the task in a tab: in your space, named after it, in a worktree of its own
    worker(&w, 2);
    let b = w.add("In a tab");
    delegate(&b);
    let t = settle(&b);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.at("link").s("workspace"), "mine"); assert!(t.at("link").b("hosted")); assert!(!t.at("link").b("ownsWorkspace"));
    assert!(Path::new(t.s("worktree")).join("h2.txt").exists()); assert_ne!(t.s("worktree"), w.root_str());
    assert_eq!(tab_label(&w, t.at("link").s("tab")), format!("{} Agent", tag(&b)));
    assert_eq!(tabs_in(&w, "mine"), vec!["mine:t1".to_string(), t.at("link").t("tab")]);
    // herdr's sidebar: the task's pane says what it does, your space keeps its name
    sw::flow::sidebar::tell_sidebar(&t);
    assert_eq!(label_of(&w, "mine"), name);
    assert_eq!(splits(&w), 0); assert!(touched(&w, &["mine", "mine:p1"]).is_empty(), "{:?}", touched(&w, &["mine", "mine:p1"]));
    assert!(w.state().arr("focus").is_empty());
    // merged: its tab, worktree and branch go, with your command still running in your tab of the same space
    finish_line::approve(&b, None, false).unwrap();
    let t = settle(&b);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(t.b("cleaned"), "{}", t.at("kept"));
    assert!(!Path::new(&task(&b).t("worktree")).exists() || task(&b).at("worktree").is_null());
    assert_eq!(tabs_in(&w, "mine"), vec!["mine:t1".to_string()]); assert!(pane_there(&w, "mine:p1")); assert_eq!(label_of(&w, "mine"), name);
    assert!(touched(&w, &["mine", "mine:p1"]).is_empty(), "{:?}", touched(&w, &["mine", "mine:p1"]));
    assert!(w.state().arr("workspaces").iter().any(|x| x.s("workspace_id") == theirs));
}

#[test]
fn restarted_gone_into_and_archived_in_your_space_a_tab_of_its_own_again_and_only_its_own_panes_close() {
    let w = tab_world();
    let name = repo_name(&w);
    your_space(&w, "mine", &name, false);
    let api = Herdr::new(Some(w.socket.clone()));
    worker(&w, 1);
    let b = w.add("In a tab");
    delegate(&b);
    let t = settle(&b);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    // its terminal is gone: a reply starts it again in a new tab of your space, never in a split of a pane
    api.pane_close(t.at("link").s("pane"));
    let said = talk::tell(&b, "go on", false).unwrap().msg;
    assert!(said.contains("starting it again"), "{said}");
    let t = settle(&b);
    assert_eq!(t.at("link").s("workspace"), "mine"); assert!(t.at("link").b("hosted")); assert!(!t.at("link").b("ownsWorkspace"));
    assert_eq!(tab_label(&w, t.at("link").s("tab")), format!("{} Agent", tag(&b)));
    assert!(prompts(&w, t.at("link").s("pane")).iter().any(|p| p.contains("go on")));
    assert_eq!(splits(&w), 0); assert!(touched(&w, &["mine", "mine:p1"]).is_empty(), "{:?}", touched(&w, &["mine", "mine:p1"]));
    // gone again, and you go into the task: the same
    api.pane_close(t.at("link").s("pane"));
    let (pane, _, t) = sw::delegate::reopen(&b, false).unwrap();
    assert_eq!(pane.s("workspace_id"), "mine"); assert_eq!(t.at("link").s("pane"), pane.s("pane_id")); assert!(t.at("link").b("hosted")); assert!(!t.at("link").b("ownsWorkspace"));
    assert_eq!(tab_label(&w, pane.s("tab_id")), format!("{} Agent", tag(&b)));
    assert_eq!(splits(&w), 0); assert!(touched(&w, &["mine", "mine:p1"]).is_empty(), "{:?}", touched(&w, &["mine", "mine:p1"]));
    // a pane you split off in its tab is yours: archive closes the task's own pane and leaves yours, your tab and your space
    let beside = api.pane_split(pane.s("pane_id"), t.s("worktree"), "right", &[]).unwrap().t("pane_id");
    archive::archive(&b).unwrap();
    assert!(!pane_there(&w, pane.s("pane_id"))); assert!(pane_there(&w, &beside)); assert!(pane_there(&w, "mine:p1"));
    assert_eq!(label_of(&w, "mine"), name);
    assert!(touched(&w, &["mine", "mine:p1"]).is_empty(), "{:?}", touched(&w, &["mine", "mine:p1"]));
    assert!(calls(&w).iter().all(|c| !c.starts_with("workspace close")), "{:?}", calls(&w));
}

#[test]
fn with_no_space_of_yours_open_a_task_gets_a_space_of_its_own_and_the_space_you_look_at_comes_first() {
    let w = tab_world();
    worker(&w, 1);
    let a = w.add("No space of yours");
    delegate(&a);
    let t = settle(&a);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(t.at("link").b("ownsWorkspace")); assert!(!t.at("link").b("hosted"));
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("worktree create")).count(), 1);
    // one named after the repo and one in front: the one in front
    your_space(&w, "named", &repo_name(&w), false);
    your_space(&w, "front", "elsewhere", true);
    worker(&w, 2);
    let b = w.add("Looked at first");
    delegate(&b);
    let t = settle(&b);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.at("link").s("workspace"), "front"); assert!(t.at("link").b("hosted"));
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("worktree create")).count(), 1);
    assert!(touched(&w, &["named", "named:p1", "front", "front:p1"]).is_empty(), "{:?}", touched(&w, &["named", "named:p1", "front", "front:p1"]));
}

#[test]
fn its_planner_and_reviewer_have_tabs_named_after_the_task_in_your_space_and_go_with_it() {
    let w = world_with(Opts { repo: true, settings: json!({ "workspace.place": "tab", "plan.approve": "you", "plan.agent": "codex/gpt-5.5", "worker.agent": "claude" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    let name = repo_name(&w);
    your_space(&w, "mine", &name, false);
    w.script(json!({ "worker": [{ "plan": "# Plan\n\n1. Add h.txt.\n", "note": "Planned." }, { "files": { "h.txt": "x\n" }, "commit": "h", "note": "Did it." }], "reviewer": [{ "verdict": { "meets": true, "text": "Looked at it.", "findings": [] } }] }));
    let b = w.add("Planned in a tab");
    delegate(&b);
    let t = settle(&b);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    let labels = |w: &World| w.state().arr("tabs").iter().filter(|x| x.s("workspace_id") == "mine").map(|x| x.t("label")).collect::<Vec<_>>();
    assert_eq!(labels(&w), vec!["1".to_string(), format!("{} Agent", tag(&b)), format!("{} Planner", tag(&b))]);
    finish_line::approve(&b, None, false).unwrap();
    let t = settle(&b);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    let made: Vec<String> = calls(&w).into_iter().filter(|c| c.starts_with("tab create")).collect();
    assert!(made.iter().any(|c| c.starts_with("tab create --workspace mine ") && c.contains(&format!(" --label {} Reviewer #1 ", tag(&b)))), "{made:?}");
    finish_line::approve(&b, None, false).unwrap();
    let t = settle(&b);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(labels(&w), vec!["1"]); assert!(pane_there(&w, "mine:p1")); assert_eq!(label_of(&w, "mine"), name);
    assert_eq!(splits(&w), 0); assert!(touched(&w, &["mine", "mine:p1"]).is_empty(), "{:?}", touched(&w, &["mine", "mine:p1"]));
}
