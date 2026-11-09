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
/// A park's record of this round: once its space is closed (`pane_closed`) that round is over, and so is the park.
fn ours(t: &Obj) -> bool { rec(t).b("park") && rec(t).s("phase") != "pane_closed" }
pub fn parked(t: &Obj) -> bool { ours(t) && rec(t).b("parked") }
/// A park under way, or done, on a task that still waits for you: its turns are its own.
pub fn parking(t: &Obj) -> bool { ours(t) && !rec(t).b("held") && STAGES.contains(&t.s("stage")) }
/// Under way and not yet through: nothing else is typed into its agent.
pub fn busy(t: &Obj) -> bool { parking(t) && !parked(t) }
/// A park's record that proves no exit (held, or left behind by a task that moved on): it counts for nothing.
pub fn unfinished(t: &Obj) -> bool { rec(t).b("park") && (!rec(t).b("parked") || rec(t).b("held")) }
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
    if crate::plan::planner(&t) { crate::fail!("{id}'s agent is its plan writer: only a worker is parked."); }
    if runner_alive(&t) { crate::fail!("{id} is busy. Try again in a moment."); }
    if !pending_text(&id).is_empty() { crate::fail!("Words wait to go to {id}'s agent: nothing was parked."); }
    if rec(&t).is_object() { crate::fail!("{id} has a closeout record already ({}): nothing was parked.", if parked(&t) { "parked" } else { rec(&t).s("phase") }); }
    if !crate::delegate::launch::resumable(&t) { crate::fail!("{id}'s conversation could not be resumed afterwards, so it is not parked."); }
    update_wait(&id, 10000, |c| { c.put("closeout", json!({ "park": true })); Ok(Change::Write) })?;
    drive(&id);
    let c = get(&id).unwrap_or(t);
    // Held before anything went in: no park at all, its line as it was.
    if rec(&c).b("held") && rec(&c).s("phase").is_empty() { let why = c.t("msg"); let _ = forget(&id); crate::fail!("{id} was not parked, and nothing was typed: {}", why.strip_prefix(rec(&c).s("was")).unwrap_or(&why).trim_start_matches(['.', ' '])); }
    let line = c.t("msg");
    Ok((c, line))
}
/// One look at a park under way (the housekeeping pass, every half minute, and `park` itself).
pub fn drive(id: &str) {
    let Some(t) = get(id) else { return };
    if !busy(&t) { return; }
    // One driver at a time (park itself, the housekeeping pass's runner): a second one finds it claimed and leaves it.
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = crate::clock::now();
    let mine = json!({ "at": now, "by": format!("{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)) });
    let claimed = update_wait(id, 10000, |c| { let at = c.at("parkDriver").i("at"); if !busy(c) || (at > 0 && now - at < 120000) { return Ok(Change::Keep); } c.put("parkDriver", mine.clone()); Ok(Change::Write) });
    if !claimed.map(|c| c.at("parkDriver") == &mine).unwrap_or(false) { return; }
    steps(id, &t);
    let _ = update_wait(id, 10000, |c| { if c.at("parkDriver") != &mine { return Ok(Change::Keep); } c.put("parkDriver", Value::Null); Ok(Change::Write) });
}
fn steps(id: &str, t: &Task) {
    let t = t.clone();
    let g = if rec(&t).b("blocked") {
        // Its agent said it cannot exit: held once its turn is over (so that turn stays the closeout's own).
        match crate::flow::task::h(&t).agent_get(t.at("link").s("pane")) { Ok(Some(a)) if a.s("agent_status") == "working" => Gate::Wait("Closeout not through yet: its agent reported it cannot exit, and its turn is under way".into()), _ => hold_why(format!("its agent reported: {}", rec(&t).s("blocked"))) }
    } else { match who_of(&t) { Some(w) => step(&w, rec(&t), &|p, whole| save_to(id, p, whole)), None => hold_why("its worktree cannot be read, so whether its work moved cannot be told") } };
    if matches!(g, Gate::Wait(_)) && rec(&t).s("phase").is_empty() && get(id).map(|c| c.at("closeout").s("phase") == "checkpointing").unwrap_or(false) { crate::store::said_by::note_board(id, &prompt(t.s("agent"))); }
    // Held for anything but new input: the closeout's own turns stay its own, not turns of yours for the hook to find later.
    let own = match &g { Gate::Held(why) => !["new input", "session changed", "another session", "moved"].iter().any(|n| why.contains(n)), _ => false };
    let seen = if own { t.at("link").b("pane").then(|| crate::flow::task::h(&t).agent_get(t.at("link").s("pane")).ok().flatten()).flatten() } else { None };
    let line = match &g { Gate::Go => "Parked: its agent exited after a record-only closeout; go into the task (or reply) to resume the same conversation".to_string(), Gate::Wait(l) | Gate::Held(l) => l.clone() };
    let _ = update_wait(id, 10000, |c| {
        if !busy(c) { return Ok(Change::Keep); }
        let was = if c.at("closeout").has("was") { c.at("closeout").t("was") } else { c.t("msg") };
        c.obj_mut("closeout").put("was", was.clone());
        match &g { Gate::Go => { c.obj_mut("closeout").put("parked", true); c.obj_mut("closeout").put("parkedAt", crate::clock::now()); } Gate::Held(_) => c.obj_mut("closeout").put("held", true), Gate::Wait(_) => {} }
        if let Some(a) = &seen { let h = c.obj_mut("handled"); h.put("comp", a.at("completion_seq").clone()); h.put("seq", a.at("state_change_seq").clone()); h.put("term", a.at("terminal_id").clone()); h.put("at", crate::clock::now()); }
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
/// Its own agent reports itself blocked during the closeout: the task keeps what it waits on, and the park is held
/// with those words (`drive`). Some: so taken; None: no park under way.
pub fn reported(t: &Obj, words: &str) -> Option<Task> {
    if !busy(t) { return None; }
    let r = update_wait(t.s("id"), 10000, |c| { if !busy(c) { return Ok(Change::Keep); } c.obj_mut("closeout").put("blocked", words); Ok(Change::Write) }).ok();
    drive(t.s("id"));
    r.and_then(|_| get(t.s("id")))
}
/// Before an agent is started on a parked task: only its own conversation again, never a fresh one or another.
pub fn may_start(t: &Obj, resume: Option<&str>) -> Res<()> {
    refuse_busy(t)?; // its closeout under way (or its agent gone mid-way, a crash the next look holds): nothing is started
    if parked(t) && !rec(t).b("held") { refuse_foreign(t)?; }
    if parked(t) && !rec(t).b("held") && (resume != Some(rec(t).s("session")) || !std::path::Path::new(t.s("worktree")).is_dir()) {
        crate::fail!("{} was parked, and {}: nothing was started (a fresh one is never started in its place). Its stage and waiting words are as they were; archive it, or delegate it again after: herdr-projects park {} --forget", t.s("id"), why_not(t, resume.is_some()), t.s("id"));
    }
    Ok(())
}
/// Why a parked task's agent cannot come back. `resumable`: its program still has its conversation.
pub fn why_not(t: &Obj, resumable: bool) -> &'static str {
    if !std::path::Path::new(t.s("worktree")).is_dir() { "its worktree is gone" } else if !resumable { "its conversation cannot be resumed" } else { "another conversation would be started" }
}
/// A reopen that did not bring a parked task's agent back: its line says why, not that it can be resumed.
pub fn not_back(id: &str, resumable: bool) {
    let _ = update_wait(id, 10000, |c| {
        if !parked(c) { return Ok(Change::Keep); }
        let was = c.at("closeout").t("was");
        c.put("msg", format!("{was}{}Parked, and not brought back: {} (nothing was started; herdr-projects park {} --forget to give it up).", if was.is_empty() { "" } else if was.ends_with('.') { " " } else { ". " }, why_not(c, resumable), c.s("id")));
        Ok(Change::Write)
    });
}
/// Before anything is typed into a parked task's pane: an agent there now is its own conversation, or nothing goes in;
/// with no agent, its pane is still the terminal it was parked in, its shell at a bare prompt (nothing typed unsent).
pub fn refuse_foreign(t: &Obj) -> Res<()> {
    if !parked(t) { return Ok(()); }
    let (api, pane) = (crate::flow::task::h(t), t.at("link").s("pane"));
    match api.agent_get(pane) {
        Ok(None) => {
            let term = rec(t).s("terminal");
            if term.is_empty() || api.pane_get(pane).map(|p| p.t("terminal_id")).as_deref() != Some(term) { crate::fail!("{} is parked, and its pane is no longer the terminal it was parked in (or herdr does not say which): nothing was typed.", t.s("id")); }
            if !api.pane_idle(pane) { crate::fail!("{} is parked, and its shell runs something (or herdr does not say what): nothing was typed.", t.s("id")); }
            if !super::closeout::shell_at_prompt(&api.pane_read(pane, "recent-unwrapped", Some(2000)).unwrap_or_default(), crate::agents::kinds::program(t.s("agent"))) { crate::fail!("{} is parked, and its shell may hold words typed and not sent (or its prompt cannot be read): nothing was typed.", t.s("id")); }
            Ok(())
        }
        Ok(Some(a)) if a.at("agent_session").s("value") == rec(t).s("session") && !rec(t).s("session").is_empty() => Ok(()),
        Ok(Some(_)) => crate::fail!("{} is parked, and another agent than its own is in its pane: nothing was sent.", t.s("id")),
        Err(e) => crate::fail!("{} is parked, and herdr did not say which agent is in its pane ({}): nothing was sent.", t.s("id"), e.code),
    }
}
/// A start or restart that finds another agent than a parked task's own in its pane (or herdr not saying which): the
/// task says so and stops, its waiting words kept, and nothing is typed or pressed in that pane. true: so.
pub fn kept_from_foreign(t: &Obj) -> bool {
    let Err(e) = refuse_foreign(t) else { return false };
    let _ = update_wait(t.s("id"), 10000, |c| {
        c.put("stage", "failed"); c.put("msg", e.msg.clone()); c.put("turnOpen", false); c.put("expectedWake", Value::Null);
        c.put("failure", json!({ "stage": "agent", "title": if e.msg.contains("another agent") { "Another agent is in its pane" } else { "Its pane is not as it was parked" }, "detail": e.msg, "agentCanFix": false, "at": crate::clock::now() }));
        Ok(Change::Write)
    });
    true
}
/// Its agent is back in its own conversation: the park is over (a reopen puts its line back as before it: jobs.rs).
pub fn resumed(id: &str) {
    let _ = update_wait(id, 10000, |c| {
        if !parked(c) { return Ok(Change::Keep); }
        c.put("closeout", Value::Null);
        Ok(Change::Write)
    });
}
/// herdr shows its own conversation in its pane again (it came back by itself): a parked task's park is over.
pub fn back(t: &Obj, a: &Value) -> bool { parked(t) && !rec(t).s("session").is_empty() && a.at("agent_session").s("value") == rec(t).s("session") }
/// `park --forget`: you give up on its conversation; the record goes, and the task is an ordinary one again.
pub fn forget(id: &str) -> Res<(Task, String)> {
    let t = find(id)?;
    if !rec(&t).b("park") { crate::fail!("{} has no park to forget.", t.s("id")); }
    refuse_busy(&t)?;
    let id = t.t("id");
    let _ = update_wait(&id, 10000, |c| { if c.at("closeout").has("was") { let was = c.at("closeout").t("was"); c.put("msg", was); } c.put("closeout", Value::Null); Ok(Change::Write) });
    Ok((get(&id).unwrap_or(t), format!("{id}: its park is forgotten")))
}
