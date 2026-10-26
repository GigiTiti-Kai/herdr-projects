//! Room in memory before an agent's program starts: under `board.memoryFree` GB available (core memory.rs), a task
//! about to start waits in the queue with a plain line instead of starting, and a reviewer, an agent started again or
//! one taking over waits where it is; each starts by itself once there is room. Nothing that already runs is stopped.
use crate::clock::now;
use crate::disk::{gb, GB};
use crate::err::Res;
use crate::json::*;
use crate::runner::Guard;
use crate::store::settings::lookup::setting;
use crate::store::tasks::Task;

/// The memory a new program leaves free: `board.memoryFree` GB (0: none is looked at).
pub fn needed() -> u64 { setting("board.memoryFree", None).as_i64().unwrap_or(3).max(0) as u64 * GB }
/// Less memory available than a start needs: how much there is, and how much it needs.
#[derive(Clone, Debug)]
pub struct Low { pub free: u64, pub need: u64 }
impl Low {
    /// The task's line while it waits.
    pub fn line(&self) -> String {
        format!("Memory nearly full on {}: {} free < {} (board.memoryFree); it starts by itself once there is room. Nothing that runs is stopped.", crate::remote::check::hostname(), gb(self.free), gb(self.need))
    }
}
/// Too little memory for a new program, None while there is room (or it is not looked at, or cannot be read).
pub fn low() -> Option<Low> {
    let need = needed();
    if need == 0 { return None; }
    let free = crate::memory::available()?;
    (free < need).then_some(Low { free, need })
}
/// A queued task that waits for memory: still too little (the queue leaves it where it is, and starts no runner for it).
pub fn still_waiting(t: &Task) -> bool { t.b("memWait") && low().is_some() }
/// Put back in the queue to wait for memory, with its line.
pub fn wait(t: &mut Obj, l: &Low) { t.put("stage", "queued"); t.put("msg", l.line()); t.put("memWait", json!({ "at": now() })); }
/// A program about to start in a runner (a reviewer, an agent started again or taking over): it waits here until there
/// is room, its line on the task meanwhile, then the task says what it said before. A stop ends the wait.
pub fn hold(g: &mut Guard) -> Res<()> {
    let Some(mut l) = low() else { return Ok(()) };
    let before = g.check()?.t("msg");
    loop {
        let line = l.line();
        if g.check()?.s("msg") != line { g.set(|t| t.put("msg", line))?; }
        crate::clock::pause(crate::finish::shared::ms("HERDR_PROJECTS_POLL_MS", 3000) as u64);
        match low() { Some(n) => l = n, None => break }
    }
    g.set(|t| t.put("msg", before))?;
    Ok(())
}
