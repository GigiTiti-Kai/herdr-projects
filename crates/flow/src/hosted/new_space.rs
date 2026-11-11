//! A repo space the board opens for a task set to a tab (or a pane) when none of the repo's is open: named after the
//! repo's folder, at the task's worktree, with the task's agent in its first and only pane. No task owns it, so the
//! repo's later tasks open their tabs (or split their panes) there; herdr closes it with its last pane, and the board
//! never does.
use crate::err::Res;
use crate::flow;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::files::board_env;
use std::path::Path;

/// Open the repo's space at `dir` for `t`: answers as a tab of a space found does, marked `hosted`. `label`: its first
/// tab's name. `split`: the task is set to a pane, and the repo's later tasks split off beside it, so its tab is named
/// after none of them.
pub fn open(api: &Herdr, root: &Path, dir: &str, t: &Obj, label: &str, split: bool) -> Res<Value> {
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let r = api.workspace_create(dir, &name, &board_env())?;
    let ws = r.at("workspace").t("workspace_id");
    let pane = if r.at("root_pane").is_object() { Some(r.at("root_pane").clone()) } else { api.panes(Some(&ws)).ok().and_then(|p| p.into_iter().next()) };
    let Some(pane) = pane else { let _ = api.workspace_close(&ws); crate::fail!("herdr opened no terminal for {}", t.s("id")) };
    let tab = if r.at("tab").b("tab_id") { r.at("tab").t("tab_id") } else { pane.t("tab_id") };
    if !split { flow::tabs::name_tab(api, &tab, label); }
    Ok(json!({ "workspace": { "workspace_id": ws }, "tab": { "tab_id": tab }, "root_pane": pane, "worktree": { "path": dir }, "hosted": true, "split": split }))
}
