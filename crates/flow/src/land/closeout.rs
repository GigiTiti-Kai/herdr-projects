//! The record-only closeout before a done task's space is closed (`workspace.closeout`, off unless you turn it on):
//! the board types a `/session-close` that ships, pushes, merges and reviews nothing into the working agent, as a turn
//! of its own that the finish line never takes for one (the task stays done: hook.rs acts on no turn of a done task).
//! Each step is proved apart, and the task's `closeout` record moves on only at a proved step (`phase`):
//! `checkpointing` (sent), `saved` (Agent Memory's queue holds a job accepted for that session, from that worktree,
//! since it was sent: memory_queue.rs, looked up again on every pass), `closing` (looked at again right before, then
//! `/exit` typed), `agent_exited` (herdr sees no agent in the pane and nothing runs at its shell), `pane_closed` (the
//! clean-up closed its space: cleanup.rs). New input (a turn beyond the closeout's own, another session, a new branch
//! tip), an agent gone before it was told to exit, a dialog, herdr not answering past `workspace.closeoutWait`, or no
//! accepted job by then: nothing more is typed and nothing is closed; the space is kept, with the reason. Nothing is
//! ever sent twice. Only the worker's own pane: no reviewer, watcher or planner.
use crate::clock::now;
use crate::flow::task::h;
use crate::herdr::turns::completion;
use crate::json::*;
use crate::memory_queue::{accepted, jobs_of};
use crate::store::repos::{folder_of, git_out};
use crate::store::settings::lookup::setting;
use crate::store::settings::values::ms;
use crate::store::tasks::{update_wait, Change, Task};
use crate::submit::{hand, note_sent};

/// Go: clean up. Wait: look again on the next pass. Held: keep the space, say why, and stop trying.
#[derive(Debug, PartialEq)]
pub enum Gate { Go, Wait(String), Held(String) }
pub const WORDS: &str = "記録だけの session-close。ship・push・merge・PR・再 review・削除はしない。自分が所有し現在の権限で止められる背景処理（監視・cron・接続）だけ止めてから報告。他者・共有・所有不明のものは止めず、巻き込む可能性があれば残して理由付きで保留";
/// What is typed: the skill's own command for the agent's program (Codex names skills with `$`).
pub fn prompt(agent: &str) -> String { format!("{} {WORDS}", if agent.starts_with("codex") { "$session-close" } else { "/session-close" }) }
pub const EXIT: &str = "/exit";
pub fn on(t: &Task) -> bool { setting("workspace.closeout", Some(t)) == json!(true) }
fn held(why: impl std::fmt::Display) -> Gate { Gate::Held(format!("closeout 未確認: {why}. Nothing more was typed; archive the task to close its space")) }
fn put(id: &str, f: impl Fn(&mut Obj)) { let _ = update_wait(id, 10000, |c| { f(c.obj_mut("closeout")); Ok(Change::Write) }); }

