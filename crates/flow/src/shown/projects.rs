//! The projects you hold, for the band over the list (docs/projects.md): each open task under the first project of
//! `board.projects` one of whose words is its repo or stands in its title, else under its repo; with how many open
//! tasks each holds and the most urgent section one of them stands in. A project here is a theme of your work, which
//! may span repos, not a repo.
use super::ongoing::{section_of, SECTIONS};
use crate::json::*;

/// One project of the band: its name, its open tasks, the most urgent section one of them stands in and how many do.
#[derive(Clone, Debug, PartialEq)]
pub struct Band { pub name: String, pub n: usize, pub at: &'static str, pub at_n: usize }

/// A section in the band's words, as a chip names where a task stands (board/view/kind.rs).
pub fn word(section: &str) -> &'static str {
    match section { "needs you" => "要対応", "ready to approve" => "承認待ち", "working" => "作業中", super::ongoing::ONGOING => "運用中", "done" => "完了", _ => "未着手" }
}
/// What a project of the band says after its count: where its most urgent tasks stand, with how many where not all do
/// (`要対応1`).
pub fn says(b: &Band) -> String { if b.at_n < b.n { format!("{}{}", word(b.at), b.at_n) } else { word(b.at).to_string() } }
/// `name=word|word`, or `name` alone (its own word), as `board.projects` holds it; the words in lower case.
fn rule(e: &str) -> (&str, Vec<String>) {
    let (name, words) = e.split_once('=').unwrap_or((e, e));
    (name, words.split('|').map(str::trim).filter(|w| !w.is_empty()).map(str::to_lowercase).collect())
}
/// The project a task's row is under: the first rule with a word that is its repo (another machine's by its id
/// there) or is in its title, letters in any case; else its repo.
pub fn project_of(rules: &[String], r: &Obj) -> String {
    let repo = (if r.b("rid") { r.s("rid") } else { r.s("id") }).split('/').next().unwrap_or("").to_string();
    let (low, title) = (repo.to_lowercase(), r.s("title").to_lowercase());
    rules.iter().map(|e| rule(e)).find(|(_, words)| words.iter().any(|w| *w == low || title.contains(w.as_str()))).map(|(n, _)| n.to_string()).unwrap_or(repo)
}
/// The band: every project with an open task (not done, not archived), the most urgent first, then the one that holds
/// more, then by name.
pub fn bands<'a>(rules: &[String], rows: impl IntoIterator<Item = &'a Obj>, now: i64) -> Vec<Band> {
    let rank = |s: &str| SECTIONS.iter().position(|x| *x == s).unwrap_or(SECTIONS.len());
    let mut out: Vec<Band> = vec![];
    for r in rows.into_iter().filter(|r| !r.b("archivedAt")) {
        let at = section_of(r, now);
        if at == "done" { continue; }
        let name = project_of(rules, r);
        let b = match out.iter().position(|b| b.name == name) { Some(k) => &mut out[k], None => { out.push(Band { name, n: 0, at, at_n: 0 }); out.last_mut().unwrap() } };
        b.n += 1;
        if rank(at) < rank(b.at) { b.at = at; b.at_n = 0; }
        if at == b.at { b.at_n += 1; }
    }
    out.sort_by(|a, b| rank(a.at).cmp(&rank(b.at)).then(b.n.cmp(&a.n)).then(a.name.cmp(&b.name)));
    out
}
