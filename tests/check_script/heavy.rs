//! [real programs] The whole check under the machine's shared lock for heavy jobs (`scripts/heavy.sh`), with a
//! stand-in `heavy-run` that writes down each job it is given and runs it as its unit would: the whole check takes the
//! lock once, also when the caller exported an empty `HEAVY_RUN_ACTIVE`; a run of names, or a check already inside a
//! heavy job, never takes it.
use super::Bench;

/// The stand-in lock, and a `setsid` that only hands on (macOS has none).
fn with_lock(b: &Bench) {
    b.stub("heavy-run", &format!("echo \"$*\" >> '{}'\nwhile [ \"$1\" != -- ]; do shift; done; shift\nHEAVY_RUN_ACTIVE=1 exec \"$@\"", b.base.join("heavy-runs").display()));
    b.stub("setsid", "[ \"$1\" != -w ] || shift\nexec \"$@\"");
}
fn locks(b: &Bench) -> usize { std::fs::read_to_string(b.base.join("heavy-runs")).unwrap_or_default().lines().count() }

#[test]
fn real_programs_the_whole_check_takes_the_shared_lock_once_and_a_run_of_names_or_one_inside_a_heavy_job_never() {
    let _env = crate::common::env_kept();
    let b = Bench::new("heavy");
    with_lock(&b);
    // the whole check: once, and it runs inside (the stand-in cargo stops it at the build, exit 7)
    let (code, err, saw) = b.check(&[]);
    assert_eq!((code, locks(&b)), (7, 1), "{err}");
    assert!(saw.starts_with("build "), "the check did not run inside the lock: {err}");
    // a caller's empty HEAVY_RUN_ACTIVE: still once, never a second wait for its own lock
    let (code, err, saw) = b.check(&[("HEAVY_RUN_ACTIVE", "")]);
    assert_eq!((code, locks(&b)), (7, 2), "{err}");
    assert!(saw.starts_with("build "), "{err}");
    // already inside a heavy job, or a run of names: no lock
    assert_eq!(b.check(&[("HEAVY_RUN_ACTIVE", "1")]).0, 7);
    assert_eq!(b.sh(&["scripts/check.sh", "board-panel"], &[]).0, 7);
    assert_eq!(locks(&b), 2);
}
