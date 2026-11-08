//! Every caller that would type into a parked task's agent or start it again: an approval needs no agent; a reply, a
//! review sent back or a fix, and a restart, resume its own conversation or say why not (never a fresh one); the board's
//! own turns and a conflict type nothing; and a crash or herdr not answering is never taken for a park.
use super::*;

fn since(w: &World, n: usize) -> Vec<String> { calls(w)[n..].to_vec() }
fn started(c: &[String]) -> bool { c.iter().any(|c| c.starts_with("agent start")) }
fn typed(c: &[String]) -> bool { c.iter().any(|c| c.starts_with("agent prompt") || c.starts_with("pane send")) }

#[test]
fn an_approval_merges_with_no_agent_started_and_the_clean_up_types_no_second_closeout() {
    let w = world();
    on();
    sw::store::settings::change::set_value("workspace.closeout", &json!(true), &sw::store::settings::lookup::Scope::you()).unwrap();
    let t = parked(&w, 10, false);
    let n = calls(&w).len();
    sw::actions::finish_line::approve(t.s("id"), None, false).unwrap();
    let c = settle(t.s("id"));
    assert_eq!(c.s("stage"), "done", "{}", c.at("failure"));
    let after = since(&w, n);
    assert!(!started(&after) && !typed(&after), "{after:?}");
    sw::land::cleanup_job(t.s("id"), "merged");
    let c = task(t.s("id"));
    assert!(c.b("cleaned"), "{}", c.s("msg"));
    assert_eq!(c.at("closeout").s("phase"), "pane_closed");
    assert!(!typed(&since(&w, n)));
    set_fake(None);
}

#[test]
fn a_reply_resumes_its_own_conversation_with_the_words_and_never_a_fresh_one() {
    let w = world();
    on();
    let t = parked(&w, 11, true);
    let id = t.t("id");
    let n = calls(&w).len();
    w.script(json!({ "worker": [{ "said": "Removed it." }] }));
    sw::actions::talk::tell(&id, "Yes, remove it.", false).unwrap();
    let c = settle(&id);
    let after = since(&w, n);
    let starts: Vec<&String> = after.iter().filter(|c| c.starts_with("agent start")).collect();
    assert_eq!(starts.len(), 1, "{after:?}");
    assert!(starts[0].contains(&format!("--resume {}", t.s("session"))), "{}", starts[0]);
    assert!(!after.iter().any(|c| c.contains("Work board task")), "no first prompt: {after:?}");
    assert_eq!(said(&w, &c, "Yes, remove it."), 1);
    assert!(!c.at("closeout").is_object() && c.s("session") == t.s("session"));
    set_fake(None);
}

#[test]
fn a_conversation_that_cannot_be_resumed_starts_nothing_and_says_why_with_the_words_kept() {
    let w = world();
    on();
    let t = parked(&w, 12, false);
    let id = t.t("id");
    // its saved conversation is gone (the program cannot resume it)
    let _ = std::fs::remove_dir_all(std::path::Path::new(&std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join("projects"));
    let n = calls(&w).len();
    sw::actions::talk::tell(&id, "One more change.", false).unwrap();
    let c = settle(&id);
    assert!(!started(&since(&w, n)), "{:?}", since(&w, n));
    assert!(c.at("failure").s("detail").contains("cannot be resumed") || c.s("msg").contains("cannot be resumed"), "{} / {}", c.s("msg"), c.at("failure"));
    assert!(sw::store::inbox::pending_text(&id).iter().any(|m| m.s("text") == "One more change."), "the words still wait");
    assert!(sw::land::park::parked(&c), "the park stays until you resume it or forget it");
    // reopening does not start it either; forgetting it is yours
    let (_, has, _) = sw::delegate::reopen(&id, true).unwrap();
    assert!(!has && !started(&since(&w, n)));
    sw::land::park::forget(&id).unwrap();
    assert!(!task(&id).at("closeout").is_object());
    set_fake(None);
}

#[test]
fn while_it_is_being_parked_a_reply_an_approval_and_the_boards_own_turns_type_nothing() {
    let w = world();
    on();
    let t = waiting(&w, 13, false, SAFE);
    let id = t.t("id");
    sw::land::park::park(&id).unwrap();
    let n = calls(&w).len();
    for e in [sw::actions::talk::tell(&id, "wait", false).map(|_| ()), sw::actions::finish_line::approve(&id, None, false).map(|_| ()), sw::flow::send::send_own(&id, "fix the check", "check", false, Obj::new(), 0).map(|_| ())] {
        assert!(e.unwrap_err().msg.contains("being parked"));
    }
    assert!(!typed(&since(&w, n)) && !started(&since(&w, n)));
    assert_eq!(task(&id).s("stage"), "approval");
    set_fake(None);
}

#[test]
fn a_parked_task_gets_no_board_turn_and_a_conflict_on_its_approval_asks_you() {
    let w = world();
    on();
    let t = parked(&w, 14, false);
    let id = t.t("id");
    let n = calls(&w).len();
    assert!(sw::flow::send::send_own(&id, "fix the check", "check", false, Obj::new(), 0).unwrap_err().msg.contains("is parked"));
    // main moved over the same file: the approval's merge finds a conflict, and with no agent to send it to, you are asked
    std::fs::write(w.root.join("p14.txt"), "from main\n").unwrap(); git(&w.root, &["add", "-A"]); git(&w.root, &["commit", "-qm", "main: p14"]);
    let _ = sw::actions::finish_line::approve(&id, None, false);
    let c = settle(&id);
    let after = since(&w, n);
    assert!(!started(&after) && !typed(&after), "{after:?}");
    assert_eq!((c.s("stage"), c.at("failure").s("title"), c.at("failure").b("sentAt")), ("approval", "Merge conflict with main", false), "{}", c.s("msg"));
    set_fake(None);
}

#[test]
fn a_crash_before_its_exit_holds_it_and_herdr_not_answering_only_waits() {
    let w = world();
    on();
    // herdr does not answer: it waits, nothing is held or taken for gone
    let t = waiting(&w, 15, false, SAFE);
    let id = t.t("id");
    sw::land::park::park(&id).unwrap();
    w.patch(|s| s["agentGetFails"] = json!(true));
    pass(&w, &id);
    let c = task(&id);
    assert!(sw::land::park::busy(&c) && c.s("msg").contains("herdr did not answer"), "{}", c.s("msg"));
    w.patch(|s| s["agentGetFails"] = json!(false));
    // its agent gone before it was told to exit: a crash, held, never parked, and an ordinary task again
    pane(&w, &t, |p| { p["agent"] = Value::Null; });
    pass(&w, &id);
    let c = task(&id);
    assert!(c.at("closeout").b("held") && !sw::land::park::parked(&c) && !sw::land::park::parking(&c), "{:?}", c.at("closeout"));
    assert!(c.s("msg").contains("left its pane before it was told to exit"), "{}", c.s("msg"));
    assert_eq!((c.s("stage"), exits(&w, &t)), ("approval", 0));
    let n = calls(&w).len();
    for _ in 0..2 { assert_eq!(pass(&w, &id), 0); }
    assert!(!started(&since(&w, n)), "a crash is not brought back by itself");
    set_fake(None);
}
