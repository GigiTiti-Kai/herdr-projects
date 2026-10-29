//! [real programs] What the shorter check of a fix round runs (`scripts/changed-tests.sh`, for `check.sh --since`):
//! the test files a change since a commit reaches, by their names or by naming its module, or `all` where it cannot
//! tell: a file every test shares, one no test names, one that is not code or a page, a commit the branch lost.
use std::path::{Path, PathBuf};

/// A small repo laid out as this one is, with the script, and a first commit to pick from.
struct Repo { dir: PathBuf, first: String }
impl Repo {
    fn new() -> Repo {
        let dir = std::env::temp_dir().join(format!("sw-changed-tests-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut r = Repo { dir, first: String::new() };
        for f in ["tests/main.rs", "tests/sync.rs", "tests/sync_conflict.rs", "tests/board_keys.rs", "tests/web_board.rs", "tests/guard.rs", "tests/common/mod.rs",
            "tests/far/mod.rs", "src/lib.rs", "crates/flow/src/lib.rs", "crates/flow/src/sync/conflict.rs", "crates/flow/src/finish/wrapup.rs", "crates/flow/src/board/keys.rs", "crates/board/src/board/view/panel/mod.rs", "docs/keys.md", "Cargo.toml"] { r.write(f, "x\n"); }
        r.write("tests/web_board.rs", "use crate::far;\nuse sw::finish::wrapup;\n");
        r.write("tests/guard.rs", "// reads docs/keys.md\n");
        std::fs::create_dir_all(r.dir.join("scripts")).unwrap();
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/changed-tests.sh"), r.dir.join("scripts/changed-tests.sh")).unwrap();
        r.git(&["init", "-q"]); r.git(&["add", "-A"]); r.git(&["commit", "-qm", "first"]);
        r.first = r.git(&["rev-parse", "HEAD"]);
        r
    }
    fn write(&self, f: &str, text: &str) { let p = self.dir.join(f); std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, text).unwrap(); }
    fn git(&self, a: &[&str]) -> String {
        let o = std::process::Command::new("git").args(a).current_dir(&self.dir).env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@t").env("GIT_COMMITTER_NAME", "t").env("GIT_COMMITTER_EMAIL", "t@t").output().unwrap();
        assert!(o.status.success(), "git {a:?}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }
    /// The pick from `since` for the folder as it stands, as one line, and what it said on stderr.
    fn pick(&self, since: &str) -> (String, String) {
        let o = std::process::Command::new("/bin/sh").args(["scripts/changed-tests.sh", since]).current_dir(&self.dir).output().unwrap();
        assert!(o.status.success());
        (String::from_utf8_lossy(&o.stdout).split_whitespace().collect::<Vec<_>>().join(" "), String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
    /// Each change alone, from the first commit: a fresh start for the next.
    fn alone(&self, f: &str) -> (String, String) { self.write(f, "changed\n"); let p = self.pick(&self.first); self.git(&["reset", "-q", "--hard", &self.first]); self.git(&["clean", "-qfd"]); p }
}
impl Drop for Repo { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.dir); } }

#[test]
fn real_programs_the_shorter_check_picks_the_test_files_a_change_reaches_and_all_where_it_cannot_tell() {
    let _env = crate::common::env_kept(); // no world moves HOME or PATH under the programs it starts
    let r = Repo::new();
    assert_eq!(r.pick(&r.first).0, "", "nothing changed: no test file");
    // by the file's name and its folder's; by its module's path; a test file itself, new or changed; a shared module by its users
    assert_eq!(r.alone("crates/flow/src/sync/conflict.rs").0, "sync sync_conflict");
    assert_eq!(r.alone("crates/flow/src/board/keys.rs").0, "board_keys web_board");
    assert_eq!(r.alone("crates/flow/src/finish/wrapup.rs").0, "web_board");
    assert_eq!(r.alone("tests/board_keys.rs").0, "board_keys");
    assert_eq!(r.alone("tests/new_one.rs").0, "new_one");
    assert_eq!(r.alone("tests/far/mod.rs").0, "web_board");
    assert_eq!(r.alone("docs/keys.md").0, "guard");
    // committed or not, all of it since the commit counts
    r.write("crates/flow/src/board/keys.rs", "a\n"); r.git(&["commit", "-qam", "keys"]); r.write("tests/sync.rs", "b\n");
    assert_eq!(r.pick(&r.first).0, "board_keys sync web_board");
    r.git(&["reset", "-q", "--hard", &r.first]);
    // what it cannot tell: all, and why
    for (f, why) in [("tests/common/mod.rs", "tests/common/mod.rs is shared by every test."), ("src/lib.rs", "src/lib.rs holds the whole program."),
        ("crates/flow/src/lib.rs", "crates/flow/src/lib.rs is the root of a crate (docs/crates.md): it names all of that crate's code."),
        ("crates/board/src/board/view/panel/mod.rs", "no test file is named by crates/board/src/board/view/panel/mod.rs or writes its path."), ("Cargo.toml", "Cargo.toml is not a test, the program or a page: what it reaches is not known.")] {
        assert_eq!(r.alone(f), ("all".to_string(), format!("Every test runs: {why}")), "{f}");
    }
    assert_eq!(r.pick("").0, "all");
    assert_eq!(r.pick("nope").0, "all");
    r.git(&["commit", "-q", "--allow-empty", "-m", "lost"]);
    let lost = r.git(&["rev-parse", "HEAD"]);
    r.git(&["reset", "-q", "--hard", &r.first]);
    assert_eq!(r.pick(&lost), ("all".to_string(), format!("Every test runs: the branch no longer holds {lost}.")));
}
