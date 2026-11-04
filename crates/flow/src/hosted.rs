//! A task that works in a tab of a space you have open (`workspace.place` = `tab`): a space of its repo that no task
//! owns, the one you look at first, then one named after the repo, then any other. Its worktree is its own, as ever; its agent, reviewer
//! and planner each have a tab there named after the task (`PRJ-1 Agent`). The space stays yours: nothing of yours in it
//! is split, typed into, renamed or closed, and when the task is done only its own panes close (and with them its tabs).
//! The task's link says so: `hosted`, and `ownsWorkspace` false. A space opened without herdr's worktree commands
//! counts when its first tab works in the repo; the repo space herdr opened for the board never does. With none open,
//! the board opens the repo's space itself, the task's agent in its first tab, and herdr closes it with its last pane.
//! Set to `pane`, the agent's pane is split off beside yours in a tab of that space instead (split.rs).
mod new_space;
mod of_repo;
mod split;
use crate::err::Res;
use crate::flow;
use crate::herdr::cli::{args, same_socket};
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::attempts::attempt_count;
use crate::store::files::{board_env, data_dir, with_lock_wait};
use crate::store::keys::{repo_keys, tag_of};
use crate::store::repos::{git_out, git_slow};
use crate::store::settings::lookup::setting;
use crate::store::tasks::{all, Task};
use std::path::Path;

