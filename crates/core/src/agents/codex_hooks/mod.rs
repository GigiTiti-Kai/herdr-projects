//! Codex's "Hooks need review" dialog at a seat's start, answered by the board itself when every hook it asks about is
//! the human's own and does nothing a hook of theirs should not; otherwise left for the human. The check stays on:
//! nothing is skipped, and Codex's own "Review hooks" screen is never opened. More: docs/codex-hooks-trust.md.
pub mod dialog;
pub mod list;
pub mod origin;
pub mod press;
pub mod scan;

use crate::store::settings::{lookup::setting, values::list_of};
use list::Hook;
use origin::Allowed;
use std::path::Path;

/// What the human allows, from their own settings (never a repo's file: the code under review cannot widen it).
pub fn allowed() -> Allowed {
    let plugins = list_of(&setting("board.trustHookPlugins", None)).iter().filter_map(|p| p.split_once('=').map(|(id, url)| (id.trim().to_string(), url.trim().to_string()))).collect();
    Allowed { plugins, repos: list_of(&setting("board.trustHookRepos", None)) }
}

/// The hooks the dialog asks about, each the human's own and clean, and as many as the screen counts: what was found
/// for each (Ok), or the first reason not to trust them (Err). `config` is Codex's config.toml, `home` the human's.
pub fn decide(shown: usize, hooks: &[Hook], a: &Allowed, config: &Path, home: &Path) -> Result<Vec<String>, String> {
    let pending: Vec<&Hook> = hooks.iter().filter(|h| h.pending()).collect();
    let events = pending.iter().map(|h| h.event.as_str()).collect::<Vec<_>>().join(", ");
    if pending.len() != shown { return Err(format!("the screen counts {shown} new or changed hooks, Codex's own list {} ({events})", pending.len())); }
    if pending.is_empty() { return Err("Codex's own list has no hook to trust".into()); }
    let config = std::fs::read_to_string(config).ok().and_then(|t| t.parse::<toml::Table>().ok()).unwrap_or_default();
    let mut found = vec![];
    for h in pending {
        let why = |e: String| format!("{} hook {} ({}): {e}", h.event, h.key, h.command.chars().take(120).collect::<String>());
        let (root, from) = if h.source == "plugin" { origin::plugin(h, a, &config) } else { origin::owned_file(&h.file, a) }.map_err(why)?;
        let files = scan::files(&h.command, &root, home).map_err(why)?;
        for (f, _) in &files {
            let inside = if h.source == "plugin" { f.canonicalize().map(|p| p.starts_with(root.canonicalize().unwrap_or(root.clone()))).unwrap_or(false) } else { origin::owned_file(f, a).is_ok() };
            if !inside { return Err(why(format!("it runs {}, which is not {}", f.display(), if h.source == "plugin" { "in the plugin's own folder" } else { "a tracked file of a repo you allowed, the same as on its default branch" }))); }
        }
        if let Some(bad) = scan::flag(&h.command) { return Err(why(format!("its command {bad}"))); }
        if let Some((f, bad)) = files.iter().find_map(|(f, text)| scan::flag(text).map(|b| (f, b))) { return Err(why(format!("{} {bad}", f.display()))); }
        let read: Vec<String> = files.iter().map(|(f, _)| f.strip_prefix(&root).unwrap_or(f).display().to_string()).collect();
        found.push(format!("{} {}: {from}; read {}", h.event, h.key, if read.is_empty() { "its command only".into() } else { read.join(", ") }));
    }
    Ok(found)
}
