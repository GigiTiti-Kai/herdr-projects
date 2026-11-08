//! Parking a worker that waits for you (`workspace.park`, off unless you turn it on; `herdr-projects park <id>`, never
//! by itself): a task at `approval` or `asks` has its agent closed out as a done task's is (closeout.rs: a record-only
//! session-close, its save accepted, its Exit decision, `/exit`, its process ended), and is left so, its stage, approved
//! commit, question and inbox as they were and its pane open at its shell. While it is under way the turns it causes are
//! its own: the hook and the finish line take none of them for yours (`parking`), and nothing else is typed in (a reply,
//! an approval, the board's own turns wait). It comes back only the way an exited agent does (going into the task, a
//! reply, a restart), in the same conversation: one that cannot be resumed starts nothing (`may_start`), never a fresh
//! one. An approval needs no agent: it merges, and the clean-up finds the agent already gone. What cannot be proved holds
//! it (`held`), and it is then an ordinary task again; a crash (an agent gone before `/exit`) or herdr not answering is
//! never taken for a park.
use super::closeout::{held as hold_why, prompt, save_to, step, who_of, Gate};
use crate::err::Res;
use crate::json::*;
use crate::runner::{runner_alive, spawn_runner};
use crate::store::inbox::pending_text;
use crate::store::settings::lookup::setting;
use crate::store::tasks::{find, get, update_wait, Change, Task};

pub const STAGES: [&str; 2] = ["approval", "asks"];
pub fn on(t: Option<&Task>) -> bool { setting("workspace.park", t) == json!(true) }
fn rec(t: &Obj) -> &Value { t.at("closeout") }
/// Its agent is closed out and gone, its park proved.
pub fn parked(t: &Obj) -> bool { rec(t).b("park") && rec(t).b("parked") }
/// A park under way, or done, on a task that still waits for you: its turns are its own.
pub fn parking(t: &Obj) -> bool { rec(t).b("park") && !rec(t).b("held") && STAGES.contains(&t.s("stage")) }
/// Under way and not yet through: nothing else is typed into its agent.
pub fn busy(t: &Obj) -> bool { parking(t) && !parked(t) }
/// A park's record that proves no exit (held, or left behind by a task that moved on): it counts for nothing.
pub fn unfinished(t: &Obj) -> bool { rec(t).b("park") && (!parked(t) || rec(t).b("held")) }
pub fn drop_record(id: &str) { let _ = update_wait(id, 10000, |c| { if !c.at("closeout").b("park") { return Ok(Change::Keep); } c.put("closeout", Value::Null); Ok(Change::Write) }); }
/// Refused while a park is under way: what would be typed into its agent, or change what it waits on.
pub fn refuse_busy(t: &Obj) -> Res<()> {
    if busy(t) { crate::fail!("{} is being parked (its closeout is under way): nothing was sent or changed. Try again once it is parked or held.", t.s("id")); }
    Ok(())
}