pub fn gate(t: &Task) -> Gate {
    if !on(t) || !t.at("link").b("pane") { return Gate::Go; }
    let (id, pane, rec) = (t.s("id"), t.at("link").s("pane"), t.at("closeout"));
    let phase = rec.s("phase");
    let since = if phase == "closing" { rec.i("exitAt") } else { rec.i("sentAt") };
    let late = !phase.is_empty() && now() - since > ms(setting("workspace.closeoutWait", Some(t)).as_str().unwrap_or("10m")).max(1);
    let wait = |why: &str| if late { held(why) } else { Gate::Wait(format!("Closeout not through yet: {why}")) };
    let api = h(t);
    let a = match api.agent_get(pane) {
        Err(e) => return wait(&format!("herdr did not answer for its agent ({})", e.code)), // an error is not a pane gone
        Ok(None) if phase == "agent_exited" => return Gate::Go,
        Ok(None) if phase == "closing" => {
            if !api.pane_idle(pane) { return wait("its agent left, but something still runs at its shell"); }
            put(id, |r| { r.put("phase", "agent_exited"); r.put("exitedAt", now()); });
            return Gate::Go;
        }
        Ok(None) if phase.is_empty() => return held("no agent in its pane to save a record, so none can be proved"),
        Ok(None) => return held("its agent left its pane before it was told to exit"),
        Ok(Some(a)) => a,
    };
    let status = a.s("agent_status");
    if phase == "agent_exited" { return held("an agent is in its pane again"); }
    if phase == "closing" { return wait(&format!("its agent was told to exit and is still there ({status})")); }
    let tip = if t.b("branch") { git_out(&folder_of(id), &["rev-parse", "--verify", &format!("refs/heads/{}", t.s("branch"))]).unwrap_or_default() } else { String::new() };
    let cur = completion(&a);
    if !phase.is_empty() {
        // New input since it was sent: what any save proves is out of date, and nothing more is typed.
        // shortcut: without the agent's hook (`events`), a turn of yours that ends before the board's first look after
        // the closeout is not told from the closeout's own; the hook's count tells it then.
        if rec.s("session") != t.s("session") { return held("its agent's session changed since it was sent"); }
        if rec.s("tip") != tip || rec.s("worktree") != t.s("worktree") { return held("its branch moved since it was sent"); }
        if status == "working" { return wait("its agent's turn is under way"); } // herdr may leave the count out meanwhile: looked at once it ends
        if (rec.has("comp1") && *rec.at("comp1") != cur) || t.i("eventSeq") - rec.i("events") > 1 { return held("new input reached its agent after it was sent"); }
        if status != "idle" && status != "done" { return held(format!("its agent is {} (a dialog, or herdr cannot place it)", if status.is_empty() { "unknown" } else { status })); }
        if !rec.has("comp1") && cur != *rec.at("comp0") { let c1 = cur.clone(); put(id, |r| r.put("comp1", c1.clone())); }
        let job = match jobs_of(t.s("session")) {
            Ok(jobs) => match accepted(&jobs, t.s("worktree"), rec.i("sentAt")) { Some(j) => j.id, None => return wait("Agent Memory has no accepted job for its session") },
            Err(e) => return wait(&e),
        };
        if phase == "checkpointing" { let j = job.clone(); put(id, |r| { r.put("phase", "saved"); r.put("job", j.clone()); r.put("savedAt", now()); }); }
        // The exit, decided now: still at rest, nothing running under it, the same job still there.
        if !api.pane_childless(pane, a.s("agent")) { return wait("a command still runs under its agent"); }
        if let Err(e) = hand(&api, pane, EXIT) { return held(format!("/exit could not be typed in ({})", e.message)); }
        put(id, |r| { r.put("phase", "closing"); r.put("exitAt", now()); r.put("job", job.clone()); });
        return Gate::Wait("Closeout saved: its agent was told to exit".into());
    }
    // Nothing sent yet: typed only into an agent plainly at rest.
    if status == "working" { return Gate::Wait("Closeout waits for the agent's turn to end".into()); }
    if status != "idle" && status != "done" { return held(format!("its agent is {} (a dialog, or herdr cannot place it), so nothing was typed", if status.is_empty() { "unknown" } else { status })); }
    if t.s("session").is_empty() { return held("its agent's session is not known, so its record could not be proved"); }
    if !api.pane_childless(pane, a.s("agent")) { return Gate::Wait("Closeout waits: a command still runs under its agent".into()); }
    let text = prompt(t.s("agent"));
    let sent = now(); // taken before it goes in: a job accepted before this is not its record
    match hand(&api, pane, &text) {
        Ok(mark) => {
            note_sent(id, mark, &text, false);
            let r = json!({ "phase": "checkpointing", "sentAt": sent, "comp0": cur, "events": t.i("eventSeq"), "session": t.s("session"), "tip": tip, "worktree": t.s("worktree") });
            let _ = update_wait(id, 10000, |c| { let was = c.at("closeout").at("was").clone(); let mut r = r.clone(); if was.is_string() { r["was"] = was; } c.put("closeout", r); Ok(Change::Write) }); // the line before the first hold stays
            Gate::Wait("Closeout sent: waiting for Agent Memory to accept it".into())
        }
        Err(e) => held(format!("it could not be typed in ({})", e.message)),
    }
}
/// A closeout not through yet: the task's line says so (what it said before is kept in `closeout.was`), and a held one
/// leaves the space as it is, out of the housekeeping pass's retries (`tidy` cleared), with the reason in `kept`.
pub fn hold(id: &str, same: &dyn Fn(&Task) -> bool, g: &Gate) {
    let (Gate::Wait(line) | Gate::Held(line)) = g else { return };
    let _ = update_wait(id, 10000, |c| {
        if !same(c) { return Ok(Change::Keep); }
        let was = if c.at("closeout").has("was") { c.at("closeout").t("was") } else { c.t("msg") };
        c.obj_mut("closeout").put("was", was.clone());
        c.put("msg", format!("{}{}{line}.", was, if was.is_empty() { "" } else if was.ends_with('.') { " " } else { ". " }));
        if matches!(g, Gate::Held(_)) { c.put("tidy", Value::Null); c.put("kept", json!([line])); }
        Ok(Change::Write)
    });
}
/// Through: the task's line is what it was before the closeout, for the clean-up to add to.
pub fn through(id: &str) -> Option<Task> {
    update_wait(id, 10000, |c| { if !c.at("closeout").has("was") { return Ok(Change::Keep); } let was = c.at("closeout").t("was"); c.put("msg", was); c.obj_mut("closeout").remove("was"); Ok(Change::Write) }).ok()
}
