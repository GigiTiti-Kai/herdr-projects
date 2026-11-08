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
use crate::herdr::api::read_screen;
use crate::herdr::Herdr;
use crate::json::*;
use crate::memory_queue::{accepted, jobs_of};
use crate::store::repos::{cwd_of, folder_of, git_out};
use crate::store::settings::lookup::setting;
use crate::store::settings::values::ms;
use crate::store::tasks::{update_wait, Change, Task};
use std::path::{Path, PathBuf};

/// Go: clean up (or stop). Wait: look again on the next pass. Held: keep it as it is, say why, and stop trying.
#[derive(Debug, PartialEq)]
pub enum Gate { Go, Wait(String), Held(String) }
pub const WORDS: &str = "記録だけの session-close。ship・push・merge・PR・再 review・削除はしない。自分が所有し現在の権限で止められる背景処理（監視・cron・接続）だけ止めてから報告。他者・共有・所有不明のものは止めず、巻き込む可能性があれば残して理由付きで保留";
/// What is typed: the skill's own command for the agent's program (Codex names skills with `$`).
pub fn prompt(agent: &str) -> String { format!("{} {WORDS}{ASK}", if agent.starts_with("codex") { "$session-close" } else { "/session-close" }) }
/// The Exit decision asked for in a form the board can read without guessing: a last line of its own.
pub const ASK: &str = "。終了してよいと判断した時だけ、返事の最後の行に `safe to Exit` とだけ書く（それ以外は書かない）";
pub const EXIT: &str = "/exit";
/// The session-close skill's own words for its Exit decision.
pub const SAFE: &str = "safe to Exit";
pub fn on(t: Option<&Task>) -> bool { setting("workspace.closeout", t) == json!(true) }
pub fn held(why: impl std::fmt::Display) -> Gate { Gate::Held(format!("closeout 未確認: {why}. Nothing more was typed")) }

