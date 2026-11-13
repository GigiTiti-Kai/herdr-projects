//! The whole check on GitHub as the board takes it (docs/check-on-github.md): a check that waits for CI says so and
//! which run, apart from the wait for its turn here; it is handed `check.timeout`; and a run GitHub gave no answer on
//! waits for you instead of going back to the agent, while a check that failed still goes back, whatever its log says.
use crate::common;
use common::*;
use serde_json::json;
use sw::check_progress::{check_shown, read_report, start_check, step_check, CI_DOWN_LINE};
use sw::json::*;

const MIN: i64 = 60000;
const T0: i64 = 1_000_000_000_000;

#[test]
fn a_check_waiting_for_ci_says_so_and_which_run_never_as_the_wait_for_its_turn() {
    assert_eq!(read_report(r#"{"state":"running","ci":"GitHub run 123"}"#).unwrap(), json!({ "state": "running", "ci": "GitHub run 123" }));
    assert_eq!(read_report(r#"{"ci":"run <b>9</b>"}"#).unwrap().s("ci"), "run b9b", "only plain words are kept");
    assert!(read_report(r#"{"state":"waiting","ci":"GitHub run 1"}"#).unwrap().at("ci").is_null(), "the wait for a turn here stays itself");
    let c = step_check(&start_check(T0, 4 * MIN), read_report(r#"{"state":"running","ci":"pushing the branch"}"#).as_ref(), T0);
    let c = step_check(&c, read_report(r#"{"state":"running","ci":"GitHub run 123"}"#).as_ref(), T0 + MIN);
    assert_eq!(c.i("movedAt"), T0 + MIN, "a new word from CI is a move");
    let s = check_shown(&c, T0 + 2 * MIN);
    assert_eq!((s.msg.as_str(), s.progress, s.eta), ("Waiting for CI · GitHub run 123 · 2m of about 4m", None, Some(2)));
    assert_eq!(check_shown(&c, T0 + 20 * MIN).msg, "Waiting for CI far longer than usual · GitHub run 123 · 20m, usually 4m");
    let waiting = step_check(&start_check(T0, 0), read_report(r#"{"state":"waiting"}"#).as_ref(), T0);
    assert_eq!(check_shown(&waiting, T0).msg, "Waiting for its turn");
}

#[test]
fn a_run_github_gave_no_answer_on_waits_for_you_and_is_not_one_of_the_agents_rounds() {
    let saw = std::env::temp_dir().join(format!("sw-ci-timeout-{}", std::process::id()));
    let cmd = format!("printf %s \"$HERDR_PROJECTS_CHECK_TIMEOUT\" > '{}'; echo 'Checking on GitHub'; echo '{CI_DOWN_LINE}: the run ended cancelled · https://github.com/me/proj/actions/runs/7'; exit 3", saw.display());
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": cmd, "check.timeout": "2m" })), ..Default::default() });
    w.script(json!({ "worker": [{ "note": "Done." }, { "note": "Done again." }] }));
    let id = w.add("Checked on GitHub");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(std::fs::read_to_string(&saw).unwrap_or_default(), "120", "the check is handed check.timeout, in seconds");
    let _ = std::fs::remove_file(&saw);
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(t.s("msg"), format!("{CI_DOWN_LINE}: the run ended cancelled · https://github.com/me/proj/actions/runs/7"));
    let f = t.at("failure");
    assert_eq!((f.s("title"), f.b("agentCanFix")), (CI_DOWN_LINE, false));
    assert_eq!(t.i("checkFails"), 0);
    assert_eq!(prompts(&w, t.at("link").s("pane")).len(), 1, "nothing was sent back to the agent");
    assert_eq!(row(&id).s("state"), "error");
}

#[test]
fn a_check_the_board_stops_at_its_time_while_it_waits_for_ci_waits_for_you() {
    let cmd = r#"printf '{"state":"running","ci":"GitHub run 9"}' > "$HERDR_PROJECTS_PROGRESS"; sleep 30"#;
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": cmd, "check.timeout": "1s" })), ..Default::default() });
    w.script(json!({ "worker": [{ "note": "Done." }, { "note": "Done again." }] }));
    let id = w.add("Out of time on GitHub");
    delegate(&id);
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("failure").b("agentCanFix"), t.i("checkFails")), ("failed", false, 0));
    assert_eq!(t.s("msg"), format!("{CI_DOWN_LINE}: the board stopped it at check.timeout while it waited for CI (GitHub run 9)"));
    assert_eq!(prompts(&w, t.at("link").s("pane")).len(), 1, "nothing was sent back to the agent");
}

#[test]
fn a_check_that_failed_on_github_still_goes_back_to_the_agent_even_when_its_log_holds_the_words_of_no_answer() {
    let cmd = format!("echo 'Failed on GitHub (job / step):'; echo '  check / The check'; echo 'assertion failed: \"{CI_DOWN_LINE}: …\"'; echo 'The check failed on GitHub: check / The check · https://github.com/me/proj/actions/runs/8'; exit 1");
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": cmd })), ..Default::default() });
    w.script(json!({ "worker": [{ "note": "Done." }, { "note": "Done again." }] }));
    let id = w.add("Fails on GitHub");
    delegate(&id);
    let t = settle(&id);
    assert!(t.i("checkFails") > 1, "it went back, round after round");
    let p = prompts(&w, t.at("link").s("pane"));
    assert!(p.len() >= 2 && p[1].contains("The check failed, so this is not done yet") && p[1].contains("check / The check"), "{p:?}");
}
