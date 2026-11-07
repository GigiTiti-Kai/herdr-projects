//! The record-only closeout before a done task's space is closed (`workspace.closeout`, off unless you turn it on):
//! the board types a `/session-close` that ships, pushes, merges and reviews nothing into the working agent, as a turn
//! of its own that the finish line never takes for one (the task stays done: hook.rs acts on no turn of a done task),
//! and lets the clean-up go on only once Agent Memory's queue holds a job accepted for that same session, from that
//! worktree, at that branch tip, since it was sent (memory_queue.rs). Its record is the task's `closeout`: `phase`
//! (`checkpointing` once sent, `saved` once accepted), `sentAt`, `session`, `tip`, `job`, `comp`. A record for another
//! session or tip, or a turn of the agent after the save, is no proof: it is looked at again, and sent again. What
//! cannot be told (herdr did not answer, a dialog, no accepted job before `workspace.closeoutWait`) closes nothing and
//! types nothing more: the space is kept, with the reason. Only the worker's own pane: no reviewer, watcher or planner.
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
pub fn on(t: &Task) -> bool { setting("workspace.closeout", Some(t)) == json!(true) }

pub fn gate(t: &Task) -> Gate {
    if !on(t) || !t.at("link").b("pane") { return Gate::Go; }
    let id = t.s("id");
    let pane = t.at("link").s("pane");
    let a = match h(t).agent_get(pane) {
        Ok(Some(a)) => a,
        Ok(None) => return Gate::Go, // no agent in its pane any more: nothing to close out, and nothing is typed
        Err(e) => return Gate::Wait(format!("Closeout not confirmed yet: herdr did not answer for its agent ({})", e.code)),
    };
    let tip = if t.b("branch") { git_out(&folder_of(id), &["rev-parse", "--verify", &format!("refs/heads/{}", t.s("branch"))]).unwrap_or_default() } else { String::new() };
    let rec = t.at("closeout");
    let same = rec.is_object() && rec.s("session") == t.s("session") && rec.s("tip") == tip && rec.s("worktree") == t.s("worktree");
    let status = a.s("agent_status");
    if same && rec.s("phase") == "saved" && *rec.at("comp") == completion(&a) { return Gate::Go; }
    if same && rec.s("phase") == "checkpointing" {
        if status == "working" { return Gate::Wait("Closeout under way: waiting for its record".into()); }
        let late = now() - rec.i("sentAt") > ms(setting("workspace.closeoutWait", Some(t)).as_str().unwrap_or("10m")).max(1);
        let why = match jobs_of(t.s("session")) {
            Ok(jobs) => match accepted(&jobs, t.s("worktree"), rec.i("sentAt")) {
                Some(j) => {
                    let (job, comp) = (j.id, completion(&a));
                    let _ = update_wait(id, 10000, |c| { c.obj_mut("closeout").put("phase", "saved"); c.obj_mut("closeout").put("job", job.clone()); c.obj_mut("closeout").put("comp", comp.clone()); Ok(Change::Write) });
                    return Gate::Go;
                }
                None => "Agent Memory has no accepted job for its session".to_string(),
            },
            Err(e) => e,
        };
        return if late { Gate::Held(format!("closeout 未確認: {why}. Nothing more was typed; archive the task to close its space")) } else { Gate::Wait(format!("Closeout sent: waiting for Agent Memory to accept it ({why} yet)")) };
    }
    // Nothing sent for this session and tip yet (or a turn since the save): typed only into an agent plainly at rest.
    if status == "working" { return Gate::Wait("Closeout waits for the agent's turn to end".into()); }
    if status != "idle" && status != "done" { return Gate::Held(format!("closeout 未確認: its agent is {} (a dialog, or herdr cannot place it), so nothing was typed", if status.is_empty() { "unknown" } else { status })); }
    if t.s("session").is_empty() { return Gate::Held("closeout 未確認: its agent's session is not known, so its record could not be proved".into()); }
    if !h(t).pane_childless(pane, a.s("agent")) { return Gate::Wait("Closeout waits: a command still runs under its agent".into()); }
    let text = prompt(t.s("agent"));
    match hand(&h(t), pane, &text) {
        Ok(mark) => {
            note_sent(id, mark, &text, false);
            let r = json!({ "phase": "checkpointing", "sentAt": now(), "session": t.s("session"), "tip": tip, "worktree": t.s("worktree") });
            let _ = update_wait(id, 10000, |c| { c.put("closeout", r.clone()); Ok(Change::Write) });
            Gate::Wait("Closeout sent: waiting for Agent Memory to accept it".into())
        }
        Err(e) => Gate::Held(format!("closeout 未確認: it could not be typed in ({})", e.message)),
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
