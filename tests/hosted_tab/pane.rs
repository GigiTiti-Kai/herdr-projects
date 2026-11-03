//! `workspace.place` = `pane`: a task's agent pane is split off beside yours in a tab of your space of the repo (the
//! one you last looked at there, else its first, never a task's own), so the repo's agents all show at once. Your tab
//! keeps its name, the task's pane is named instead, and only the task's own pane closes: a pane of yours busy beside
//! it does not keep it in use. With no space of the repo open, the one the board opens is shared the same way.
use super::{label_of, pane_there, repo_name, tab_label, tabs_in, tag, touched, worker, your_space, yours_in};
use crate::common;
use common::*;
use serde_json::json;
use sw::actions::{finish_line, talk};
use sw::herdr::Herdr;
use sw::json::*;

fn pane_world() -> World { world_with(Opts { repo: true, settings: json!({ "review.when": "never", "workspace.place": "pane" }), ..Default::default() }) }
fn pane_label(w: &World, pane: &str) -> String { w.state().arr("panes").iter().find(|p| p.s("pane_id") == pane).map(|p| p.t("label")).unwrap_or_default() }
fn split_calls(w: &World) -> Vec<String> { calls(w).into_iter().filter(|c| c.starts_with("pane split")).collect() }
fn last_looked_at(w: &World, ws: &str, tab: &str) { w.patch(|s| { for x in s["workspaces"].as_array_mut().unwrap() { if x.s("workspace_id") == ws { x["active_tab_id"] = json!(tab); } } }); }

#[test]
fn a_task_set_to_a_pane_is_split_off_beside_yours_in_the_tab_you_last_looked_at_and_leaves_with_its_own_pane_only() {
    let w = pane_world();
    let name = repo_name(&w);
    your_space(&w, "mine", &name, false);
    // a second tab of yours, with a command running in it: the one you last looked at in that space
    let root = w.root_str();
    w.patch(|s| {
        s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": "mine:p2", "workspace_id": "mine", "tab_id": "mine:t2", "terminal_id": "term_mine2", "cwd": root, "agent": null, "busy": true }));
        s["tabs"].as_array_mut().unwrap().push(json!({ "tab_id": "mine:t2", "workspace_id": "mine", "label": "notes" }));
    });
    last_looked_at(&w, "mine", "mine:t2");
    worker(&w, 1);
    let b = w.add("Beside yours");
    delegate(&b);
    let t = settle(&b);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.at("link").s("workspace"), "mine"); assert_eq!(t.at("link").s("tab"), "mine:t2");
    assert!(t.at("link").b("hosted")); assert!(t.at("link").b("split")); assert!(!t.at("link").b("ownsWorkspace"));
    assert_ne!(t.s("worktree"), w.root_str());
    let split = split_calls(&w);
    assert_eq!(split.len(), 1, "{split:?}"); assert!(split[0].starts_with("pane split mine:p2 --direction right "), "{split:?}");
    // your tabs keep their names and no tab of its own opens: its pane is named in its border
    let pane = t.at("link").t("pane");
    assert_eq!(tabs_in(&w, "mine"), vec!["mine:t1".to_string(), "mine:t2".to_string()]);
    assert_eq!(tab_label(&w, "mine:t1"), "1"); assert_eq!(tab_label(&w, "mine:t2"), "notes"); assert_eq!(label_of(&w, "mine"), name);
    assert_eq!(pane_label(&w, &pane), format!("{} Agent", tag(&b)));
    assert!(calls(&w).iter().all(|c| !c.starts_with("tab create") && !c.starts_with("tab rename")), "{:?}", calls(&w));
    // merged: only its own pane closes, and your busy pane beside it in the same tab does not keep it in use
    finish_line::approve(&b, None, false).unwrap();
    let t = settle(&b);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(t.b("cleaned"), "{}", t.at("kept"));
    assert!(!pane_there(&w, &pane)); assert!(pane_there(&w, "mine:p1")); assert!(pane_there(&w, "mine:p2"));
    assert_eq!(tabs_in(&w, "mine"), vec!["mine:t1".to_string(), "mine:t2".to_string()]); assert_eq!(tab_label(&w, "mine:t2"), "notes");
    assert!(calls(&w).iter().all(|c| !c.starts_with("tab close") && !c.starts_with("workspace close")), "{:?}", calls(&w));
    assert!(touched(&w, &["mine", "mine:p1"]).is_empty(), "{:?}", touched(&w, &["mine", "mine:p1"]));
    assert_eq!(touched(&w, &["mine:p2"]), split, "your pane was only split");
}