/// Ask for it: one task, by you. Every check before anything is typed; the closeout's first step at once.
pub fn park(id: &str) -> Res<(Task, String)> {
    let t = find(id)?;
    let id = t.t("id");
    if !on(Some(&t)) { crate::fail!("Parking is off: herdr-projects set workspace.park true (only for the tasks you choose to park)."); }
    if !STAGES.contains(&t.s("stage")) { crate::fail!("{id} waits for nothing of yours: only a task that waits for your approval or your answer is parked."); }
    if t.b("archivedAt") || !crate::flow::task::is_agent_task(&t) || !t.at("link").b("pane") { crate::fail!("{id} has no agent of its own to park."); }
    if runner_alive(&t) { crate::fail!("{id} is busy. Try again in a moment."); }
    if !pending_text(&id).is_empty() { crate::fail!("Words wait to go to {id}'s agent: nothing was parked."); }
    if rec(&t).is_object() { crate::fail!("{id} has a closeout record already ({}): nothing was parked.", if parked(&t) { "parked" } else { rec(&t).s("phase") }); }
    if !crate::delegate::launch::resumable(&t) { crate::fail!("{id}'s conversation could not be resumed afterwards, so it is not parked."); }
    update_wait(&id, 10000, |c| { c.put("closeout", json!({ "park": true })); Ok(Change::Write) })?;
    drive(&id);
    let c = get(&id).unwrap_or(t);
    let line = c.t("msg");
    Ok((c, line))
}
/// One look at a park under way (the housekeeping pass, every half minute, and `park` itself).
pub fn drive(id: &str) {
    let Some(t) = get(id) else { return };
    if !busy(&t) { return; }
    let g = match who_of(&t) { Some(w) => step(&w, rec(&t), &|p, whole| save_to(id, p, whole)), None => hold_why("its worktree cannot be read, so whether its work moved cannot be told") };
    if matches!(g, Gate::Wait(_)) && rec(&t).s("phase").is_empty() && get(id).map(|c| c.at("closeout").s("phase") == "checkpointing").unwrap_or(false) { crate::store::said_by::note_board(id, &prompt(t.s("agent"))); }
    let line = match &g { Gate::Go => "Parked: its agent exited after a record-only closeout; go into the task (or reply) to resume the same conversation".to_string(), Gate::Wait(l) | Gate::Held(l) => l.clone() };
    let _ = update_wait(id, 10000, |c| {
        if !busy(c) { return Ok(Change::Keep); }
        let was = if c.at("closeout").has("was") { c.at("closeout").t("was") } else { c.t("msg") };
        c.obj_mut("closeout").put("was", was.clone());
        match &g { Gate::Go => { c.obj_mut("closeout").put("parked", true); c.obj_mut("closeout").put("parkedAt", crate::clock::now()); } Gate::Held(_) => c.obj_mut("closeout").put("held", true), Gate::Wait(_) => {} }
        c.put("msg", format!("{was}{}{line}.", if was.is_empty() { "" } else if was.ends_with('.') { " " } else { ". " }));
        Ok(Change::Write)
    });
}
/// The housekeeping pass: a park under way is looked at again; one left behind by a task that moved on is dropped.
pub fn park_pass(tasks: &[Task], socket: &str) {
    for t in tasks {
        if !rec(t).b("park") || !crate::herdr::cli::same_socket(t.at("link").s("socket"), socket) || runner_alive(t) { continue; }
        if busy(t) { if crate::store::marks::due(&format!("park-{}", sha1_smol::Sha1::from(t.s("id")).digest().to_string()[..8].to_string()), 30000) { spawn_runner("park", t.s("id"), &[]); } }
        else if unfinished(t) && !STAGES.contains(&t.s("stage")) { drop_record(t.s("id")); }
    }
}
/// Before an agent is started on a parked task: only its own conversation again, never a fresh one or another.
pub fn may_start(t: &Obj, resume: Option<&str>) -> Res<()> {
    if parked(t) && !rec(t).b("held") && resume != Some(rec(t).s("session")) {
        crate::fail!("{} was parked, and its conversation cannot be resumed: nothing was started (a fresh one is never started in its place). Its worktree, stage and waiting words are as they were; archive it, or delegate it again after: herdr-projects park {} --forget", t.s("id"), t.s("id"));
    }
    Ok(())
}
/// Its agent is back in its own conversation: the park is over (a reopen puts its line back as before it: jobs.rs).
pub fn resumed(id: &str) {
    let _ = update_wait(id, 10000, |c| {
        if !parked(c) { return Ok(Change::Keep); }
        c.put("closeout", Value::Null);
        Ok(Change::Write)
    });
}
/// `park --forget`: you give up on its conversation; the record goes, and the task is an ordinary one again.
pub fn forget(id: &str) -> Res<(Task, String)> {
    let t = find(id)?;
    if !rec(&t).b("park") { crate::fail!("{} has no park to forget.", t.s("id")); }
    refuse_busy(&t)?;
    let id = t.t("id");
    let _ = update_wait(&id, 10000, |c| { if c.at("closeout").has("was") { let was = c.at("closeout").t("was"); c.put("msg", was); } c.put("closeout", Value::Null); Ok(Change::Write) });
    Ok((get(&id).unwrap_or(t), format!("{id}: its park is forgotten")))
}
