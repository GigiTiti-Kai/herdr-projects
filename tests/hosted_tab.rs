//! `workspace.place` = `tab`: a task works in a tab of your own space of its repo, in a worktree of its own. Your panes
//! are never split, typed into or closed and your space keeps its name, at the start, a restart, going into it, clean-up
//! and archive; only the task's own panes close. No space of yours open: a space of its own, as ever.
use crate::common;
use common::herdr;
use common::*;
use serde_json::json;
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use sw::actions::{archive, finish_line, talk};
use sw::herdr::cli::{Fake, Ran};
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
/// A pane of yours, added in `tab` as you would split one off there.
fn yours_in(fake: &herdr::FakeHerdr, tab: &str) -> String {
    let mut st = fake.st.borrow_mut();
    let ws = tab.split(':').next().unwrap_or("").to_string();
    let id = format!("{ws}:u{}", st.arr("panes").len());
    st["panes"].as_array_mut().unwrap().push(json!({ "pane_id": id, "workspace_id": ws, "tab_id": tab, "terminal_id": format!("term_{id}"), "cwd": "/tmp", "agent": null }));
    id
}
/// The world's fake herdr, with a look at every call first (`on`), as you would look while a step runs.
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
/// Once the reviewer of `id` has its brief, a pane of yours is split off in its tab (and `then` runs); its id.
fn yours_beside_reviewer(w: &World, id: &str, then: impl Fn(&str) + 'static) -> Rc<RefCell<String>> {
    let mine: Rc<RefCell<String>> = Rc::default();
    let h = Rc::new(Hooked { inner: w.fake.clone(), on: RefCell::new(None), busy: Cell::new(false) });
    sw::herdr::cli::set_fake(Some(h.clone()));
    let (fake, id, m) = (w.fake.clone(), id.to_string(), mine.clone());
    *h.on.borrow_mut() = Some(Box::new(move |_| {
        if !m.borrow().is_empty() { return; }
        let rv = task(&id).at("link").at("reviewer").clone();
        let briefed = fake.st.borrow().arr("panes").iter().any(|p| p.s("pane_id") == rv.s("pane") && !p.at("agent").arr("prompts").is_empty());
        if !briefed { return; }
        *m.borrow_mut() = yours_in(&fake, rv.s("tab"));
        then(&id);
    }));
    mine
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
fn its_planner_and_reviewer_have_tabs_named_after_the_task_in_your_space_and_a_pane_of_yours_in_one_stays() {
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
    // a pane of yours beside the planner, and one beside the reviewer once it runs: each stays when its tab's agent goes
    let (planner, planner_tab) = (t.at("link").t("pane"), t.at("link").t("tab"));
    let by_planner = yours_in(&w.fake, &planner_tab);
    let by_reviewer = yours_beside_reviewer(&w, &b, |_| {});
    finish_line::approve(&b, None, false).unwrap();
    let t = settle(&b);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    let by_reviewer = by_reviewer.borrow().clone();
    assert!(!by_reviewer.is_empty(), "the review ran");
    let made: Vec<String> = calls(&w).into_iter().filter(|c| c.starts_with("tab create")).collect();
    assert!(made.iter().any(|c| c.starts_with("tab create --workspace mine ") && c.contains(&format!(" --label {} Reviewer #1 ", tag(&b)))), "{made:?}");
    assert!(!pane_there(&w, &planner)); assert!(pane_there(&w, &by_planner)); assert!(pane_there(&w, &by_reviewer));
    assert!(calls(&w).iter().all(|c| !c.starts_with("tab close")), "{:?}", calls(&w));
    finish_line::approve(&b, None, false).unwrap();
    let t = settle(&b);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(pane_there(&w, "mine:p1")); assert!(pane_there(&w, &by_planner)); assert!(pane_there(&w, &by_reviewer)); assert_eq!(label_of(&w, "mine"), name);
    assert!(!labels(&w).contains(&format!("{} Agent", tag(&b))), "{:?}", labels(&w));
    assert_eq!(splits(&w), 0); let mine = ["mine", "mine:p1", by_planner.as_str(), by_reviewer.as_str()];
    assert!(touched(&w, &mine).is_empty(), "{:?}", touched(&w, &mine));
}

#[test]
fn stopped_while_its_reviewer_works_the_reviewers_pane_closes_and_a_pane_of_yours_in_its_tab_stays() {
    let w = world_with(Opts { repo: true, settings: json!({ "workspace.place": "tab" }), ..Default::default() });
    your_space(&w, "mine", &repo_name(&w), false);
    w.script(json!({ "worker": [{ "files": { "h.txt": "x\n" }, "commit": "h", "note": "Did it." }], "reviewer": [{ "status": "working", "screen": "● Reading the diff\n" }] }));
    let b = w.add("Stopped in review");
    let yours = yours_beside_reviewer(&w, &b, |id| { sw::actions::talk::stop(id).unwrap(); });
    delegate(&b);
    let t = settle(&b);
    let yours = yours.borrow().clone();
    assert!(!yours.is_empty(), "the review ran: {}", t.s("stage"));
    assert!(t.at("link").at("reviewer").is_null() || !pane_there(&w, t.at("link").at("reviewer").s("pane")));
    assert!(pane_there(&w, &yours)); assert!(pane_there(&w, "mine:p1"));
    assert!(w.state().arr("panes").iter().all(|p| p.at("agent").is_null() || p.s("tab_id") != w.state().arr("panes").iter().find(|q| q.s("pane_id") == yours).unwrap().s("tab_id")), "the reviewer's own pane is gone");
    assert!(calls(&w).iter().all(|c| !c.starts_with("tab close") && !c.starts_with("workspace close")), "{:?}", calls(&w));
    assert!(touched(&w, &["mine", "mine:p1", yours.as_str()]).is_empty(), "{:?}", touched(&w, &["mine", "mine:p1", yours.as_str()]));
}

#[test]
fn a_space_of_the_repo_you_named_otherwise_and_do_not_look_at_is_still_used() {
    let w = tab_world();
    your_space(&w, "mine", "My coding desk", false);
    worker(&w, 1);
    let b = w.add("Elsewhere named");
    delegate(&b);
    let t = settle(&b);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert_eq!(t.at("link").s("workspace"), "mine"); assert!(t.at("link").b("hosted")); assert!(!t.at("link").b("ownsWorkspace"));
    assert_eq!(label_of(&w, "mine"), "My coding desk");
    assert!(calls(&w).iter().all(|c| !c.starts_with("worktree create")), "{:?}", calls(&w));
}
