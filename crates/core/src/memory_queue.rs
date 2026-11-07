//! Agent Memory's save queue, read only: the jobs a session-close filed for one agent session (`jobs` in
//! `$XDG_STATE_HOME/agent-memory/queue.sqlite3`, read with the `sqlite3` program), so that a record-only closeout can
//! be proved accepted before a task's space is closed (land/closeout.rs). Nothing is ever written to it. A test answers
//! for the queue itself (`set_fake`), so no program is started then.
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;

/// One job of the queue: its id, its status, when it was accepted (ms since the epoch) and the folder it was filed from.
#[derive(Clone, Debug, PartialEq)]
pub struct Job { pub id: String, pub status: String, pub accepted: i64, pub cwd: String }
type Fake = Rc<dyn Fn(&str) -> Result<Vec<Job>, String>>;
thread_local! { static FAKE: RefCell<Option<Fake>> = const { RefCell::new(None) }; }
/// A stand-in for the queue: it gets the session id, and answers its jobs or why they cannot be read.
pub fn set_fake(f: Option<Fake>) { FAKE.with(|x| *x.borrow_mut() = f); }

pub fn file() -> PathBuf {
    let state = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).filter(|p| p.is_absolute())
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"));
    state.join("agent-memory/queue.sqlite3")
}
/// The session's jobs, or why they cannot be read (no queue, no `sqlite3`, an id that is no plain id): never "none".
pub fn jobs_of(session: &str) -> Result<Vec<Job>, String> {
    if session.is_empty() || !session.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') { return Err(format!("the session id {session:?} cannot be looked up")); }
    if let Some(f) = FAKE.with(|x| x.borrow().clone()) { return f(session); }
    jobs_in(&file(), session)
}
fn jobs_in(db: &std::path::Path, session: &str) -> Result<Vec<Job>, String> {
    if !db.is_file() { return Err(format!("no Agent Memory queue at {}", db.display())); }
    let q = format!("select job_id, status, accepted_at, cwd from jobs where session_id = '{session}' order by queue_seq;");
    let out = Command::new("sqlite3").args(["-readonly", "-separator", "\t"]).arg(db).arg(q).stdin(Stdio::null()).output().map_err(|e| format!("sqlite3 could not be run: {e}"))?;
    if !out.status.success() { return Err(format!("sqlite3: {}", String::from_utf8_lossy(&out.stderr).trim())); }
    Ok(String::from_utf8_lossy(&out.stdout).lines().filter_map(parse).collect())
}
fn parse(line: &str) -> Option<Job> {
    let f: Vec<&str> = line.split('\t').collect();
    let at = chrono::DateTime::parse_from_rfc3339(f.get(2)?).ok()?.timestamp_millis();
    Some(Job { id: f[0].into(), status: f[1].into(), accepted: at, cwd: f.get(3)?.to_string() })
}
/// The job that proves a closeout sent at `sent` (ms) was accepted: the same session (asked), from `cwd`, accepted at
/// or after `sent`, and not given up on. The newest such.
pub fn accepted(jobs: &[Job], cwd: &str, sent: i64) -> Option<Job> {
    jobs.iter().rev().find(|j| j.accepted >= sent && j.cwd.trim_end_matches('/') == cwd.trim_end_matches('/') && !["discarded", "superseded", "corrupt_acknowledged"].contains(&j.status.as_str())).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_job_counts_only_from_the_same_folder_after_the_closeout_and_not_given_up_on() {
        let j = |id: &str, st: &str, at: i64, cwd: &str| Job { id: id.into(), status: st.into(), accepted: at, cwd: cwd.into() };
        let jobs = vec![j("old", "complete", 5, "/w"), j("else", "queued", 20, "/other"), j("gone", "discarded", 30, "/w")];
        assert_eq!(accepted(&jobs, "/w", 10), None);
        let more = [jobs, vec![j("new", "queued", 40, "/w/")]].concat();
        assert_eq!(accepted(&more, "/w", 10).map(|j| j.id), Some("new".into()));
        assert_eq!(parse("a\tqueued\t2026-11-07T20:40:19.402681+09:00\t/w").map(|j| j.accepted), Some(1794051619402));
        assert!(parse("a\tqueued\tyesterday\t/w").is_none());
        assert!(jobs_of("x'; drop table jobs; --").is_err());
    }
    #[test]
    fn real_programs_sqlite3_reads_the_session_s_jobs_from_the_queue() {
        if Command::new("sqlite3").arg("-version").output().is_err() { return; }
        let dir = std::env::temp_dir().join(format!("memory-queue-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("queue.sqlite3");
        let _ = std::fs::remove_file(&db);
        let made = Command::new("sqlite3").arg(&db).arg("create table jobs (queue_seq integer primary key, job_id text, session_id text, status text, accepted_at text, cwd text); insert into jobs (job_id, session_id, status, accepted_at, cwd) values ('j1', 's-1', 'queued', '2026-11-07T20:40:19+09:00', '/w'), ('j2', 's-2', 'queued', '2026-11-07T20:40:19+09:00', '/w');").status().unwrap();
        assert!(made.success());
        assert_eq!(jobs_in(&db, "s-1").unwrap(), vec![Job { id: "j1".into(), status: "queued".into(), accepted: 1794051619000, cwd: "/w".into() }]);
        assert!(jobs_in(&dir.join("none.sqlite3"), "s-1").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
