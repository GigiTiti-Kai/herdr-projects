//! Whether a hook is the human's own, and the folder its files must be in. A plugin they allowed (its id and the git
//! address it is installed from, `board.trustHookPlugins`), enabled in Codex's config.toml, installed from that
//! address, its checkout clean and on its origin's default branch. Or a file in a repo of theirs (`board.trustHookRepos`,
//! `owner/name` on GitHub), tracked and the same as on that repo's default branch at origin: never a change on a work
//! branch nobody has reviewed yet. shortcut: what origin's branches are is read from the local refs, which anything on
//! this machine can move; asking GitHub itself would close that, when it is worth a network call per dialog.
use super::list::Hook;
use crate::store::repos::git_out;
use std::path::{Path, PathBuf};

/// What the human allows: plugins as (id, git address), repos as `owner/name`.
pub struct Allowed { pub plugins: Vec<(String, String)>, pub repos: Vec<String> }

/// A git address written one way: no `.git` or `/` at the end, `git@github.com:` as https, in lower case.
pub fn same_address(a: &str) -> String {
    let a = a.trim().trim_end_matches('/').trim_end_matches(".git");
    a.strip_prefix("git@github.com:").map(|r| format!("https://github.com/{r}")).unwrap_or_else(|| a.to_string()).to_lowercase()
}
/// `owner/name` of a GitHub address.
fn repo_name(url: &str) -> Option<String> { same_address(url).strip_prefix("https://github.com/").map(str::to_string).filter(|r| r.split('/').count() == 2) }
fn real(p: &Path) -> PathBuf { p.canonicalize().unwrap_or_else(|_| p.to_path_buf()) }

/// A plugin's hook: the plugin's own folder, and what shows it is the one allowed.
pub fn plugin(h: &Hook, a: &Allowed, config: &toml::Table) -> Result<(PathBuf, String), String> {
    let id = h.plugin.as_str();
    let Some((_, url)) = a.plugins.iter().find(|(p, _)| p == id) else { return Err(format!("plugin {id} is not one you allowed (board.trustHookPlugins)")) };
    let on = config.get("plugins").and_then(|p| p.get(id)).and_then(|p| p.get("enabled")).and_then(|v| v.as_bool()) == Some(true);
    if !on { return Err(format!("plugin {id} is not enabled in Codex's config.toml")); }
    let market = id.rsplit_once('@').map(|(_, m)| m).unwrap_or("");
    let from = config.get("marketplaces").and_then(|m| m.get(market)).and_then(|m| m.get("source")).and_then(|v| v.as_str()).unwrap_or("");
    if same_address(from) != same_address(url) { return Err(format!("plugin {id} is installed from {}, not from {url}", if from.is_empty() { "a place Codex's config.toml does not name" } else { from })); }
    // The hook file's place in the plugin is in its key: `<plugin>:<path>:<event>:…`.
    let rel = h.key.strip_prefix(&format!("{id}:")).and_then(|r| r.split(':').next()).unwrap_or("");
    let root = h.file.to_str().filter(|_| !rel.is_empty()).and_then(|f| f.strip_suffix(rel)).map(|r| PathBuf::from(r.trim_end_matches('/')))
        .ok_or_else(|| format!("plugin {id}'s hook file {} is not where its key says", h.file.display()))?;
    let top = git_out(&root, &["rev-parse", "--show-toplevel"]).map_err(|_| format!("plugin {id}'s folder {} is not a git checkout", root.display()))?;
    if real(Path::new(&top)) != real(&root) { return Err(format!("plugin {id}'s folder {} is inside another checkout ({top})", root.display())); }
    let origin = git_out(&root, &["remote", "get-url", "origin"]).unwrap_or_default();
    if same_address(&origin) != same_address(url) { return Err(format!("plugin {id}'s checkout is of {}, not of {url}", if origin.is_empty() { "no origin" } else { &origin })); }
    let changed = git_out(&root, &["status", "--porcelain"]).map_err(|e| format!("plugin {id}'s checkout does not read: {}", e.msg))?;
    if let Some(first) = changed.lines().next() { return Err(format!("plugin {id}'s checkout has changes of its own ({})", first.trim())); }
    let main = default_branch(&root).ok_or_else(|| format!("plugin {id}'s checkout has no default branch of its origin"))?;
    if git_out(&root, &["merge-base", "--is-ancestor", "HEAD", &main]).is_err() { return Err(format!("plugin {id}'s checkout is at a commit that is not on {main} of {url}")); }
    let head = git_out(&root, &["rev-parse", "--short", "HEAD"]).unwrap_or_default();
    Ok((root, format!("plugin {id} from {url}, its checkout clean at {head} on {main}")))
}

/// The repo's default branch at origin, as its remote-tracking ref: `origin/HEAD` only when it names one of origin's
/// branches (a link of it to a local branch would make a work branch look like origin's).
fn default_branch(top: &Path) -> Option<String> {
    let full = git_out(top, &["rev-parse", "--symbolic-full-name", "origin/HEAD"]).unwrap_or_default();
    if let Some(b) = full.strip_prefix("refs/remotes/origin/") { return Some(format!("origin/{b}")); }
    if !full.is_empty() { return None; }
    ["origin/main", "origin/master"].into_iter().find(|b| git_out(top, &["rev-parse", "--verify", "--quiet", &format!("refs/remotes/{b}")]).is_ok()).map(String::from)
}

/// A file in one of the human's repos, tracked, and on disk the same as on its default branch at origin: its repo's
/// folder, and what shows it. What is on disk is compared, so a file git is told to look away from still counts.
pub fn owned_file(f: &Path, a: &Allowed) -> Result<(PathBuf, String), String> {
    let dir = f.parent().unwrap_or(Path::new("/"));
    let top = git_out(dir, &["rev-parse", "--show-toplevel"]).map(PathBuf::from).map_err(|_| format!("{} is in no git repo", f.display()))?;
    let origin = git_out(&top, &["remote", "get-url", "origin"]).unwrap_or_default();
    let Some(name) = repo_name(&origin).and_then(|n| a.repos.iter().find(|r| r.eq_ignore_ascii_case(&n)).cloned()) else {
        return Err(format!("{} is in {}, not a repo you allowed (board.trustHookRepos)", f.display(), if origin.is_empty() { "a repo with no origin".to_string() } else { origin }));
    };
    let rel = real(f).strip_prefix(real(&top)).map(|r| r.display().to_string()).map_err(|_| format!("{} is outside its repo {}", f.display(), top.display()))?;
    if git_out(&top, &["ls-files", "--error-unmatch", "--", &rel]).is_err() { return Err(format!("{rel} in {name} is not tracked by git")); }
    let main = default_branch(&top).ok_or_else(|| format!("{name} has no default branch at origin to compare {rel} with"))?;
    let on_main = git_out(&top, &["rev-parse", &format!("{main}:{rel}")]).ok();
    if on_main.is_none() || on_main != git_out(&top, &["hash-object", "--", &rel]).ok() { return Err(format!("{rel} in {name} is not the same as on {main}")); }
    Ok((top, format!("{rel} in {name}, the same as on {main}")))
}
