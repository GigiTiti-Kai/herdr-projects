//! [real programs] The whole check on GitHub as its script runs it (`scripts/check-cloud.sh`), with a stand-in `gh`
//! that answers as GitHub's did and a real origin to push to: only a success of the run it fixed, on the commit still
//! checked out, passes; a failed step comes back with its job, step, log and address; everything else is GitHub giving
//! no answer, on its one line; and run by anyone but the board it is the whole check here, never a push.
use std::path::{Path, PathBuf};
use std::process::Command;
use sw::check_progress::CI_DOWN_LINE;

/// A repo with the script on a branch, a bare origin that `https://github.com/me/proj` leads to, and a `gh` that writes
/// down what it was asked and answers from files: `list`, `view`, `steps`, `jobs`, `log`.
struct Bench { base: PathBuf, sha: String }
impl Bench {
    fn new(name: &str) -> Bench {
        let base = std::env::temp_dir().join(format!("sw-check-cloud-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        for d in ["proj/scripts", "home", "bin", "gh", "origin.git"] { std::fs::create_dir_all(base.join(d)).unwrap(); }
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/check-cloud.sh"), base.join("proj/scripts/check-cloud.sh")).unwrap();
        std::fs::write(base.join("proj/scripts/check.sh"), "echo \"the whole check here: $*\"\n").unwrap();
        std::fs::write(base.join("proj/.gitignore"), "progress\n").unwrap();
        let mut b = Bench { base, sha: String::new() };
        b.stub("gh", &format!("d='{}'\n[ -f \"$d/first\" ] || cp \"$HERDR_PROJECTS_PROGRESS\" \"$d/first\"\necho \"$*\" >> \"$d/calls\"\ncase \"$*\" in\n  *--log-failed*) cat \"$d/log\" ;;\n  *select*) cat \"$d/steps\" 2>/dev/null ;;\n  *'join(\", \")'*) cat \"$d/jobs\" 2>/dev/null ;;\n  'run view'*) cat \"$d/view\" ;;\n  'run list'*) cat \"$d/list\" 2>/dev/null ;;\nesac\nexit 0", b.base.join("gh").display()));
        Self::git_in(&b.base.join("origin.git"), &["init", "-q", "--bare"]);
        b.git(&["init", "-q", "-b", "work/1-thing"]);
        b.git(&["remote", "add", "origin", "https://github.com/me/proj.git"]);
        b.git(&["config", &format!("url.{}.insteadOf", b.base.join("origin.git").display()), "https://github.com/me/proj.git"]);
        b.git(&["add", "-A"]); b.git(&["commit", "-qm", "first"]);
        b.sha = b.git(&["rev-parse", "HEAD"]);
        b
    }
    fn stub(&self, name: &str, body: &str) {
        let p = self.base.join("bin").join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    }
    fn git_in(dir: &Path, a: &[&str]) -> String {
        let o = Command::new("git").args(a).current_dir(dir).env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@t").env("GIT_COMMITTER_NAME", "t").env("GIT_COMMITTER_EMAIL", "t@t").output().unwrap();
        assert!(o.status.success(), "git {a:?}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }
    fn git(&self, a: &[&str]) -> String { Self::git_in(&self.base.join("proj"), a) }
    /// What `gh` answers with, `{sha}` standing for the commit checked out.
    fn says(&self, file: &str, text: &str) { std::fs::write(self.base.join("gh").join(file), text.replace("{sha}", &self.sha)).unwrap(); }
    fn calls(&self) -> String { std::fs::read_to_string(self.base.join("gh/calls")).unwrap_or_default() }
    fn progress(&self) -> String { std::fs::read_to_string(self.base.join("proj/progress")).unwrap_or_default() }
    /// What origin holds for the branch, empty where nothing was pushed.
    fn pushed(&self) -> String { Command::new("git").args(["rev-parse", "-q", "--verify", "refs/heads/work/1-thing"]).current_dir(self.base.join("origin.git")).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default() }
    /// The script as the board starts it (with its progress file), or not: its exit code and the last line it said.
    fn run(&self, board: bool, env: &[(&str, &str)]) -> (i32, String, String) {
        let mut c = Command::new("/bin/sh");
        c.arg("scripts/check-cloud.sh").current_dir(self.base.join("proj")).env_clear().env("HOME", self.base.join("home"))
            .env("PATH", format!("{}:/usr/bin:/bin", self.base.join("bin").display())).env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_NOSYSTEM", "1")
            .env("CHECK_CLOUD_POLL", "0").envs(env.iter().copied());
        if board { c.env("HERDR_PROJECTS_PROGRESS", self.base.join("proj/progress")); }
        let o = c.output().unwrap();
        let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
        let last = out.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("").to_string();
        (o.status.code().unwrap_or(-1), last, out)
    }
}
impl Drop for Bench { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.base); } }

const URL: &str = "https://github.com/me/proj/actions/runs/101";

#[test]
fn real_programs_only_a_success_of_the_run_it_fixed_on_the_commit_still_checked_out_passes() {
    let b = Bench::new("pass");
    b.says("list", "101|1|Check\n");
    b.says("view", &format!("completed|success|{{sha}}|1|push|Check|{URL}\n"));
    let (code, last, out) = b.run(true, &[]);
    assert_eq!((code, last.as_str()), (0, format!("The check passed on GitHub: {URL}").as_str()), "{out}");
    assert_eq!(b.pushed(), b.sha, "the branch was pushed first");
    let calls = b.calls();
    assert!(calls.contains(&format!("run list -R me/proj --workflow ci.yml --commit {} --branch work/1-thing --event push", b.sha)), "{calls}");
    assert!(calls.contains("run view 101 -R me/proj"), "{calls}");
    assert_eq!(b.progress().trim(), r#"{"state":"running","ci":"GitHub run 101"}"#);
    let first = std::fs::read_to_string(b.base.join("gh/first")).unwrap_or_default();
    assert!(first.contains(r#""ci""#), "it says it waits for CI before its first call to gh, which may hang: {first:?}");
    // The same run, on another commit, another attempt or another event, or with the branch moved on: no answer.
    for view in ["completed|success|0000000|1|push|Check", "completed|success|{sha}|2|push|Check", "completed|success|{sha}|1|pull_request|Check"] {
        b.says("view", &format!("{view}|{URL}\n"));
        let (code, last, out) = b.run(true, &[]);
        assert_eq!(code, 3, "{view}: {out}");
        assert!(last.starts_with(&format!("{CI_DOWN_LINE}: the run is no longer the one started for")) && last.ends_with(URL), "{last}");
    }
}

#[test]
fn real_programs_a_failed_step_comes_back_with_its_job_step_log_and_address() {
    let b = Bench::new("fail");
    b.says("list", "101|1|Check\n");
    b.says("view", &format!("completed|failure|{{sha}}|1|push|Check|{URL}\n"));
    b.says("steps", "check / The made tables are committed\n");
    b.says("log", "check\tThe made tables are committed\t\u{feff}2026-11-13T02:00:00.1234567Z These files in git are out of date in the commit:\ncheck\tThe made tables are committed\t2026-11-13T02:00:00.2234567Z   docs/keys.md\n");
    let (code, last, out) = b.run(true, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("Failed on GitHub (job / step):\n  check / The made tables are committed\n"), "{out}");
    assert!(out.contains("The end of its log:\nThese files in git are out of date in the commit:\n  docs/keys.md\n"), "each line without its job, step and time: {out}");
    assert_eq!(last, format!("The check failed on GitHub: check / The made tables are committed · {URL}"));
    assert!(!out.contains(CI_DOWN_LINE));
}

#[test]
fn real_programs_a_run_that_ended_with_no_step_failed_never_started_or_never_ended_is_no_answer() {
    let b = Bench::new("down");
    b.says("list", "101|1|Check\n");
    // A step of GitHub's own failed (an outage at the job's start): not the check's, whatever the run's conclusion says.
    b.says("view", &format!("completed|failure|{{sha}}|1|push|Check|{URL}\n"));
    b.says("steps", "check / Set up job\n");
    let (code, last, out) = b.run(true, &[]);
    assert_eq!((code, last.as_str()), (3, format!("{CI_DOWN_LINE}: a step of GitHub's own failed, not the check: check / Set up job · {URL}").as_str()), "{out}");
    std::fs::remove_file(b.base.join("gh/steps")).unwrap();
    // As GitHub had it on 2026-11-02: the run failed, its macOS job cancelled before any step ran.
    b.says("view", &format!("completed|failure|{{sha}}|1|push|Check|{URL}\n"));
    b.says("jobs", "macos cancelled, check success\n");
    let (code, last, out) = b.run(true, &[]);
    assert_eq!((code, last.as_str()), (3, format!("{CI_DOWN_LINE}: the run ended failure (macos cancelled, check success), with no step of its own failed · {URL}").as_str()), "{out}");
    assert!(out.contains("herdr-projects retry <task>") && out.contains("gh run rerun 101 -R me/proj") && out.contains("CHECK_LOCAL=1"), "it says how to go on: {out}");
    b.says("view", &format!("completed|cancelled|{{sha}}|1|push|Check|{URL}\n"));
    assert_eq!(b.run(true, &[]).0, 3);
    // Still running when the time the board allows is nearly out (here: 2 seconds, so it ends after one).
    b.says("view", &format!("in_progress||{{sha}}|1|push|Check|{URL}\n"));
    let (code, last, _) = b.run(true, &[("HERDR_PROJECTS_CHECK_TIMEOUT", "2")]);
    assert_eq!(code, 3);
    assert!(last.starts_with(&format!("{CI_DOWN_LINE}: the run did not finish within the check's time (check.timeout)")), "{last}");
    // Its waits never go past its end: GitHub looked at every 15 seconds, and 2 allowed.
    let at = std::time::Instant::now();
    assert_eq!(b.run(true, &[("HERDR_PROJECTS_CHECK_TIMEOUT", "2"), ("CHECK_CLOUD_POLL", "15")]).0, 3);
    assert!(at.elapsed() < std::time::Duration::from_secs(10), "it waited {:?}", at.elapsed());
    // No run ever started for the commit.
    std::fs::remove_file(b.base.join("gh/list")).unwrap();
    let (code, last, _) = b.run(true, &[("HERDR_PROJECTS_CHECK_TIMEOUT", "2")]);
    assert_eq!(code, 3);
    assert_eq!(last, format!("{CI_DOWN_LINE}: no run of ci.yml started on GitHub for {} on work/1-thing", b.sha));
}

#[test]
fn real_programs_run_by_anyone_but_the_board_or_told_to_it_is_the_whole_check_here_and_nothing_is_pushed() {
    let b = Bench::new("local");
    assert_eq!(b.run(false, &[]).1, "the whole check here: --all");
    assert_eq!(b.run(true, &[("CHECK_LOCAL", "1")]).1, "the whole check here: --all");
    assert_eq!(b.calls(), "");
    assert_eq!(b.pushed(), "");
    // The board's own run checks the commit: a change that is not in it is refused, with nothing pushed.
    std::fs::write(b.base.join("proj/new.txt"), "x\n").unwrap();
    let (code, _, out) = b.run(true, &[]);
    assert_eq!(code, 2);
    assert!(out.contains("commit them first") && out.contains("?? new.txt"), "{out}");
    assert_eq!(b.pushed(), "");
}