#[test]
fn the_biggest_pane_of_your_tab_is_split_along_its_longer_side_never_a_tab_of_a_tasks_own_and_again_after_a_restart() {
    let w = pane_world();
    your_space(&w, "mine", &repo_name(&w), false);
    // a task set to a tab has a tab of its own in your space: the one you last looked at there
    worker(&w, 1);
    let a = w.add("In a tab");
    delegate_with(&a, json!({ "workspace.place": "tab" }));
    let ta = settle(&a);
    assert_eq!(ta.s("stage"), "approval", "{}", ta.at("failure")); assert!(!ta.at("link").b("split"));
    let theirs = ta.at("link").t("tab");
    last_looked_at(&w, "mine", &theirs);
    // your tab: a pane of yours as wide as it is tall on screen, and a short one under it
    let short = yours_in(&w.fake, "mine:t1");
    w.patch(|s| { s["layouts"]["mine:p1"] = json!({ "area": { "x": 0, "y": 0, "width": 60, "height": 44 }, "zoomed": false, "focused_pane_id": "mine:p1", "tab_id": "mine:t1", "workspace_id": "mine", "splits": [],
        "panes": [{ "pane_id": "mine:p1", "focused": true, "rect": { "x": 0, "y": 0, "width": 60, "height": 34 } }, { "pane_id": short, "focused": false, "rect": { "x": 0, "y": 34, "width": 60, "height": 10 } }] }); });
    worker(&w, 2);
    let b = w.add("Beside yours");
    delegate(&b);
    let t = settle(&b);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.at("link").s("tab"), "mine:t1"); assert!(t.at("link").b("split"));
    let split = split_calls(&w);
    assert_eq!(split.len(), 1, "{split:?}"); assert!(split[0].starts_with("pane split mine:p1 --direction down "), "{split:?}");
    assert_eq!(tab_label(&w, &theirs), format!("{} Agent", tag(&a))); assert_eq!(tab_label(&w, "mine:t1"), "1");
    // its terminal is gone: a reply splits it off beside yours again, and names it again
    Herdr::new(Some(w.socket.clone())).pane_close(t.at("link").s("pane"));
    let said = talk::tell(&b, "go on", false).unwrap().msg;
    assert!(said.contains("starting it again"), "{said}");
    let t = settle(&b);
    assert_eq!(t.at("link").s("workspace"), "mine"); assert_eq!(t.at("link").s("tab"), "mine:t1");
    assert!(t.at("link").b("split")); assert!(t.at("link").b("hosted")); assert!(!t.at("link").b("ownsWorkspace"));
    assert_eq!(pane_label(&w, t.at("link").s("pane")), format!("{} Agent", tag(&b)));
    assert!(prompts(&w, t.at("link").s("pane")).iter().any(|p| p.contains("go on")));
    assert_eq!(split_calls(&w).len(), 2); assert_eq!(tab_label(&w, "mine:t1"), "1");
}

#[test]
fn with_no_space_of_yours_open_the_repo_space_the_board_opens_is_shared_and_the_next_task_splits_off_beside_the_first() {
    let w = pane_world();
    worker(&w, 1);
    let a = w.add("First");
    delegate(&a);
    let ta = settle(&a);
    assert_eq!(ta.s("stage"), "approval", "{}", ta.at("failure"));
    let (ws, tab) = (ta.at("link").t("workspace"), ta.at("link").t("tab"));
    assert!(ta.at("link").b("split")); assert!(ta.at("link").b("hosted")); assert!(!ta.at("link").b("ownsWorkspace"));
    assert_eq!(label_of(&w, &ws), repo_name(&w)); assert_eq!(tab_label(&w, &tab), "1", "named after no task");
    assert_eq!(pane_label(&w, ta.at("link").s("pane")), format!("{} Agent", tag(&a)));
    worker(&w, 2);
    let b = w.add("Second");
    delegate(&b);
    let tb = settle(&b);
    assert_eq!(tb.s("stage"), "approval", "{}", tb.at("failure"));
    assert_eq!(tb.at("link").s("workspace"), ws); assert_eq!(tb.at("link").s("tab"), tab); assert!(tb.at("link").b("split"));
    let split = split_calls(&w);
    assert_eq!(split.len(), 1, "{split:?}"); assert!(split[0].starts_with(&format!("pane split {} ", ta.at("link").s("pane"))), "{split:?}");
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("workspace create")).count(), 1);
    // the first is merged: its pane closes, the second's agent beside it keeps nothing in use and stays
    finish_line::approve(&a, None, false).unwrap();
    let ta = settle(&a);
    assert_eq!(ta.s("stage"), "done", "{}", ta.at("failure")); assert!(ta.b("cleaned"), "{}", ta.at("kept"));
    assert!(pane_there(&w, tb.at("link").s("pane"))); assert_eq!(tabs_in(&w, &ws), vec![tab.clone()]);
}

#[test]
fn its_planner_has_a_tab_of_its_own_and_your_tab_keeps_its_name_while_the_agent_waits_beside_yours() {
    let w = world_with(Opts { repo: true, settings: json!({ "workspace.place": "pane", "review.when": "never", "plan.approve": "you", "plan.agent": "codex/gpt-5.5", "worker.agent": "claude" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap") };
    your_space(&w, "mine", &repo_name(&w), false);
    w.script(json!({ "worker": [{ "plan": "# Plan\n\n1. Add h.txt.\n", "note": "Planned." }, { "files": { "h.txt": "x\n" }, "commit": "h", "note": "Did it." }] }));
    let b = w.add("Planned beside yours");
    delegate(&b);
    let t = settle(&b);
    assert_eq!(t.s("stage"), "plan", "{}", t.at("failure"));
    let labels = |w: &World| w.state().arr("tabs").iter().filter(|x| x.s("workspace_id") == "mine").map(|x| x.t("label")).collect::<Vec<_>>();
    assert_eq!(labels(&w), vec!["1".to_string(), format!("{} Planner", tag(&b))]);
    let home = t.at("plan").at("home").t("pane");
    assert_eq!(t.at("plan").at("home").s("tab"), "mine:t1"); assert!(t.at("link").b("split"));
    finish_line::approve(&b, None, false).unwrap();
    let t = settle(&b);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.at("link").s("pane"), home); assert_eq!(t.at("link").s("tab"), "mine:t1");
    assert_eq!(labels(&w), vec!["1".to_string()], "the planner's tab closed, yours kept its name");
    assert_eq!(pane_label(&w, &home), format!("{} Agent", tag(&b)));
}
