//! Answering Codex's hooks review on a seat's pane: judged first (mod.rs), "Trust all and continue" pressed only when
//! every hook passed, and every judgement written to `hooks-trust.log` in the board's data folder. The screen is read
//! again before each key: the line's number is typed only on that dialog, and enter pressed only with that line
//! chosen, so "Review hooks" is never opened and no other dialog gets a key.
use super::{allowed, decide, dialog, list};
use crate::agents::homes::codex_file;
use crate::herdr::{api::read_screen, Herdr};
use crate::store::files::{data_dir, user_home};
use std::path::{Path, PathBuf};

pub enum Answer { NotIt, Trusted, Refused(String) }

fn sleep(ms: u64) { std::thread::sleep(std::time::Duration::from_millis(ms)); }
fn screen(api: &Herdr, pane: &str) -> String { read_screen(api, pane, Some("blocked"), 40) }
/// What the screen shows now, read again a few times over two seconds: a terminal draws what a key did a moment later.
fn soon(mut ok: impl FnMut() -> bool) -> bool { (0..10).any(|i| { if i > 0 { sleep(200); } ok() }) }

/// One look at a pane's dialog at a time: a second (two runs of the board seeing the same seat) waits for nothing and
/// does nothing. A mark older than two minutes is from a look that died.
struct Lock(PathBuf);
impl Lock {
    fn take(pane: &str) -> Option<Lock> {
        let dir = data_dir().join("hooks-trust");
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join(pane.replace(['/', '\\'], "_"));
        let old = std::fs::metadata(&f).and_then(|m| m.modified()).ok().and_then(|m| m.elapsed().ok()).is_some_and(|e| e.as_secs() > 120);
        if old { let _ = std::fs::remove_file(&f); }
        std::fs::OpenOptions::new().write(true).create_new(true).open(&f).ok().map(|_| Lock(f))
    }
}
impl Drop for Lock { fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); } }

/// The judgement, as one line of the log.
pub fn log(who: &str, pane: &str, cwd: &Path, what: &str) {
    let line = format!("{} {who} pane {pane} in {}: {what}\n", crate::clock::iso(crate::clock::now()), cwd.display());
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(data_dir().join("hooks-trust.log")).and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
}

/// Codex's hooks review on the pane, answered when every hook it asks about is the human's own and clean. `who` names
/// the seat in the log (`<task> reviewer`).
pub fn answer(api: &Herdr, pane: &str, cwd: &Path, who: &str) -> Answer {
    let Some(n) = dialog::count(&screen(api, pane)) else { return Answer::NotIt };
    let Some(_lock) = Lock::take(pane) else { return Answer::NotIt };
    match list::hooks(cwd).and_then(|hs| decide(n, &hs, &allowed(), &codex_file(), &user_home(), cwd)) {
        Err(why) => { log(who, pane, cwd, &format!("not trusted: {why}")); Answer::Refused(why) }
        Ok(found) => match press(api, pane) {
            Ok(()) => { log(who, pane, cwd, &format!("trusted {n}: {}", found.join(" | "))); Answer::Trusted }
            Err(why) => { log(who, pane, cwd, &format!("not answered ({why}), though each passed: {}", found.join(" | "))); Answer::Refused(why) }
        },
    }
}

fn press(api: &Herdr, pane: &str) -> Result<(), String> {
    let s = screen(api, pane);
    if dialog::count(&s).is_none() { return Err("the dialog went away before it was answered".into()); }
    let (number, chosen) = dialog::trust_line(&s).ok_or("its \"Trust all and continue\" line is not on the screen")?;
    if !chosen {
        api.pane_text(pane, &number).map_err(|e| e.message)?;
        if !soon(|| { let s = screen(api, pane); dialog::count(&s).is_some() && dialog::trust_line(&s).is_some_and(|(_, c)| c) }) { return Err(format!("\"Trust all and continue\" was not chosen after {number} was typed, so nothing was confirmed")); }
    }
    api.agent_keys(pane, &["enter"]).map_err(|e| e.message)?;
    if !soon(|| dialog::count(&screen(api, pane)).is_none()) { return Err("enter was pressed on \"Trust all and continue\", and the dialog is still there".into()); }
    Ok(())
}