/// The agent a closeout is for, as its owner knows it. `terminal`: the terminal its agent was started in ("" when not
/// known: then the one herdr shows at the send is the one every later look must see). `events`: the turn ends its hook
/// filed (0 without one).
pub struct Who { pub api: Herdr, pub pane: String, pub terminal: String, pub session: String, pub transcript: String, pub agent: String, pub cwd: PathBuf, pub tip: String, pub events: i64, pub wait_ms: i64 }
/// One look. `rec`: the record so far; `save(patch, whole)`: merge into it, or (`whole`) start it again from the patch,
/// false when it could not be written. What is typed is written down as on its way first (`sending`, `exiting`): a
/// record left so (it went in, or not; the record after could not be written) is never sent again.
pub fn step(w: &Who, rec: &Value, save: &dyn Fn(Value, bool) -> bool) -> Gate {
    let phase = rec.s("phase");
    if phase == "sending" || phase == "exiting" { return held(format!("whether {} went in is not known (its record could not be written after it)", if phase == "sending" { "the closeout" } else { "/exit" })); }
    if phase.is_empty() && !rec.has("waitFrom") { save(json!({ "waitFrom": now() }), false); }
    let since = if phase == "closing" { rec.i("exitAt") } else if !phase.is_empty() { rec.i("sentAt") } else if rec.has("waitFrom") { rec.i("waitFrom") } else { now() };
    let late = now() - since > w.wait_ms.max(1);
    let wait = |why: &str| if late { held(why) } else { Gate::Wait(format!("Closeout not through yet: {why}")) };
    let (api, pane) = (&w.api, w.pane.as_str());
    let a = match api.agent_get(pane) {
        Err(e) => return wait(&format!("herdr did not answer for its agent ({})", e.code)), // an error is not a pane gone
        Ok(None) if phase == "closing" || phase == "agent_exited" => {
            // The pane is still the one its agent was in: its terminal as at the send.
            if api.pane_get(pane).map(|p| p.t("terminal_id")) != Some(rec.t("terminal")) || rec.t("terminal").is_empty() { return held("another terminal than its agent's is in its pane, or herdr does not say which"); }
            if !api.pids_gone(pane, &rec.arr("pids").iter().map(|p| p.as_i64().unwrap_or(0)).collect::<Vec<_>>()) { return wait("its agent's process has not ended"); }
            if !api.pane_idle(pane) { return wait("its agent left, but something still runs at its shell"); }
            if let Some(g) = still_saved(w, rec, &wait) { return g; } // nothing new since /exit, and the save still there
            if phase == "closing" { save(json!({ "phase": "agent_exited", "exitedAt": now() }), false); }
            return Gate::Go;
        }
        Ok(None) if phase.is_empty() => return held("no agent in its pane to save a record, so none can be proved"),
        Ok(None) => return held("its agent left its pane before it was told to exit"),
        Ok(Some(a)) => a,
    };
    if phase == "agent_exited" { return held("an agent is in its pane again"); }
    // The agent herdr shows there is the one this is for: its session, and its terminal.
    let (live_session, live_term) = (a.at("agent_session").t("value"), a.t("terminal_id"));
    if live_session != w.session { return held(if live_session.is_empty() { "herdr does not say which session is in its pane".to_string() } else { "another session than its own is in its pane".to_string() }); }
    let term = if phase.is_empty() { w.terminal.clone() } else { rec.t("terminal") };
    if live_term.is_empty() || (!term.is_empty() && live_term != term) { return held("another terminal than its agent's is in its pane, or herdr does not say which"); }
    let status = a.s("agent_status");
    if phase == "closing" { if let Some(g) = still_saved(w, rec, &wait) { return g; } return wait(&format!("its agent was told to exit and is still there ({status})")); }
    let busy = || if status == "working" { Some(wait("its agent's turn is under way")) } else if status != "idle" && status != "done" { Some(held(format!("its agent is {} (a dialog, or herdr cannot place it)", if status.is_empty() { "unknown" } else { status }))) } else { None };
    // Words of yours waiting unsent in its input box would go in with whatever is typed: nothing is typed then.
    let draft = || match draft(&read_screen(api, pane, Some(status), 40)) { Some(d) if d.is_empty() => None, Some(d) => Some(held(format!("words wait unsent in its input box ({})", crate::store::tasks::one_line(&d, 60)))), None => Some(held("its input box cannot be read, so it cannot be told empty")) };
    if phase.is_empty() {
        // Nothing sent yet: typed only into an agent plainly at rest, whose saved conversation can be read.
        if let Some(g) = busy() { return g; }
        if w.session.is_empty() { return held("its agent's session is not known, so its record could not be proved"); }
        if !api.pane_childless(pane, a.s("agent")) { return wait("a command still runs under its agent"); }
        if let Some(g) = draft() { return g; }
        let text = prompt(&w.agent);
        let sent = now(); // taken before it goes in
        if !save(json!({ "phase": "sending", "sentAt": sent, "events": w.events, "session": w.session, "terminal": live_term, "tip": w.tip, "cwd": w.cwd.display().to_string() }), true) { return wait("its record could not be written, so nothing was typed"); }
        return match type_in(w, &live_term, pane, &text) {
            Ok(_) => { save(json!({ "phase": "checkpointing" }), false); Gate::Wait("Closeout sent: waiting for Agent Memory to accept it".into()) }
            Err(e) => { save(json!({ "phase": "", "failed": e.clone() }), false); held(format!("nothing was typed: {e}")) }
        };
    }
    // New input since it was sent: what any save proves is out of date, and nothing more is typed.
    if rec.s("session") != w.session { return held("its agent's session changed since it was sent"); }
    if rec.s("tip") != w.tip || rec.s("cwd") != w.cwd.display().to_string() { return held("its branch moved since it was sent"); }
    if w.events - rec.i("events") > 1 { return held("new input reached its agent after it was sent"); }
    if let Some(g) = busy() { return g; }
    // Its own last reply: the closeout's (begun by its words after it was sent, the same one each look), its Exit decision.
    let Some(turn) = last_turn(w) else { return wait("its saved conversation cannot be read") };
    if !turn.finished || turn.pending || turn.began < rec.i("sentAt") { return wait("its reply to the closeout has not ended yet"); }
    if !turn.asked.contains(WORDS) { return held("new input reached its agent after it was sent (its last turn answers something else)"); }
    if rec.has("turn") && rec.s("turn") != turn.id { return held("new input reached its agent after it was sent"); }
    if !rec.has("turn") { save(json!({ "turn": turn.id }), false); }
    if !safe(&turn.said) { return held(format!("its reply gives no Exit decision (a last line `{SAFE}`): {}", crate::store::tasks::one_line(&turn.said, 160))); }
    let job = match accepted_job(w, rec) { Ok(j) => j, Err(why) => return wait(&why) };
    if phase == "checkpointing" { save(json!({ "phase": "saved", "job": job, "savedAt": now() }), false); }
    // The exit, decided now: at rest, nothing under it, nothing unsent, its processes known (to see them end).
    if !api.pane_childless(pane, a.s("agent")) { return wait("a command still runs under its agent"); }
    if let Some(g) = draft() { return g; }
    let pids = match api.front_pids(pane) { Some(p) if !p.is_empty() => p, _ => return wait("herdr does not say which process is its agent") };
    if !save(json!({ "phase": "exiting", "exitAt": now(), "job": job, "pids": pids }), false) { return wait("its record could not be written, so /exit was not typed"); }
    if let Err(e) = type_in(w, &rec.t("terminal"), pane, EXIT) { save(json!({ "phase": "saved" }), false); return held(format!("/exit was not typed: {e}")); }
    save(json!({ "phase": "closing" }), false);
    Gate::Wait("Closeout saved: its agent was told to exit".into())
}
fn last_turn(w: &Who) -> Option<crate::agents::read_turn::Turn> { read_turn_of(&w.agent, (!w.transcript.is_empty()).then(|| Path::new(&w.transcript)), &w.session, &w.cwd, false) }
fn accepted_job(w: &Who, rec: &Value) -> Result<String, String> {
    let jobs = jobs_of(&w.session)?;
    accepted(&jobs, &w.cwd.display().to_string(), rec.i("sentAt")).map(|j| j.id).ok_or_else(|| "Agent Memory has no accepted job for its session".to_string())
}
/// After `/exit` went in: no turn since the closeout's but the exit itself, no turn end beyond, and its save still
/// accepted. None: so; else what to do.
fn still_saved(w: &Who, rec: &Value, wait: &dyn Fn(&str) -> Gate) -> Option<Gate> {
    if w.events - rec.i("events") > 1 { return Some(held("new input reached its agent after /exit was typed")); }
    let Some(turn) = last_turn(w) else { return Some(wait("its saved conversation cannot be read")) };
    if turn.id != rec.s("turn") && !(is_exit(&turn.asked) && turn.began >= rec.i("exitAt")) { return Some(held("new input reached its agent after /exit was typed")); }
    accepted_job(w, rec).err().map(|why| wait(&why))
}
/// The prompt that began a turn is `/exit` itself (as typed, or as Claude Code keeps a command it ran), not words
/// that only name it.
fn is_exit(asked: &str) -> bool { let a = asked.trim(); a == EXIT || a.starts_with("<command-name>/exit</command-name>") }
/// The last look right before anything is typed, on the same answer of herdr's the text then goes in on: the same
/// session and terminal, at rest, nothing waiting unsent in its input box. Err: why nothing was typed.
fn type_in(w: &Who, term: &str, pane: &str, text: &str) -> Result<(), String> {
    let a = match w.api.agent_get(pane) { Ok(Some(a)) => a, Ok(None) => return Err("its agent left its pane".into()), Err(e) => return Err(format!("herdr did not answer ({})", e.code)) };
    if a.at("agent_session").t("value") != w.session || a.t("terminal_id") != term || term.is_empty() { return Err("another session or terminal came into its pane right before".into()); }
    if !["idle", "done"].contains(&a.s("agent_status")) { return Err(format!("its agent is {} right before", a.s("agent_status"))); }
    match draft(&read_screen(&w.api, pane, Some(a.s("agent_status")), 40)) { Some(d) if d.is_empty() => {} Some(_) => return Err("words wait unsent in its input box".into()), None => return Err("its input box cannot be read".into()) }
    match w.api.agent_prompt(pane, text) { Ok(_) => Ok(()), Err(e) if ["agent_prompt_stalled", "timeout"].contains(&e.code.as_str()) => Ok(()), Err(e) => Err(e.message) } // these do not prove nothing went in: never typed twice
}
/// Its reply says it may exit: it names the decision once, as its own last line, plain (not quoted, not in a list),
/// and nothing in it takes that back (`unsafe`, `not safe`, an Exit blocked, a retraction).
pub fn safe(said: &str) -> bool {
    let low = said.to_lowercase();
    let last = said.lines().rev().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    let last = last.strip_prefix('`').and_then(|l| l.strip_suffix('`')).unwrap_or(last).trim_end_matches(['.', '。']);
    last == SAFE && said.matches(SAFE).count() == 1 && !["unsafe to exit", "not safe to exit", "never safe to exit", "exit blocked", "exit is blocked", "撤回", "do not exit", "don't exit"].iter().any(|n| low.contains(n))
}
/// What waits in an agent's input box, read from its screen: the text after the last prompt mark (`❯`, `>`, `›`) at a
/// line's start, inside its frame, and on every line below it down to the frame's end or a rule (`╰`, `└`, `─`).
/// Below an unframed box (Codex), a line is its footer only as the program writes it (`a · b`, `? for shortcuts`):
/// any other words count as yours. Some(""): empty, or the program's own placeholder. None: no input box seen.
/// shortcut: only Codex's usual placeholder is known as one; another placeholder holds the closeout (it waits for you).
pub fn draft(screen: &str) -> Option<String> {
    let lines: Vec<&str> = screen.lines().collect();
    let strip = |l: &str| l.trim().trim_start_matches('│').trim_end_matches('│').trim().to_string();
    let at = lines.iter().rposition(|l| ["❯", ">", "›"].iter().any(|m| strip(l).starts_with(m)))?;
    let first = strip(lines[at]).trim_start_matches(['❯', '>', '›']).trim().to_string();
    let mut words = vec![if ["Ask Codex to do anything"].contains(&first.as_str()) { String::new() } else { first }];
    for l in &lines[at + 1..] {
        let t = l.trim();
        if t.starts_with(['╰', '└', '─', '━']) { break; }
        let framed = t.starts_with('│');
        let x = strip(l);
        if !framed && (x.contains(" · ") || x.contains("? for shortcuts")) { continue; }
        words.push(x);
    }
    Some(words.into_iter().filter(|w| !w.is_empty()).collect::<Vec<_>>().join("\n"))
}
/// Has the pane gone: herdr says there is no such pane. Some(false): it is there; None: herdr did not say.
pub fn pane_gone(api: &Herdr, pane: &str) -> Option<bool> {
    match api.call(crate::herdr::cli::args(&["pane", "get", pane])) { Ok(_) => Some(false), Err(e) if e.code.contains("not_found") => Some(true), Err(_) => None }
}
fn wait_ms(t: Option<&Task>) -> i64 { ms(setting("workspace.closeoutWait", t).as_str().unwrap_or("10m")) }

/// A done task's worker, before its space is closed.
pub fn gate(t: &Task) -> Gate {
    if !on(Some(t)) || !t.at("link").b("pane") { return Gate::Go; }
    let id = t.s("id");
    let tip = if t.b("branch") { git_out(&folder_of(id), &["rev-parse", "--verify", &format!("refs/heads/{}", t.s("branch"))]).unwrap_or_default() } else { String::new() };
    let w = Who { api: h(t), pane: t.at("link").t("pane"), terminal: t.at("link").t("terminal"), session: t.t("session"), transcript: t.t("transcript"), agent: t.t("agent"), cwd: cwd_of(t), tip, events: t.i("eventSeq"), wait_ms: wait_ms(Some(t)) };
    let save = |patch: Value, whole: bool| update_wait(id, 10000, |c| {
        let was = c.at("closeout").at("was").clone(); // the line before the first hold stays
        if whole { c.put("closeout", patch.clone()); if was.is_string() { c.obj_mut("closeout").put("was", was.clone()); } } else { for (k, v) in patch.as_object().cloned().unwrap_or_default() { c.obj_mut("closeout").insert(k, v); } }
        Ok(Change::Write)
    }).is_ok();
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