/// The task works in a tab of a space it does not own.
pub fn is(t: &Obj) -> bool { t.at("link").b("hosted") }
/// The task's agent pane is split off in a tab it shares with yours (`workspace.place` = `pane`): the tab is not its own.
fn split(t: &Obj) -> bool { t.at("link").b("split") }
/// A planner of its own holds the task's link, in a tab of its own, while the agent's pane waits as the plan's `home`.
fn planning(t: &Obj) -> bool { t.at("plan").at("home").is_object() }
/// The link names that split-off pane, so the tab it names is not the task's.
pub fn shared(t: &Obj) -> bool { split(t) && !planning(t) }
/// Asked for at the start: `workspace.place` is `tab` or `pane`, and the task is done once (attempts share a space of their own).
pub fn wanted(t: &Obj) -> bool {
    !t.b("attemptOf") && [json!("tab"), json!("pane")].contains(&setting("workspace.place", Some(t))) && attempt_count(&setting("attempts.by", Some(t))) <= 1
}
/// Its next place is a pane beside yours. A planner and an attempt always have a tab of their own.
fn beside(t: &Obj) -> bool { !planning(t) && !t.b("attemptOf") && setting("workspace.place", Some(t)) == json!("pane") }
/// What the link says of the place it was just given (`placed`: what opened it, null for a space of its own).
pub fn mark(t: &mut Obj, placed: &Value) {
    let waits = planning(t); // a planner's new tab leaves the agent's pane where it is
    let l = t.obj_mut("link");
    if placed.b("hosted") { l.put("ownsWorkspace", false); l.put("hosted", true); } else { l.del("hosted"); }
    if placed.b("split") { l.put("split", true); } else if !waits { l.del("split"); }
}
/// A tab's name in a space of yours starts with the task's key, so you can tell whose it is (an attempt's: its task's).
pub fn label(t: &Obj, base: &str) -> String {
    if !is(t) { return base.into(); }
    let id = if t.b("attemptOf") { t.t("attemptOf") } else { t.t("id") };
    let tag = tag_of(&id, &repo_keys());
    format!("{} {base}", if tag.is_empty() { id } else { tag })
}
/// The tasks of `api`'s herdr session: herdr's ids are each session's own, so another's say nothing of this one's.
fn of_session(api: &Herdr) -> Vec<Task> { let here = api.socket_path(); all().into_iter().filter(|t| same_socket(t.at("link").s("socket"), &here)).collect() }
/// A space of the repo at `root` that no task owns: `prefer` while it still is one, else the one you look at, else the
/// one named after the repo, else any other. A space id herdr gave out again to another task's space is that task's,
/// never a host; nor is the repo space herdr opened for the board, which the board closes after its last task.
/// Err: herdr did not say which spaces are open.
fn host(api: &Herdr, root: &Path, prefer: &str) -> Res<Option<String>> {
    let mut not: Vec<String> = of_session(api).iter().filter(|t| t.at("link").b("ownsWorkspace")).map(|t| t.at("link").t("workspace")).collect();
    not.extend(flow::spaces::made_by_herdr(api, root));
    let of_repo: Vec<Value> = of_repo::spaces(api, root)?.into_iter().filter(|w| !not.contains(&w.t("workspace_id"))).collect();
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(of_repo.iter().find(|w| !prefer.is_empty() && w.s("workspace_id") == prefer)
        .or_else(|| of_repo.iter().find(|w| w.b("focused")))
        .or_else(|| of_repo.iter().find(|w| w.s("label") == name))
        .or_else(|| of_repo.first()) // one you named otherwise is still yours
        .map(|w| w.t("workspace_id")))
}
/// A new tab in `ws` at `dir`, labelled `label` (herdr numbers one with none): answers as `herdr worktree create`
/// does, marked `hosted`.
fn tab_in(api: &Herdr, ws: &str, dir: &str, label: &str, t: &Obj) -> Res<Value> {
    let r = api.tab_create(ws, dir, label, &board_env())?;
    let tab = if r.at("tab").b("tab_id") { r.at("tab").t("tab_id") } else { r.at("root_pane").t("tab_id") };
    let pane = if r.at("root_pane").is_object() { Some(r.at("root_pane").clone()) } else { api.panes(Some(ws))?.into_iter().find(|p| p.s("tab_id") == tab) };
    let Some(pane) = pane else { if !tab.is_empty() { api.tab_close(&tab); } crate::fail!("herdr opened no terminal for {}", t.s("id")) };
    Ok(json!({ "workspace": { "workspace_id": ws }, "tab": { "tab_id": if tab.is_empty() { pane.t("tab_id") } else { tab } }, "root_pane": pane, "worktree": { "path": dir }, "hosted": true }))
}
/// The task's agent tab's name in a space of yours (`PRJ-1 Agent`).
fn own_label(t: &Obj) -> String { let mut named = t.clone(); mark(&mut named, &json!({ "hosted": true })); flow::tabs::agent_tab(Some(&named)) }
/// The task's agent at `dir`: a tab (or, set to `pane`, a pane beside yours) in a space found (`prefer` first), else
/// the repo's space the board opens. One repo at a time, from the search to the space made, so two tasks starting at
/// once never open two spaces, nor split the same pane.
fn place(api: &Herdr, root: &Path, dir: &str, t: &Obj, prefer: &str) -> Res<Value> {
    with_lock_wait(&format!("host:{}|{}", api.socket_path(), flow::spaces::real(root).display()), 60000, || {
        if let Some(ws) = host(api, root, prefer)? { // a search herdr did not answer fails the start: never a second space
            match if beside(t) { split::beside(api, &ws, dir, t) } else { tab_in(api, &ws, dir, &own_label(t), t) } {
                Ok(r) => return Ok(r),
                Err(e) => match api.call(args(&["workspace", "get", &ws])) {
                    Err(gone) if gone.code.contains("not_found") => {} // herdr closed it as the tab was asked for: the repo gets a space again
                    _ => return Err(e), // still there, or herdr did not answer: never a second space
                },
            }
        }
        new_space::open(api, root, dir, t, beside(t))
    })
}
/// A first start that asked for a tab: its worktree, made as herdr makes one, and its place (`place`). None: the task
/// does not work in a tab.
pub fn open(api: &Herdr, root: &Path, t: &Obj, branch: &str, from: &str) -> Res<Option<Value>> {
    if !wanted(t) { return Ok(None); }
    let name: String = branch.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '-' }).collect();
    let dir = data_dir().join("worktrees").join(t.s("repo")).join(name).display().to_string();
    git_slow(root, &["worktree", "add", "--no-track", "-b", branch, &dir, from], 120000)?; // outside the lock: a big repo takes a while
    place(api, root, &dir, t, "").map(Some).inspect_err(|_| { let _ = git_out(root, &["worktree", "remove", &dir]); let _ = git_out(root, &["branch", "-D", branch]); })
}
/// A new tab at the task's worktree: in the space it was in while that is still a host, else placed as at a start.
fn tab_again(api: &Herdr, root: &Path, t: &Obj) -> Res<Option<Value>> {
    let prefer = if is(t) { t.at("link").t("workspace") } else { String::new() };
    place(api, root, &crate::store::repos::cwd_of(t).display().to_string(), t, &prefer).map(Some)
}
/// A task that works in a tab and whose terminal is gone (restarted, or gone into): a tab again. None: the task does
/// not work in a tab.
pub fn reopen(api: &Herdr, root: &Path, t: &Obj) -> Res<Option<Value>> {
    if is(t) || (wanted(t) && !t.at("link").b("ownsWorkspace")) { tab_again(api, root, t) } else { Ok(None) }
}
/// Delegated again with its worktree still there: a tab again where it worked in one, or asks for one now. None: the
/// task does not work in a tab. A task that still owns a space of its own stays there: moved out, that space would be
/// nobody's and never close.
pub fn again(api: &Herdr, root: &Path, t: &Obj) -> Res<Option<Value>> {
    if is(t) || (wanted(t) && !t.at("link").b("ownsWorkspace")) { tab_again(api, root, t) } else { Ok(None) }
}
/// The task's tabs and the pane each was opened with: its agent's (or planner's), the reviewer's, and the agent's
/// home while a planner has the first. `shared`: the agent's pane split off in a tab of yours, which is not the task's.
fn own(t: &Obj) -> Vec<(String, String, bool)> {
    let (l, home) = (t.at("link"), t.at("plan").at("home"));
    [(l.t("tab"), l.t("pane"), shared(t)), (l.at("reviewer").t("tab"), l.at("reviewer").t("pane"), false), (home.t("tab"), home.t("pane"), split(t))]
        .into_iter().filter(|(tab, pane, _)| !tab.is_empty() && !pane.is_empty()).collect()
}
/// Is this pane the task's: in one of its own tabs (a pane you split there counts: it is looked at before anything
/// closes), or its own pane in a tab it shares with yours (yours there are not).
pub fn in_tabs(t: &Obj, p: &Value) -> bool { own(t).iter().any(|(tab, pane, shared)| if *shared { p.s("pane_id") == pane } else { p.s("tab_id") == tab }) }
/// Close the task's own panes, and with the last one each tab: only a pane still in the tab it was opened in (a pane's
/// id can be given out again), never one of yours, and never the space.
pub fn close_tabs(api: &Herdr, t: &Obj) { for (tab, pane, _) in own(t) { close_own(api, &tab, &pane); } }
/// Close one pane of the task's own (its agent's, planner's or reviewer's), only while it is still in the tab it was
/// opened in. Its tab goes with its last pane: one you split off in it keeps it open.
pub fn close_own(api: &Herdr, tab: &str, pane: &str) {
    if pane.is_empty() { return; }
    if api.pane_get(pane).is_some_and(|p| tab.is_empty() || p.s("tab_id") == tab) { api.pane_close(pane); }
}
