//! The 運用中 section and the rows no agent of the board's works: a task said to be at work (`report --state working`,
//! by you, a coordinator or a resident seat), and one of those that has stayed so long that its end is not coming, which
//! the list shows under its own heading after `working` (docs/ongoing.md). Its group stays `working` everywhere else.
use super::states::{group_of, GROUPS};
use crate::json::*;

/// The heading the long-running rows stand under.
pub const ONGOING: &str = "運用中";
/// What line 2 of a row no agent of the board's works begins with.
pub const NO_AGENT: &str = "エージェントなし";
/// The list's sections, in the order they need you: the groups, with 運用中 after `working`.
pub const SECTIONS: [&str; 6] = [GROUPS[0], GROUPS[1], GROUPS[2], ONGOING, GROUPS[3], GROUPS[4]];
/// How long after it was added a row with no agent of the board's on it counts as ongoing: a day.
pub const AFTER: i64 = 24 * 3600 * 1000;

/// Said to be at work, with no agent of the board's on it: the board cannot tell who, or whether anyone is.
pub fn by_hand(r: &Obj) -> bool { !r.b("archivedAt") && r.s("state") == "working" && r.s("stage") == "progress" }
/// A row with no agent of the board's on it, added more than a day ago: its end is not coming.
// shortcut: counted from when the task was added (`since` moves with every report), so a task that waited long in to do
// is ongoing as soon as it is started by hand; record when it first went to work if that bites.
pub fn ongoing(r: &Obj, now: i64) -> bool { by_hand(r) && r.at("created").is_number() && now - r.i("created") > AFTER }
/// The section a row stands in: its group, or 運用中.
pub fn section_of(r: &Obj, now: i64) -> &'static str { if ongoing(r, now) { ONGOING } else { group_of(r) } }
