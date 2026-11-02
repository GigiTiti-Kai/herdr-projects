//! The open spaces of a repo, for a task set to a tab: one herdr's worktree commands made for the repo, or one whose
//! first tab has a pane working in a checkout of it (its own folder or one of its worktrees), however it was opened.
//! A folder inside the repo that is a repo of its own does not count. herdr is asked for each of its lists once.
use crate::err::{Error, Res};
use crate::flow::spaces::real;
use crate::herdr::cli::args;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::repos::git_out;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The spaces of the repo at `root`, as `workspace list` gives them. Err: herdr did not answer, or git could not say
/// which checkouts the repo has; a pane's folder git knows nothing of is outside the repo.
pub fn spaces(api: &Herdr, root: &Path) -> Res<Vec<Value>> {
    let unknown = |e: String| Error::new(format!("Could not tell which spaces of {} are open, so the task did not start: {e}", root.display()));
    let all = api.workspaces().map_err(|e| unknown(e.to_string()))?;
    let plain = |w: &Value| !w.at("worktree").is_object();
    let (mut tabs, mut panes, mut checkouts, mut common) = (vec![], vec![], vec![], PathBuf::new());
    if all.iter().any(plain) {
        tabs = api.call(args(&["tab", "list"])).map_err(|e| unknown(e.to_string()))?.arr("tabs").to_vec();
        panes = api.panes(None).map_err(|e| unknown(e.to_string()))?;
        checkouts = git_out(root, &["worktree", "list", "--porcelain"]).map_err(|e| unknown(e.msg))?
            .lines().filter_map(|l| l.strip_prefix("worktree ")).map(|p| real(Path::new(p))).collect();
        common = common_dir(root).map_err(|e| unknown(e.msg))?;
    }
    let mut seen: HashMap<PathBuf, bool> = HashMap::new();
    // git only for a folder inside a checkout: it is slow on some disks (/mnt/c)
    let mut in_repo = |dir: &str| {
        let dir = real(Path::new(dir));
        if !checkouts.iter().any(|c| dir.starts_with(c)) { return false; }
        *seen.entry(dir.clone()).or_insert_with(|| common_dir(&dir).is_ok_and(|d| d == common))
    };
    let me = real(root);
    Ok(all.into_iter().filter(|w| {
        if !plain(w) { return real(Path::new(w.at("worktree").s("repo_root"))) == me; }
        let Some(first) = tabs.iter().find(|t| t.s("workspace_id") == w.s("workspace_id")) else { return false };
        panes.iter().filter(|p| p.s("tab_id") == first.s("tab_id")).any(|p| in_repo(p.s("cwd")))
    }).collect())
}
fn common_dir(dir: &Path) -> Res<PathBuf> { git_out(dir, &["rev-parse", "--path-format=absolute", "--git-common-dir"]).map(|d| real(Path::new(&d))) }
