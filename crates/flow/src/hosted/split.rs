//! A task set to a pane (`workspace.place` = `pane`): its agent's pane is split off beside yours in a tab of your space
//! of the repo, so the repo's agents all show at once. The tab is the one you last looked at there, else its first,
//! never a tab of a task's own (its reviewer's, planner's, or a task's set to a tab) nor the board's; its biggest pane
//! is split along its longer side. The tab stays yours: never renamed (the task's pane is named in its border) and only
//! the task's own pane closes. A space with no tab of yours gets a new one, which the repo's later tasks share the same way.
use crate::err::Res;
use crate::flow::tabs::BOARD_TAB;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::files::board_env;
use crate::store::tasks::all;

/// The task's agent pane at `dir`, split off in a tab of `ws`: answers as a tab of its own does, marked `split`.
pub fn beside(api: &Herdr, ws: &str, dir: &str, t: &Obj) -> Res<Value> {
    let theirs: Vec<String> = all().iter().flat_map(|o| super::own(o)).filter(|(_, _, shared)| !shared).map(|(tab, _, _)| tab).collect();
    let tabs: Vec<String> = api.tabs(ws)?.iter().filter(|x| x.s("label") != BOARD_TAB).map(|x| x.t("tab_id")).filter(|x| !theirs.contains(x)).collect();
    let active = api.workspace_get(ws).map(|w| w.t("active_tab_id")).unwrap_or_default();
    let Some(tab) = tabs.iter().find(|x| **x == active).or(tabs.first()).cloned() else {
        let mut r = super::tab_in(api, ws, dir, "", t)?; // none of yours: a tab for the tasks' panes, herdr numbers it
        r["split"] = json!(true);
        return Ok(r);
    };
    let panes: Vec<Value> = api.panes(Some(ws))?.into_iter().filter(|p| p.s("tab_id") == tab).collect();
    let Some((at, way)) = to_split(api, &panes) else { crate::fail!("herdr showed no pane in tab {tab} for {}", t.s("id")) };
    let pane = api.pane_split(&at, dir, way, &board_env())?;
    Ok(json!({ "workspace": { "workspace_id": ws }, "tab": { "tab_id": tab }, "root_pane": pane, "worktree": { "path": dir }, "hosted": true, "split": true }))
}
/// Which pane of one tab's `panes` to split, and which way: the biggest in herdr's window, along its longer side (a
/// cell is about twice as tall as it is wide), so the tab fills as a grid. herdr not saying where they sit: the first,
/// to the right.
fn to_split(api: &Herdr, panes: &[Value]) -> Option<(String, &'static str)> {
    let first = panes.first()?.t("pane_id");
    let rects = api.pane_layout(&first).map(|l| l.arr("panes").to_vec()).unwrap_or_default();
    let big = rects.iter().filter(|r| panes.iter().any(|p| p.s("pane_id") == r.s("pane_id"))).max_by_key(|r| r.at("rect").i("width") * r.at("rect").i("height"));
    Some(match big {
        Some(r) => (r.t("pane_id"), if r.at("rect").i("width") >= 2 * r.at("rect").i("height") { "right" } else { "down" }),
        None => (first, "right"),
    })
}
