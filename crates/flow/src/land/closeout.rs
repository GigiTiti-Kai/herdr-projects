//! The record-only closeout before a worker's space is closed or a coordinator is stopped or begun anew
//! (`workspace.closeout`, off unless you turn it on): the board types a `/session-close` that ships, pushes, merges and
//! reviews nothing into the agent, as a turn of its own that the finish line never takes for one (a done task's turns
//! are not finishes: hook.rs). Each step is proved apart, and the record (`closeout`) moves on only at a proved one
//! (`phase`): `checkpointing` (sent), `saved` (Agent Memory's queue holds a job accepted for that session, from that
//! folder, after it was sent: memory_queue.rs, looked up again on every pass), then the exit decided now (the agent's
//! own last reply, read from its saved conversation, is its reply to the closeout and says `safe to Exit`; it is at
//! rest with nothing under it), `/exit` typed (`closing`), `agent_exited` (herdr sees no agent in the pane, the
//! processes that were the agent have ended, and nothing runs at its shell) and, for a task, `pane_closed`
//! (cleanup.rs). New input (a reply that is not the closeout's, a turn beyond its own, another session, a new branch
//! tip), an agent gone before it was told to exit, a dialog, an Exit blocked, or anything still unproved once
//! `workspace.closeoutWait` has passed since the first look: nothing more is typed and nothing is closed, and the reason
//! is said. Nothing is ever sent twice. Only a worker's own pane and a coordinator's: no reviewer, watcher or planner.
use crate::agents::installed::read_turn_of;
use crate::clock::now;
use crate::flow::task::h;
use crate::herdr::Herdr;
use crate::json::*;
use crate::memory_queue::{accepted, jobs_of};
use crate::store::repos::{cwd_of, folder_of, git_out};
use crate::store::settings::lookup::setting;
use crate::store::settings::values::ms;
use crate::store::tasks::{update_wait, Change, Task};
use crate::submit::hand;
use std::path::{Path, PathBuf};

/// Go: clean up (or stop). Wait: look again on the next pass. Held: keep it as it is, say why, and stop trying.
#[derive(Debug, PartialEq)]
pub enum Gate { Go, Wait(String), Held(String) }
pub const WORDS: &str = "記録だけの session-close。ship・push・merge・PR・再 review・削除はしない。自分が所有し現在の権限で止められる背景処理（監視・cron・接続）だけ止めてから報告。他者・共有・所有不明のものは止めず、巻き込む可能性があれば残して理由付きで保留";
/// What is typed: the skill's own command for the agent's program (Codex names skills with `$`).
pub fn prompt(agent: &str) -> String { format!("{} {WORDS}", if agent.starts_with("codex") { "$session-close" } else { "/session-close" }) }
pub const EXIT: &str = "/exit";
/// The session-close skill's own words for its Exit decision.
pub const SAFE: &str = "safe to Exit";
pub fn on(t: Option<&Task>) -> bool { setting("workspace.closeout", t) == json!(true) }
pub fn held(why: impl std::fmt::Display) -> Gate { Gate::Held(format!("closeout 未確認: {why}. Nothing more was typed")) }

