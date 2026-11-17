//! A task row's kind chip and where it was handed to: `[調査]`, `[自動]` or `[運用]` before its title, worked out from what
//! the task already has (no tagging by hand), and `[委任先 APP-26・作業中]` after it when its message says it was handed
//! to another task or names one that names it back, kept whole where the name is cut (docs/row-kinds.md).
use crate::board::state::State;
use crate::json::*;
use crate::research::is_research;
use crate::shown::ink::Inked;
use crate::shown::states::group_of;
use crate::store::keys::{id_of_key, say_id};
use crate::text::width;

/// The chip: research only (as `[research]` says) is 調査; a task an agent works in a worktree of its own (a `worker.*`
/// setting, or a worktree or its branch, which stays once the worktree is gone) is 自動; any other is 運用 (a ledger
/// you or a resident seat work through by hand).
pub fn chip(r: &Obj) -> &'static str {
    if is_research(r) { return "[調査]"; }
    let worker = r.at("settings").as_object().is_some_and(|s| s.keys().any(|k| k.starts_with("worker.")));
    if worker || r.b("worktree") || r.b("branch") { "[自動]" } else { "[運用]" }
}
/// The tasks its message says it was handed to (`APP-26に委任`, `OPS-46・OPS-47に委任`, `APP-38(…)に仕様ごと委任済み`),
/// and, on a live row no agent of the board's works, those its message names that name it back in their title or
/// description: each by the tag its repo goes by now. Those not done are named with where they stand, the done ones
/// counted: `[委任先 APP-26・作業中 +完了2]`, `[委任先 3件すべて完了]`; "" when it names none. A task not on the board
/// (archived) is named without a state; a number no repo has or had, not at all.
pub fn handed(s: &State, r: &Obj) -> String {
    let mut done = 0;
    let open: Vec<String> = handed_to(s, r).into_iter().filter_map(|(tag, at)| match at {
        Some("完了") => { done += 1; None }
        Some(at) => Some(format!("{tag}・{at}")),
        None => Some(tag),
    }).collect();
    match (open.is_empty(), done) {
        (true, 0) => String::new(),
        (true, n) => format!("[委任先 {n}件すべて完了]"),
        (false, 0) => format!("[委任先 {}]", open.join(", ")),
        (false, n) => format!("[委任先 {} +完了{n}]", open.join(", ")),
    }
}
/// The words that take a hand-over back when they follow it in its sentence: `委任しない`, `委任していません`,
/// `委任することは、拒否された`, `委任を取り消した`. A wrong target is worse than none, so any of them names nothing.
fn no() -> &'static regex::Regex { crate::re!(r"^[^。．.!?！？\n]*?(?:ない|ません|なかった|ず|ぬ|禁止|拒否|中止|取り?消|撤回|やめ|止め|解除|キャンセル|だめ|ダメ|不可|見送)") }
/// Whom it was handed to, in the order its message names them: each task's tag and where it stands (None: not on the
/// board).
fn handed_to(s: &State, r: &Obj) -> Vec<(String, Option<&'static str>)> {
    // shortcut: another machine's row has that machine's keys, not ours; named there, not here, until rows carry them.
    if r.b("host") { return vec![]; }
    // A row's `msg` is its shown note; the task's own message is `raw` (shown/states.rs: as_row).
    let said = if r.has("raw") { r.s("raw") } else { r.s("msg") };
    let num = crate::re!(r"[A-Z][A-Z0-9]{1,4}-\d+");
    // Each `…に委任` the message says, but none whose own sentence (up to the next 。, ! or ?) says no after it.
    let (mut named, mut taken_back) = (vec![], vec![]);
    for c in crate::re!(r"((?:[A-Z][A-Z0-9]{1,4}-\d+(?:\([^)]*\))?\s*[・、,と]?\s*)+)に[^。、\s]{0,6}委任").captures_iter(said) {
        let back = no().is_match(&said[c.get(0).map(|m| m.end()).unwrap_or(0)..]);
        let ns = num.find_iter(c.get(1).map(|m| m.as_str()).unwrap_or("")).map(|m| m.as_str().to_string());
        if back { taken_back.extend(ns) } else { named.extend(ns) }
    }
    // Named by number in its message and naming it back (a coordinator writes its parent's number into a task it hands
    // on), on a row at work or waiting for you that no agent of the board's works: not one it took back above, nor one
    // whose own sentence (past the brackets right after it) says no after it, at any place it is named.
    if crate::shown::ongoing::unassigned(r) && ["working", "needs you"].contains(&group_of(r)) {
        let own = own_tags(s, r);
        for m in num.find_iter(said) {
            let n = m.as_str();
            if named.iter().any(|x| x == n) || taken_back.iter().any(|x| x == n) || said[m.end()..].starts_with(|c: char| c.is_ascii_digit()) { continue; }
            let no_after = num.find_iter(said).filter(|k| k.as_str() == n && !said[k.end()..].starts_with(|c: char| c.is_ascii_digit())).any(|k| {
                let rest = &said[k.end()..];
                let rest = crate::re!(r"^\([^)]*\)").find(rest).map(|b| &rest[b.end()..]).unwrap_or(rest);
                no().is_match(rest)
            });
            if no_after { continue; }
            let id = match id_of_key(n, Some(&s.data.keys)) { x if x.is_empty() => before(n), x => x };
            let back = s.data.items.iter().find(|x| x.s("id") == id).is_some_and(|x| own.iter().any(|o| names(x.s("title"), o) || names(x.s("description"), o)));
            if back { named.push(n.to_string()); }
        }
    }
    let mut out: Vec<(String, Option<&'static str>)> = vec![];
    for n in named {
        // A key the repo went by before still names its tasks, as every command takes it (store/keys_given.rs).
        let id = match id_of_key(&n, Some(&s.data.keys)) { x if x.is_empty() => before(&n), x => x };
        if id.is_empty() || id == r.s("id") { continue; }
        let tag = say_id(&id, &s.data.keys);
        if out.iter().any(|(t, _)| *t == tag) { continue; }
        out.push((tag, s.data.items.iter().find(|x| x.s("id") == id).map(|x| stands(x))));
    }
    out
}
/// The tags a task goes by: by its repo's key of now, and by each key its repo went by before (store/keys_given.rs).
fn own_tags(s: &State, r: &Obj) -> Vec<String> {
    let id = r.s("id");
    let (repo, num) = id.split_once('/').unwrap_or((id, ""));
    let mut out = vec![say_id(id, &s.data.keys)];
    for (k, v) in crate::store::keys_given::had_keys() { if text(&v) == repo && !num.is_empty() { out.push(format!("{k}-{num}")); } }
    out.sort();
    out.dedup();
    out
}
/// Does `text` name task `tag` (`HCM-22`, not inside `HCM-221` or `XHCM-22`)?
fn names(text: &str, tag: &str) -> bool {
    text.match_indices(tag).any(|(i, _)| !text[..i].ends_with(|c: char| c.is_ascii_alphanumeric()) && !text[i + tag.len()..].starts_with(|c: char| c.is_ascii_digit()))
}
/// `OLD-26` -> `repo/26` by a key its repo went by before; "" for a key no repo ever had.
fn before(n: &str) -> String {
    let Some((k, num)) = n.split_once('-') else { return String::new() };
    crate::store::keys_given::had_keys().get(&k.to_uppercase()).map(|r| format!("{}/{}", text(r), num.parse::<u64>().unwrap_or(0))).unwrap_or_default()
}
/// A row's name cut to `w` cells, the tag of whom it was handed to kept whole: the title gives way before it.
pub fn cut_name(s: &State, r: &Obj, title: &str, inked: Inked, w: usize) -> Inked {
    let tag = handed(s, r);
    let at = if tag.is_empty() || width(title) <= w { None } else { title.rfind(&tag) };
    let Some(at) = at else { return inked.cut(w) };
    let head = title[..at].trim_end();
    let tail = inked.skip(width(head) + 1);
    // Too narrow for both: cut as any name is.
    if w < tail.width() + 12 { return inked.cut(w); }
    super::fields::name_ink(r, head).cut(w - tail.width() - 1).add(Inked::plain(" ")).add(tail)
}
/// Where a task stands, in the words of the chip: the board's groups (shown/states.rs: group_of).
fn stands(x: &Obj) -> &'static str {
    if x.b("archivedAt") { "アーカイブ済み" } else { crate::shown::projects::word(group_of(x)) }
}
