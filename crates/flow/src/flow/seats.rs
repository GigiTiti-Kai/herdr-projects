//! What the board measures for a row no agent of its own works (docs/ongoing.md): the seats in the folder
//! `board.seats` names for it, as herdr lists them now (`seat`), and the panes its message names that herdr no longer
//! has (`gonePanes`). Both from the one `agent list` and `pane list` the board already reads; neither when herdr was
//! not read.
use super::live::{terminals, Session};
use crate::json::*;
use crate::shown::ongoing::{by_hand, unassigned};
use crate::store::settings::{lookup::setting, values::list_of};
use crate::store::tasks::Task;
use std::collections::HashSet;
use std::sync::Arc;

/// The entries of `board.seats`: repo and the words of its rows' titles, in lower case.
pub fn rules() -> Vec<(String, Vec<String>)> {
    list_of(&setting("board.seats", None)).iter().map(|e| { let (n, w) = crate::shown::projects::rule(e); (n.to_string(), w) }).collect()
}
/// The repo whose seats work this row by hand, by the first entry with a word in its title.
pub fn seat_of<'a>(rules: &'a [(String, Vec<String>)], r: &Obj) -> Option<&'a str> {
    let title = r.s("title").to_lowercase();
    rules.iter().find(|(_, words)| words.iter().any(|w| title.contains(w.as_str()))).map(|(n, _)| n.as_str())
}
/// Does this row get the measured line 2: at work, by hand, with no agent of the board's.
pub fn measured(r: &Obj) -> bool { by_hand(r) && unassigned(r) }
/// Is herdr worth reading for these tasks without any agent task: one is worked by hand.
pub fn wanted(tasks: &[Arc<Task>]) -> bool { tasks.iter().any(|t| t.s("stage") == "progress" && !t.b("archivedAt")) }
/// A pane id as herdr writes it (`wEN:p1`, `wBQ:p1K`) in a row's message.
pub fn panes_named(msg: &str) -> Vec<String> {
    let mut out: Vec<String> = crate::re!(r"w[0-9A-Z][0-9A-Za-z]{0,5}:p[0-9][0-9A-Za-z]{0,3}").find_iter(msg).map(|m| m.as_str().to_string()).collect();
    let mut seen = HashSet::new();
    out.retain(|p| seen.insert(p.clone()));
    out
}
/// Put `seat` and `gonePanes` on the rows that get the measured line 2. Down: none, so they say `エージェントなし`.
pub fn mark(rows: &mut [Obj], tasks: &[Arc<Task>], session: &Session, socket: &str) {
    let Session::Up { agents, panes } = session else { return };
    let rules = rules();
    // The board's own agents, reviewers and coordinators are no seat: by terminal, which herdr never hands out again.
    let mut own: HashSet<String> = tasks.iter().flat_map(|t| { let mut k = terminals(t); k.push(t.at("link").at("reviewer").t("terminal")); k }).collect();
    own.extend(crate::coordinator::state::all(socket).into_iter().filter_map(|co| crate::coordinator::state::get(&co)).map(|c| c.t("terminal")));
    own.remove("");
    let roots = crate::store::repos::known_repos();
    for r in rows.iter_mut().filter(|r| measured(r)) {
        let gone: Vec<String> = panes_named(r.s("msg")).into_iter().filter(|p| !panes.contains_key(p)).collect();
        if !gone.is_empty() { r.put("gonePanes", gone); }
        let Some(repo) = seat_of(&rules, r) else { continue };
        let root = roots.s(repo).trim_end_matches('/').to_string();
        let mut here: Vec<&Value> = agents.values().filter(|a| !root.is_empty() && (a.s("cwd") == root || a.s("cwd").starts_with(&format!("{root}/"))) && !own.contains(&a.t("terminal_id"))).collect();
        here.sort_by_key(|a| a.t("pane_id"));
        let mut kinds: Vec<(String, i64)> = vec![];
        for a in &here {
            let k = a.t("agent");
            match kinds.iter_mut().find(|(n, _)| *n == k) { Some(x) => x.1 += 1, None => kinds.push((k, 1)) }
        }
        let names = |st: &str| here.iter().filter(|a| a.s("agent_status") == st).map(|a| if a.b("name") { a.t("name") } else { a.t("pane_id") }).collect::<Vec<_>>();
        r.put("seat", json!({ "repo": repo, "kinds": kinds, "working": names("working"), "blocked": names("blocked") }));
    }
}
