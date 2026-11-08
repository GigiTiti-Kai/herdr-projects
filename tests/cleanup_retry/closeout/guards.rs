//! What the closeout never takes for leave to close: a decision taken back or only quoted, another terminal or session
//! in the pane, words of yours waiting unsent in the input box, and new input after `/exit` went in.
use super::*;

#[test]
fn a_retracted_or_quoted_exit_decision_another_terminal_or_session_or_a_draft_type_nothing() {
    let w = world();
    on();
    for (n, reply) in [(20, "保存受付済。safe to Exit は撤回する。終了は待って。"), (21, "Only say `safe to Exit` after the shared watcher is stopped. Do not exit now.")] {
        let t = merged_with(&w, n, reply, |_| {});
        accepted_now(&t);
        pass(&w, t.s("id"));
        held(&task(t.s("id")), "its reply gives no Exit decision");
        assert_eq!(exits(&w, &t), 0);
    }
    // its pane id kept, but another terminal in it, or another session: nothing typed into it
    let r = merged_with(&w, 22, SAFE, |t| pane(&w, t, |p| p["terminal_id"] = json!("term-other")));
    assert_eq!(closeouts(&w, &r), 0);
    held(&task(r.s("id")), "another terminal than its agent's is in its pane");
    let o = merged_with(&w, 26, SAFE, |t| pane(&w, t, |p| p["agent"]["session"] = json!("someone-else")));
    assert_eq!(closeouts(&w, &o), 0);
    held(&task(o.s("id")), "another session than its own is in its pane");
    // the session herdr shows changes after it was sent: no /exit on the old one's word
    let s = merged(&w, 23);
    pane(&w, &s, |p| p["agent"]["session"] = json!("a-new-session"));
    accepted_now(&s);
    assert_eq!(pass(&w, s.s("id")), 1);
    held(&task(s.s("id")), "another session than its own is in its pane");
    assert_eq!(exits(&w, &s), 0);
    // words of yours waiting unsent: never sent along with the closeout
    let d = merged_with(&w, 24, SAFE, |t| pane(&w, t, |p| { p["agent"]["box"] = json!("UNFINISHED DRAFT: do not submit"); p["agent"]["screen"] = json!("Done.\n╭──\n│ ❯ UNFINISHED DRAFT: do not submit │\n╰──"); }));
    assert_eq!(closeouts(&w, &d), 0);
    assert!(!prompts(&w, d.at("link").s("pane")).iter().any(|p| p.contains("UNFINISHED DRAFT")));
    held(&task(d.s("id")), "words wait unsent in its input box");
    set_fake(None);
}

#[test]
fn new_input_after_exit_went_in_keeps_the_space() {
    let w = world();
    on();
    let t = merged(&w, 25);
    accepted_now(&t);
    assert_eq!(pass(&w, t.s("id")), 1);
    assert_eq!(task(t.s("id")).at("closeout").s("phase"), "closing"); assert_eq!(exits(&w, &t), 1);
    yours(&w, &t);
    settle(t.s("id"));
    pane(&w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; });
    pass(&w, t.s("id"));
    held(&task(t.s("id")), "new input reached its agent after /exit was typed");
    set_fake(None);
}
