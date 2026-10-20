//! A task that works in a tab of a space you have open (`workspace.place` = `tab`): a space of its repo that no task
//! owns, the one you look at first, then one named after the repo, then any other. Its worktree is its own, as ever; its agent, reviewer
//! and planner each have a tab there named after the task (`PRJ-1 Agent`). The space stays yours: nothing of yours in it
//! is split, typed into, renamed or closed, and when the task is done only its own panes close (and with them its tabs).
//! The task's link says so: `hosted`, and `ownsWorkspace` false.
use crate::err::Res;
use crate::flow;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::attempts::attempt_count;
use crate::store::files::{board_env, data_dir};
use crate::store::keys::{repo_keys, tag_of};
use crate::store::repos::{git_out, git_slow};
use crate::store::settings::lookup::setting;
use crate::store::tasks::all;
use std::path::Path;

/// The task works in a tab of a space it does not own.
pub fn is(t: &Obj) -> bool { t.at("link").b("hosted") }
/// Asked for at the start: `workspace.place` is `tab`, and the task is done once (attempts share a space of their own).
pub fn wanted(t: &Obj) -> bool {
    !t.b("attemptOf") && setting("workspace.place", Some(t)) == json!("tab") && attempt_count(&setting("attempts.by", Some(t))) <= 1
}
/// What the link says of the place it was just given.
pub fn mark(t: &mut Obj, hosted: bool) {
    let l = t.obj_mut("link");
    if hosted { l.put("ownsWorkspace", false); l.put("hosted", true); } else { l.del("hosted"); }
}
/// A tab's name in a space of yours starts with the task's key, so you can tell whose it is.
pub fn label(t: &Obj, base: &str) -> String {
    if !is(t) { return base.into(); }
    let tag = tag_of(t.s("id"), &repo_keys());
    format!("{} {base}", if tag.is_empty() { t.t("id") } else { tag })
}
/// A space of the repo at `root` that no task owns: `prefer` while it still is one, else the one you look at, else the
/// one named after the repo, else any other. A space id herdr gave out again to another task's space is that task's, never a host.
fn host(api: &Herdr, root: &Path, prefer: &str) -> Option<String> {
    let owned: Vec<String> = all().iter().filter(|t| t.at("link").b("ownsWorkspace")).map(|t| t.at("link").t("workspace")).collect();
    let real = |p: &str| flow::spaces::real(Path::new(p));
    let of_repo: Vec<Value> = api.workspaces().ok()?.into_iter()
        .filter(|w| w.at("worktree").is_object() && real(w.at("worktree").s("repo_root")) == real(&root.display().to_string()) && !owned.contains(&w.t("workspace_id"))).collect();
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    of_repo.iter().find(|w| !prefer.is_empty() && w.s("workspace_id") == prefer)
        .or_else(|| of_repo.iter().find(|w| w.b("focused")))
        .or_else(|| of_repo.iter().find(|w| w.s("label") == name))
        .or_else(|| of_repo.first()) // one you named otherwise is still yours
        .map(|w| w.t("workspace_id"))
}
/// The task's agent tab in `ws`, at `dir`: answers as `herdr worktree create` does, marked `hosted`.
fn tab_in(api: &Herdr, ws: &str, dir: &str, t: &Obj) -> Res<Value> {
    let mut named = t.clone();
    mark(&mut named, true);
    let r = api.tab_create(ws, dir, &flow::tabs::agent_tab(Some(&named)), &board_env())?;
    let tab = if r.at("tab").b("tab_id") { r.at("tab").t("tab_id") } else { r.at("root_pane").t("tab_id") };
    let pane = if r.at("root_pane").is_object() { Some(r.at("root_pane").clone()) } else { api.panes(Some(ws))?.into_iter().find(|p| p.s("tab_id") == tab) };
    let Some(pane) = pane else { if !tab.is_empty() { api.tab_close(&tab); } crate::fail!("herdr opened no terminal for {}", t.s("id")) };
    Ok(json!({ "workspace": { "workspace_id": ws }, "tab": { "tab_id": if tab.is_empty() { pane.t("tab_id") } else { tab } }, "root_pane": pane, "worktree": { "path": dir }, "hosted": true }))
}
/// A first start that asked for a tab: its worktree, made as herdr makes one, and its tab in a space found. None: no
/// such space is open, and the task gets a space of its own.
pub fn open(api: &Herdr, root: &Path, t: &Obj, branch: &str, from: &str) -> Res<Option<Value>> {
    if !wanted(t) { return Ok(None); }
    let Some(ws) = host(api, root, "") else { return Ok(None) };
    let name: String = branch.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '-' }).collect();
    let dir = data_dir().join("worktrees").join(t.s("repo")).join(name).display().to_string();
    git_slow(root, &["worktree", "add", "--no-track", "-b", branch, &dir, from], 120000)?;
    tab_in(api, &ws, &dir, t).map(Some).inspect_err(|_| { let _ = git_out(root, &["worktree", "remove", &dir]); let _ = git_out(root, &["branch", "-D", branch]); })
}
/// A new tab at the task's worktree: in the space it was in while that is still a host, else in another found as at a
/// start. None: no such space is open.
fn tab_again(api: &Herdr, root: &Path, t: &Obj) -> Res<Option<Value>> {
    let prefer = if is(t) { t.at("link").t("workspace") } else { String::new() };
    let Some(ws) = host(api, root, &prefer) else { return Ok(None) };
    tab_in(api, &ws, &crate::store::repos::cwd_of(t).display().to_string(), t).map(Some)
}
/// A task that works in a tab and whose terminal is gone (restarted, or gone into): a tab again. None: not such a
/// task, or no space to be in, and it gets a space of its own.
pub fn reopen(api: &Herdr, root: &Path, t: &Obj) -> Res<Option<Value>> { if is(t) { tab_again(api, root, t) } else { Ok(None) } }
/// Delegated again with its worktree still there: a tab again where it worked in one, or asks for one now. A task that
/// still owns a space of its own stays there: moved out, that space would be nobody's and never close.
pub fn again(api: &Herdr, root: &Path, t: &Obj) -> Res<Option<Value>> {
    if is(t) || (wanted(t) && !t.at("link").b("ownsWorkspace")) { tab_again(api, root, t) } else { Ok(None) }
}
/// The task's own tabs and the pane each was opened with: its agent's (or planner's), the reviewer's, and the agent's
/// home while a planner has the first.
fn own(t: &Obj) -> Vec<(String, String)> {
    let l = t.at("link");
    [(l.t("tab"), l.t("pane")), (l.at("reviewer").t("tab"), l.at("reviewer").t("pane")), (t.at("plan").at("home").t("tab"), t.at("plan").at("home").t("pane"))]
        .into_iter().filter(|(tab, pane)| !tab.is_empty() && !pane.is_empty()).collect()
}
/// Is this pane in one of the task's own tabs (a pane you split there counts: it is looked at before anything closes).
pub fn in_tabs(t: &Obj, p: &Value) -> bool { own(t).iter().any(|(tab, _)| p.s("tab_id") == tab) }
/// Close the task's own panes, and with the last one each tab: only a pane still in the tab it was opened in (a pane's
/// id can be given out again), never one of yours, and never the space.
pub fn close_tabs(api: &Herdr, t: &Obj) { for (tab, pane) in own(t) { close_own(api, &tab, &pane); } }
/// Close one pane of the task's own (its agent's, planner's or reviewer's), only while it is still in the tab it was
/// opened in. Its tab goes with its last pane: one you split off in it keeps it open.
pub fn close_own(api: &Herdr, tab: &str, pane: &str) {
    if pane.is_empty() { return; }
    if api.pane_get(pane).is_some_and(|p| tab.is_empty() || p.s("tab_id") == tab) { api.pane_close(pane); }
}
