//! A task row's kind chip and where it was handed to: `[調査]`, `[自動]` or `[運用]` before its title, worked out from what
//! the task already has (no tagging by hand), and `[委任先 APP-26・作業中]` after it when its message says it was handed
//! to another task, kept whole where the name is cut (docs/row-kinds.md).
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
/// each by the tag its repo goes by now, with where it stands now: `[委任先 APP-26・作業中]`; "" when it names none. A
/// task not on the board (archived) is named without a state; a number no repo has or had, not at all.
pub fn handed(s: &State, r: &Obj) -> String {
    // shortcut: another machine's row has that machine's keys, not ours; named there, not here, until rows carry them.
    if r.b("host") { return String::new(); }
    // A row's `msg` is its shown note; the task's own message is `raw` (shown/states.rs: as_row).
    let said = if r.has("raw") { r.s("raw") } else { r.s("msg") };
    // Each `…に委任` the message says, but not one it takes back right after (`に委任しない`, `に委任せず`, `に委任を取り消し`).
    let no = crate::re!(r"^(?:し(?:ない|ません|なかった|てい?ない)|せず|せぬ|は?しない|を?(?:やめ|止め|取り?消|取りやめ|中止|解除|撤回|キャンセル))");
    let named: Vec<String> = crate::re!(r"((?:[A-Z][A-Z0-9]{1,4}-\d+(?:\([^)]*\))?\s*[・、,と]?\s*)+)に[^。、\s]{0,6}委任").captures_iter(said)
        .filter(|c| !no.is_match(&said[c.get(0).map(|m| m.end()).unwrap_or(0)..]))
        .flat_map(|c| crate::re!(r"[A-Z][A-Z0-9]{1,4}-\d+").find_iter(c.get(1).map(|m| m.as_str()).unwrap_or("")).map(|m| m.as_str().to_string()).collect::<Vec<_>>())
        .filter_map(|n| {
            // A key the repo went by before still names its tasks, as every command takes it (store/keys_given.rs).
            let id = match id_of_key(&n, Some(&s.data.keys)) { x if x.is_empty() => before(&n), x => x };
            if id.is_empty() || id == r.s("id") { return None; }
            let tag = say_id(&id, &s.data.keys);
            Some(match s.data.items.iter().find(|x| x.s("id") == id) { Some(x) => format!("{tag}・{}", stands(x)), None => tag })
        }).collect();
    if named.is_empty() { String::new() } else { format!("[委任先 {}]", named.join(", ")) }
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
    if x.b("archivedAt") { return "アーカイブ済み"; }
    match group_of(x) { "needs you" => "要対応", "ready to approve" => "承認待ち", "working" => "作業中", "done" => "完了", _ => "未着手" }
}