/// The agent a closeout is for, as its owner knows it. `events`: the turn ends its hook filed (0 without one).
pub struct Who { pub api: Herdr, pub pane: String, pub session: String, pub transcript: String, pub agent: String, pub cwd: PathBuf, pub tip: String, pub events: i64, pub wait_ms: i64 }
/// One look. `rec`: the record so far; `save(patch, whole)`: merge into it, or (`whole`) start it again from the patch.
pub fn step(w: &Who, rec: &Value, save: &dyn Fn(Value, bool)) -> Gate {
    let phase = rec.s("phase");
    if phase.is_empty() && !rec.has("waitFrom") { save(json!({ "waitFrom": now() }), false); }
    let since = if phase == "closing" { rec.i("exitAt") } else if !phase.is_empty() { rec.i("sentAt") } else if rec.has("waitFrom") { rec.i("waitFrom") } else { now() };
    let late = now() - since > w.wait_ms.max(1);
    let wait = |why: &str| if late { held(why) } else { Gate::Wait(format!("Closeout not through yet: {why}")) };
    let (api, pane) = (&w.api, w.pane.as_str());
    let a = match api.agent_get(pane) {
        Err(e) => return wait(&format!("herdr did not answer for its agent ({})", e.code)), // an error is not a pane gone
        Ok(None) if phase == "agent_exited" => return Gate::Go,
        Ok(None) if phase == "closing" => {
            if !api.pids_gone(pane, &rec.arr("pids").iter().map(|p| p.as_i64().unwrap_or(0)).collect::<Vec<_>>()) { return wait("its agent's process has not ended"); }
            if !api.pane_idle(pane) { return wait("its agent left, but something still runs at its shell"); }
            save(json!({ "phase": "agent_exited", "exitedAt": now() }), false);
            return Gate::Go;
        }
        Ok(None) if phase.is_empty() => return held("no agent in its pane to save a record, so none can be proved"),
        Ok(None) => return held("its agent left its pane before it was told to exit"),
        Ok(Some(a)) => a,
    };
    let status = a.s("agent_status");
    if phase == "agent_exited" { return held("an agent is in its pane again"); }
    if phase == "closing" { return wait(&format!("its agent was told to exit and is still there ({status})")); }
    let busy = || if status == "working" { Some(wait("its agent's turn is under way")) } else if status != "idle" && status != "done" { Some(held(format!("its agent is {} (a dialog, or herdr cannot place it)", if status.is_empty() { "unknown" } else { status }))) } else { None };
    if phase.is_empty() {
        // Nothing sent yet: typed only into an agent plainly at rest, whose saved conversation can be read.
        if let Some(g) = busy() { return g; }
        if w.session.is_empty() { return held("its agent's session is not known, so its record could not be proved"); }
        if !api.pane_childless(pane, a.s("agent")) { return wait("a command still runs under its agent"); }
        let text = prompt(&w.agent);
        let sent = now(); // taken before it goes in
        return match hand(api, pane, &text) {
            Ok(_) => { save(json!({ "phase": "checkpointing", "sentAt": sent, "events": w.events, "session": w.session, "tip": w.tip, "cwd": w.cwd.display().to_string() }), true); Gate::Wait("Closeout sent: waiting for Agent Memory to accept it".into()) }
            Err(e) => held(format!("it could not be typed in ({})", e.message)),
        };
    }
    // New input since it was sent: what any save proves is out of date, and nothing more is typed.
    if rec.s("session") != w.session { return held("its agent's session changed since it was sent"); }
    if rec.s("tip") != w.tip || rec.s("cwd") != w.cwd.display().to_string() { return held("its branch moved since it was sent"); }
    if w.events - rec.i("events") > 1 { return held("new input reached its agent after it was sent"); }
    if let Some(g) = busy() { return g; }
    // Its own last reply: the closeout's (begun after it was sent, the same one each look), with its Exit decision.
    let file = (!w.transcript.is_empty()).then(|| Path::new(&w.transcript));
    let Some(turn) = read_turn_of(&w.agent, file, &w.session, &w.cwd, false) else { return wait("its saved conversation cannot be read") };
    if !turn.finished || turn.pending || turn.began < rec.i("sentAt") { return wait("its reply to the closeout has not ended yet"); }
    if rec.has("turn") && rec.s("turn") != turn.id { return held("new input reached its agent after it was sent"); }
    if !rec.has("turn") { save(json!({ "turn": turn.id }), false); }
    if !turn.said.contains(SAFE) { return held(format!("its reply gives no Exit decision (`{SAFE}`): {}", crate::store::tasks::one_line(&turn.said, 160))); }
    let job = match jobs_of(&w.session) {
        Ok(jobs) => match accepted(&jobs, &w.cwd.display().to_string(), rec.i("sentAt")) { Some(j) => j.id, None => return wait("Agent Memory has no accepted job for its session") },
        Err(e) => return wait(&e),
    };
    if phase == "checkpointing" { save(json!({ "phase": "saved", "job": job, "savedAt": now() }), false); }
    // The exit, decided now: at rest, nothing under it, its processes known (to see them end).
    if !api.pane_childless(pane, a.s("agent")) { return wait("a command still runs under its agent"); }
    let pids = match api.front_pids(pane) { Some(p) if !p.is_empty() => p, _ => return wait("herdr does not say which process is its agent") };
    if let Err(e) = hand(api, pane, EXIT) { return held(format!("/exit could not be typed in ({})", e.message)); }
    save(json!({ "phase": "closing", "exitAt": now(), "job": job, "pids": pids }), false);
    Gate::Wait("Closeout saved: its agent was told to exit".into())
}
fn wait_ms(t: Option<&Task>) -> i64 { ms(setting("workspace.closeoutWait", t).as_str().unwrap_or("10m")) }

/// A done task's worker, before its space is closed.
pub fn gate(t: &Task) -> Gate {
    if !on(Some(t)) || !t.at("link").b("pane") { return Gate::Go; }
    let id = t.s("id");
    let tip = if t.b("branch") { git_out(&folder_of(id), &["rev-parse", "--verify", &format!("refs/heads/{}", t.s("branch"))]).unwrap_or_default() } else { String::new() };
    let w = Who { api: h(t), pane: t.at("link").t("pane"), session: t.t("session"), transcript: t.t("transcript"), agent: t.t("agent"), cwd: cwd_of(t), tip, events: t.i("eventSeq"), wait_ms: wait_ms(Some(t)) };
    let save = |patch: Value, whole: bool| { let _ = update_wait(id, 10000, |c| {
        let was = c.at("closeout").at("was").clone(); // the line before the first hold stays
        if whole { c.put("closeout", patch.clone()); if was.is_string() { c.obj_mut("closeout").put("was", was.clone()); } } else { for (k, v) in patch.as_object().cloned().unwrap_or_default() { c.obj_mut("closeout").insert(k, v); } }
        Ok(Change::Write)
    }); };
    let g = step(&w, t.at("closeout"), &save);
    // in its chat, as the board's own words
    if matches!(g, Gate::Wait(_)) && t.at("closeout").s("phase").is_empty() && crate::store::tasks::get(id).map(|c| c.at("closeout").s("phase") == "checkpointing").unwrap_or(false) { crate::store::said_by::note_board(id, &prompt(t.s("agent"))); }
    if let Gate::Held(why) = g { return Gate::Held(format!("{why}; archive the task to close its space")); }
    g
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
