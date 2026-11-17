//! The 運用中 section and the rows no agent of the board's works: a live task with none (said to be at work with
//! `report --state working` by you, a coordinator or a resident seat, or waiting for you), and one at work so long that
//! its end is not coming, which the list shows under its own heading after `working` (docs/ongoing.md). Its group stays
//! `working` everywhere else.
use super::queue::queued;
use super::states::{group_of, is_agent, is_live, GROUPS};
use crate::json::*;

/// The heading the long-running rows stand under.
pub const ONGOING: &str = "運用中";
/// What line 2 of a row no agent of the board's works begins with.
pub const NO_AGENT: &str = "エージェントなし";
/// The list's sections, in the order they need you: the groups, with 運用中 after `working`.
pub const SECTIONS: [&str; 6] = [GROUPS[0], GROUPS[1], GROUPS[2], ONGOING, GROUPS[3], GROUPS[4]];
/// How long after it began to be worked by hand a row counts as ongoing: a day.
pub const AFTER: i64 = 24 * 3600 * 1000;

/// Said to be at work, with no agent of the board's on it: the board cannot tell who, or whether anyone is.
pub fn by_hand(r: &Obj) -> bool { !r.b("archivedAt") && r.s("state") == "working" && r.s("stage") == "progress" }
/// A live row (at work, waiting for you, ready) with no agent of the board's on it (none named, no pane linked: as
/// flow/task.rs: is_agent_task), nor one on its way (queued).
pub fn unassigned(r: &Obj) -> bool { is_live(r) && !(is_agent(r) || (r.at("link").b("pane") && r.s("stage") != "todo")) && !queued(r) }
/// At work by hand for more than a day, counted from when it began (`progressFrom`, actions/filing.rs: report), never
/// from its last report (`since`); one at work before that was kept, from when its task was added.
pub fn ongoing(r: &Obj, now: i64) -> bool {
    let from = if r.at("progressFrom").is_number() { r.at("progressFrom") } else { r.at("created") };
    by_hand(r) && from.is_number() && now - from.as_i64().unwrap_or(now) > AFTER
}
/// The section a row stands in: its group, or 運用中.
pub fn section_of(r: &Obj, now: i64) -> &'static str { if ongoing(r, now) { ONGOING } else { group_of(r) } }
