//! What the closeout never takes for leave to close: a decision taken back or only quoted, another terminal or session
//! in the pane, words of yours waiting unsent in the input box, and new input after `/exit` went in.
use super::*;
use std::cell::Cell;
use sw::herdr::cli::{Fake, Ran};

/// The world's herdr, but once its agent's screen has been read (the last look before anything is typed), the next
/// look at the agent finds another session in the pane: what changes between the look and the typing.
struct Swap { inner: Rc<common::herdr::FakeHerdr>, read: Cell<bool>, pane: String }
impl Fake for Swap {
    fn run(&self, a: &[String], input: &str) -> Ran {
        let is = |x: &str, y: &str| a.first().map(|s| s.as_str()) == Some(x) && a.get(1).map(|s| s.as_str()) == Some(y);
        if is("agent", "read") { self.read.set(true); }
        if is("agent", "get") && self.read.get() { for p in self.inner.st.borrow_mut()["panes"].as_array_mut().unwrap() { if p.s("pane_id") == self.pane { p["agent"]["session"] = json!("replacement-session"); } } }
        self.inner.run(a, input)
    }
    fn kids(&self, pid: i64) -> Vec<i64> { self.inner.kids(pid) }
    fn wired_to(&self, pid: i64, group: &[i64]) -> Option<bool> { self.inner.wired_to(pid, group) }
    fn wiring(&self, pid: i64, group: &[i64]) -> sw::herdr::procs::Wiring { self.inner.wiring(pid, group) }
    fn kill(&self, pid: i64) { self.inner.kill(pid) }
}
fn swap_after_the_last_look(w: &World, t: &Obj) { sw::herdr::cli::set_fake(Some(Rc::new(Swap { inner: w.fake.clone(), read: Cell::new(false), pane: t.at("link").t("pane") }))); }
fn unswap(w: &World) { sw::herdr::cli::set_fake(Some(w.fake.clone())); }
const MULTI: &str = "Done.\n╭──\n│ ❯ │\n│ UNFINISHED DRAFT: do not submit │\n╰──";

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

#[test]
fn a_quoted_decision_a_session_swapped_right_before_a_draft_below_an_empty_first_line_or_another_terminal_after_exit_type_nothing_and_close_nothing() {
    let w = world();
    on();
    // a decision only quoted as the reply's last line
    let q = merged_with(&w, 30, "The shared watcher still runs. Once it is stopped, the line to say is:\n> safe to Exit", |_| {});
    accepted_now(&q);
    pass(&w, q.s("id"));
    held(&task(q.s("id")), "its reply gives no Exit decision");
    assert_eq!(exits(&w, &q), 0);
    // another session comes in between the last look and the typing: neither the closeout nor /exit goes in
    let a = merged_with(&w, 31, SAFE, |t| swap_after_the_last_look(&w, t));
    unswap(&w);
    assert_eq!(closeouts(&w, &a), 0);
    held(&task(a.s("id")), "nothing was typed: another session or terminal came into its pane right before");
    let b = merged(&w, 32);
    accepted_now(&b);
    swap_after_the_last_look(&w, &b);
    pass(&w, b.s("id"));
    unswap(&w);
    assert_eq!(exits(&w, &b), 0);
    held(&task(b.s("id")), "/exit was not typed: another session or terminal came into its pane right before");
    // a draft on the line below an empty first line, before the closeout and before /exit
    let d = merged_with(&w, 33, SAFE, |t| pane(&w, t, |p| p["agent"]["screen"] = json!(MULTI)));
    assert_eq!(closeouts(&w, &d), 0);
    held(&task(d.s("id")), "words wait unsent in its input box");
    let e = merged(&w, 34);
    pane(&w, &e, |p| p["agent"]["screen"] = json!(MULTI));
    accepted_now(&e);
    pass(&w, e.s("id"));
    assert_eq!(exits(&w, &e), 0);
    held(&task(e.s("id")), "words wait unsent in its input box");
    // after /exit: words of yours that only name /exit are new input; another terminal in the pane is not its agent's
    for (n, other) in [(35, false), (36, true)] {
        let t = merged(&w, n);
        accepted_now(&t);
        pass(&w, t.s("id"));
        assert_eq!(exits(&w, &t), 1);
        if !other { w.script(json!({ "worker": [{ "said": "Continuing." }] })); sw::herdr::Herdr::new(None).agent_prompt(t.at("link").s("pane"), "Do not /exit; continue the task instead.").unwrap(); settle(t.s("id")); }
        pane(&w, &t, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; if other { p["terminal_id"] = json!("terminal-belonging-to-someone-else"); } });
        pass(&w, t.s("id"));
        held(&task(t.s("id")), if other { "another terminal than its agent's is in its pane" } else { "new input reached its agent after /exit was typed" });
        assert!(open(&w, &task(t.s("id"))));
    }
    set_fake(None);
}
