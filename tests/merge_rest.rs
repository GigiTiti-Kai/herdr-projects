//! Merging, the rest of the old suite's checks: a merge conflict with your checkout waits for you as a question of
//! permission, not as a review (the rest of that test is lifecycle.rs). And the GitHub issues a merge with no pull
//! request closes (`merge.closes`): closed with gh once it has landed, never on a merge that failed, and a close that
//! fails leaves the merge done with the reason on its line. And `approve --expect-review N --expect-head SHA`
//! (auto-ship): it merges only while the task is on that review and commit (as `details` and `list --json` give them),
//! merges nothing when either moved, never goes with --anyway, and `approve` without them is as it was.
use crate::common;
use common::pilot::{cli, ok, refused};
use common::*;
use serde_json::{json, Value};
use sw::json::*;

#[test]
fn a_merge_conflict_with_your_checkout_waits_for_you_and_is_not_shown_as_a_review() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "a.txt": "from the agent\n" }, "commit": "agent", "note": "Changed a.txt." }] }));
    let id = w.add("Conflict");
    sw::store::settings::change::set_value("review.rounds", &json!(0), &sw::store::settings::lookup::Scope::task(&task(&id))).unwrap();
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "approval");
    std::fs::write(w.root.join("a.txt"), "from the human\n").unwrap(); git(&w.root, &["commit", "-qam", "human"]);
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.at("mergeFailed")), ("approval", &json!(true)));
    let r = row(&id);
    assert_eq!((r.s("state"), r.s("kind"), r.at("asReview")), ("blocked", "permission", &json!(false)));
}

/// A task whose work closes `issues`, through its one commit to waiting for approval, in a repo with no remote.
fn closing(w: &World, issues: &str) -> String {
    w.script(json!({ "worker": [step(1)] }));
    let id = sw::actions::filing::add(&w.root, "Fix it", sw::actions::filing::AddOpts { settings: vec![("merge.closes".into(), json!(issues))], ..Default::default() }).unwrap().t("id");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "approval");
    id
}
fn plain_world() -> World { world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() }) }

#[test]
fn a_merge_with_no_pull_request_closes_its_issues_with_gh_once_it_has_landed() {
    let w = plain_world();
    let id = closing(&w, "5, 6");
    assert!(!gh_log(&w).contains("issue close"), "nothing closed before the merge: {}", gh_log(&w));
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    let made = git(&w.root, &["rev-parse", "HEAD"]);
    assert_eq!(t.s("mergeCommit"), made);
    let comment = format!("--comment Closed by task {} \"Fix it\": its work landed in {}.", sw::store::keys::task_ref(&id, &sw::store::keys::repo_keys()), &made[..12]);
    for n in ["5", "6"] { assert!(gh_log(&w).contains(&format!("issue close {n} {comment}")), "{}", gh_log(&w)); }
    assert!(t.s("msg").starts_with("Merged into main. Closed #5, #6."), "{}", t.s("msg"));
}

#[test]
fn a_close_that_fails_leaves_the_merge_done_and_says_why_on_its_line() {
    let w = plain_world();
    let id = closing(&w, "5");
    w.gh_says(json!({ "issueFail": "GraphQL: Could not resolve to an issue or pull request with the number of 5." }));
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(read(&w.root.join("f.txt")), "v1\n");
    assert!(t.s("msg").contains("Could not close #5: GraphQL: Could not resolve to an issue or pull request with the number of 5."), "{}", t.s("msg"));
}

#[test]
fn nothing_is_closed_on_a_merge_that_failed() {
    let w = plain_world();
    let id = closing(&w, "5");
    std::fs::write(w.root.join("mine.txt"), "unsaved\n").unwrap();
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.at("failure").s("title"), "Merge needs a clean checkout");
    assert!(!gh_log(&w).contains("issue close"), "{}", gh_log(&w));
}

fn work_on(file: &str) -> Value { json!({ "files": { file: "1\n" }, "commit": "v1", "note": "Did v1." }) }
fn pass() -> Value { json!({ "verdict": { "meets": true, "text": "Looked at it.", "findings": [] } }) }
/// A task, delegated and reviewed, that waits for approval: its round and reviewed commit, as `details` says them.
fn waiting(w: &World, title: &str, file: &str) -> (String, i64, String) {
    w.script(json!({ "worker": [work_on(file)], "reviewer": [pass()] }));
    let id = w.add(title);
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    let d = cli(&["details", &id, "--json"]).unwrap().0;
    let (round, head) = (d.at("task").i("reviewRound"), d.at("task").t("reviewedHead"));
    assert!(round >= 1 && head.len() == 40, "{}", d.at("task"));
    let row = cli(&["list", "--json"]).unwrap().0.as_array().unwrap().iter().find(|r| r.s("id") == id).cloned().unwrap();
    assert_eq!((row.i("reviewRound"), row.t("reviewedHead")), (round, head.clone()), "list --json says the same");
    (id, round, head)
}

#[test]
fn approve_held_to_a_review_and_a_commit_merges_only_when_both_still_stand() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let (id, round, head) = waiting(&w, "One", "f.txt");
    let (r, n) = (round.to_string(), (round + 1).to_string());
    // the review ran again since it was judged
    sw::store::tasks::set(&id, json!({ "reviewRound": round + 1 })).unwrap();
    assert!(refused(&["approve", &id, "--expect-review", &r, "--expect-head", &head]).contains(&format!("its review is round {n} now, not {r}")));
    sw::store::tasks::set(&id, json!({ "reviewRound": round })).unwrap();
    // the commit moved since
    assert!(refused(&["approve", &id, "--expect-review", &r, "--expect-head", &"0".repeat(40)]).contains(&format!("its reviewed commit is {head} now")));
    assert!(refused(&["approve", &id, "--expect-head", &head[..6]]).contains("not merged"), "fewer than 7 characters say nothing");
    // never with --anyway, never without a value, never a word for a round
    assert!(refused(&["approve", &id, "--expect-review", &r, "--anyway"]).contains("do not go with --anyway"));
    assert!(refused(&["approve", &id, "--expect-head"]).contains("--expect-head needs a value"));
    assert!(refused(&["approve", &id, "--expect-review", "two"]).contains("a number"));
    assert_eq!(settle(&id).s("stage"), "approval", "nothing was merged");
    assert!(!w.root.join("f.txt").exists());
    // both still stand: merged
    ok(&["approve", &id, "--expect-review", &r, "--expect-head", &head[..12]]);
    assert_eq!(settle(&id).s("stage"), "done");
    assert!(w.root.join("f.txt").exists());
    // without them, approve is as it was
    let (two, _, _) = waiting(&w, "Two", "g.txt");
    ok(&["approve", &two]);
    assert_eq!(settle(&two).s("stage"), "done");
    assert!(w.root.join("g.txt").exists());
}

#[test]
fn approve_held_to_a_review_refuses_a_task_that_does_not_wait_to_merge() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let id = w.add("Asks");
    sw::store::tasks::set(&id, json!({ "stage": "asks", "kind": "permission" })).unwrap();
    assert!(refused(&["approve", &id, "--expect-review", "1"]).contains("not waiting for an approval to merge"));
}
