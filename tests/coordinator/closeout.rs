//! A coordinator's record-only closeout before it is stopped or begun anew (`workspace.closeout`, coordinator/closeout.rs):
//! the stop waits until its record is accepted, its reply says `safe to Exit`, `/exit` went in and its agent is gone;
//! what cannot be proved leaves it running, with the reason, and the stop not carried out.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::rc::Rc;
use sw::coordinator::{pump::pump, requests::{fresh, send, stop}, state};
use sw::json::*;
use sw::memory_queue::{set_fake, Job};
use super::coord_world;

fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn get(w: &World) -> Obj { state::get(&w.socket).unwrap_or_default() }
fn agents(w: &World) -> usize { w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).count() }
fn typed(w: &World) -> Vec<String> { prompts(w, get(w).s("pane")) }
fn pane(w: &World, f: impl Fn(&mut Value)) { let id = get(w).t("pane"); w.patch(|s| { for p in s["panes"].as_array_mut().unwrap() { if p.s("pane_id") == id { f(p); } } }); }
fn session_of(w: &World) -> String { get(w).t("session") }
fn queue(r: Result<Vec<Job>, String>) { set_fake(Some(Rc::new(move |_| r.clone()))); }

#[test]
fn a_stop_waits_for_the_closeout_each_step_proved_and_what_cannot_be_proved_leaves_it_running() {
    let w = coord_world();
    sw::store::settings::change::set_value("workspace.closeout", &json!(true), &sw::store::settings::lookup::Scope::you()).unwrap();
    sw::clock::set(Some(sw::clock::now()));
    w.script(json!({ "coordinator": [{ "said": "Hello." }, { "said": "Saved.\nsafe to Exit" }] }));
    send(&w.root, "first", false, "", &w.socket).unwrap(); drain();
    assert_eq!(agents(&w), 1);
    pane(&w, |p| { p["frontPid"] = json!(5150); p["screen"] = json!("user@host:~$ claude --resume x\nuser@host:~$ "); p["agent"]["screen"] = json!("╭──\n│ ❯ │\n╰──"); p["agent"]["session"] = json!(session_of(&w)); });
    queue(Ok(vec![]));
    stop(&w.socket).unwrap(); drain();
    // sent once; not stopped while its record is not accepted
    assert_eq!(typed(&w).iter().filter(|p| p.starts_with("/session-close 記録だけ")).count(), 1, "{:?}", typed(&w));
    assert_eq!(get(&w).at("closeout").s("phase"), "checkpointing"); assert!(get(&w).b("want")); assert_eq!(agents(&w), 1);
    pump(&w.socket); drain();
    assert_eq!(agents(&w), 1);
    // accepted: /exit, once; stopped only once its agent is gone
    let (session, cwd) = (get(&w).t("session"), get(&w).at("closeout").t("cwd"));
    assert!(!session.is_empty());
    queue(Ok(vec![Job { id: "c1".into(), status: "queued".into(), accepted: sw::clock::now() + 1, cwd }]));
    pump(&w.socket); drain();
    assert_eq!(get(&w).at("closeout").s("phase"), "closing", "{:?}", get(&w));
    assert_eq!(typed(&w).iter().filter(|p| p.as_str() == "/exit").count(), 1);
    pump(&w.socket); drain();
    assert_eq!(agents(&w), 1); assert_eq!(typed(&w).iter().filter(|p| p.as_str() == "/exit").count(), 1);
    pane(&w, |p| { p["agent"] = Value::Null; p["frontPid"] = Value::Null; });
    pump(&w.socket); drain();
    assert_eq!(get(&w).at("stopped"), &json!(true), "{:?}", get(&w)); assert!(!get(&w).b("closeout") && !get(&w).b("want"));
    // begun anew while the queue cannot be read: held once closeoutWait has passed; it keeps running, with the reason
    w.script(json!({ "coordinator": [{ "said": "Back." }, { "said": "Saved.\nsafe to Exit" }] }));
    sw::clock::advance(5); send(&w.root, "again", false, "", &w.socket).unwrap(); drain();
    assert_eq!(agents(&w), 1);
    pane(&w, |p| { p["frontPid"] = json!(5152); p["agent"]["screen"] = json!("╭──\n│ ❯ │\n╰──"); p["agent"]["session"] = json!(session_of(&w)); });
    queue(Err("no Agent Memory queue".into()));
    fresh(None, &w.socket).unwrap(); drain();
    assert_eq!(get(&w).at("closeout").s("phase"), "checkpointing");
    sw::clock::advance(11 * 60 * 1000);
    pump(&w.socket); drain();
    assert!(get(&w).s("error").contains("closeout 未確認: no Agent Memory queue"), "{}", get(&w).s("error"));
    assert!(!get(&w).b("want")); assert_eq!(get(&w).at("closeout").s("phase"), "checkpointing", "held: its record stays, so a stop asked again never saves twice"); assert_eq!(agents(&w), 1);
    assert_eq!(typed(&w).iter().filter(|p| p.as_str() == "/exit").count(), 0);
    set_fake(None);
}

#[test]
fn a_reply_to_other_words_before_the_first_look_is_not_the_closeout_s_and_the_stop_is_held() {
    let w = coord_world();
    sw::store::settings::change::set_value("workspace.closeout", &json!(true), &sw::store::settings::lookup::Scope::you()).unwrap();
    sw::clock::set(Some(sw::clock::now()));
    w.script(json!({ "coordinator": [{ "said": "Hello." }, { "said": "Saved.\nsafe to Exit" }, { "said": "Done that too. safe to Exit" }] }));
    send(&w.root, "first", false, "", &w.socket).unwrap(); drain();
    pane(&w, |p| { p["frontPid"] = json!(5151); p["agent"]["screen"] = json!("╭──\n│ ❯ │\n╰──"); p["agent"]["session"] = json!(session_of(&w)); });
    queue(Ok(vec![]));
    stop(&w.socket).unwrap(); drain();
    assert_eq!(get(&w).at("closeout").s("phase"), "checkpointing");
    sw::herdr::Herdr::new(None).agent_prompt(get(&w).s("pane"), "something else").unwrap(); // typed in its tab before the board looked
    let cwd = get(&w).at("closeout").t("cwd");
    queue(Ok(vec![Job { id: "c1".into(), status: "queued".into(), accepted: sw::clock::now() + 1, cwd }]));
    pump(&w.socket); drain();
    assert!(get(&w).s("error").contains("its last turn answers something else"), "{}", get(&w).s("error"));
    assert_eq!(agents(&w), 1); assert!(!get(&w).b("want"));
    assert_eq!(typed(&w).iter().filter(|p| p.as_str() == "/exit").count(), 0);
    set_fake(None);
}

#[test]
fn round16_a_held_stop_retains_the_accepted_closeout_and_does_not_save_twice() {
    let w = coord_world();
    sw::store::settings::change::set_value("workspace.closeout", &json!(true), &sw::store::settings::lookup::Scope::you()).unwrap();
    sw::clock::set(Some(sw::clock::now()));
    w.script(json!({ "coordinator": [{ "said": "Hello." }, { "said": "Saved.\nsafe to Exit" }, { "said": "Saved again.\nsafe to Exit" }] }));
    send(&w.root, "first", false, "", &w.socket).unwrap(); drain();
    pane(&w, |p| { p["frontPid"] = json!(5155); p["agent"]["screen"] = json!("╭──\n│ ❯ │\n╰──"); p["agent"]["session"] = json!(session_of(&w)); });
    queue(Ok(vec![])); stop(&w.socket).unwrap(); drain();
    let accepted_session = session_of(&w);
    let cwd = get(&w).at("closeout").t("cwd");
    queue(Ok(vec![Job { id: "accepted-snapshot".into(), status: "queued".into(), accepted: sw::clock::now() + 1, cwd }]));
    pane(&w, |p| p["agent"]["screen"] = json!("╭──\n│ ❯ UNSENT DRAFT │\n╰──"));
    pump(&w.socket); drain();
    assert!(get(&w).s("error").contains("words wait unsent"));
    let held_record = get(&w).at("closeout").clone();
    assert_eq!(typed(&w).iter().filter(|p| p.as_str() == "/exit").count(), 0);
    // Only the unsent draft is cleared; same session, same artifact and accepted save. Retry the authorized stop.
    pane(&w, |p| p["agent"]["screen"] = json!("╭──\n│ ❯ │\n╰──"));
    sw::clock::advance(5); stop(&w.socket).unwrap(); drain();
    let count = typed(&w).iter().filter(|p| p.starts_with("/session-close 記録だけ")).count();
    eprintln!("round16 coordinator retry: held_record={held_record}; session_unchanged={}; accepted_job=accepted-snapshot; closeouts={count}; phase={}", accepted_session == session_of(&w), get(&w).at("closeout").s("phase"));
    set_fake(None);
    assert_eq!(count, 1, "same accepted snapshot must not be saved a second time after a held stop");
}

#[test]
fn round17_a_held_stop_open_same_session_stop_does_not_save_twice() {
    let w = coord_world();
    sw::store::settings::change::set_value("workspace.closeout", &json!(true), &sw::store::settings::lookup::Scope::you()).unwrap();
    sw::clock::set(Some(sw::clock::now()));
    w.script(json!({ "coordinator": [{ "said": "Hello." }, { "said": "Saved.\nsafe to Exit" }, { "said": "Saved again.\nsafe to Exit" }] }));
    send(&w.root, "first", false, "", &w.socket).unwrap(); drain();
    pane(&w, |p| { p["frontPid"] = json!(5155); p["agent"]["screen"] = json!("╭──\n│ ❯ │\n╰──"); p["agent"]["session"] = json!(session_of(&w)); });
    queue(Ok(vec![])); stop(&w.socket).unwrap(); drain();
    let accepted_session = session_of(&w);
    let cwd = get(&w).at("closeout").t("cwd");
    queue(Ok(vec![Job { id: "accepted-snapshot".into(), status: "queued".into(), accepted: sw::clock::now() + 1, cwd }]));
    pane(&w, |p| p["agent"]["screen"] = json!("╭──\n│ ❯ UNSENT DRAFT │\n╰──"));
    pump(&w.socket); drain();
    assert!(get(&w).s("error").contains("words wait unsent"));
    let held_record = get(&w).at("closeout").clone();
    assert_eq!(typed(&w).iter().filter(|p| p.as_str() == "/exit").count(), 0);
    // Only the unsent draft is cleared; same session, same artifact and accepted save. Retry the authorized stop.
    pane(&w, |p| p["agent"]["screen"] = json!("╭──\n│ ❯ │\n╰──"));
    sw::clock::advance(5);
    sw::coordinator::requests::open(&w.socket).unwrap(); drain();
    assert_eq!(accepted_session, session_of(&w), "open preserves the same conversation");
    assert_eq!(typed(&w).iter().filter(|p| !p.starts_with("/session-close")).count(), 1, "no new work sent");
    let after_open_record = get(&w).at("closeout").clone();
    sw::clock::advance(5); stop(&w.socket).unwrap(); drain();
    eprintln!("ROUND17_COORDINATOR after_open_record={after_open_record}; same_session={}; agents={}", accepted_session == session_of(&w), agents(&w));
    let count = typed(&w).iter().filter(|p| p.starts_with("/session-close 記録だけ")).count();
    eprintln!("ROUND17_COORDINATOR retry: held_record={held_record}; session_unchanged={}; accepted_job=accepted-snapshot; closeouts={count}; phase={}", accepted_session == session_of(&w), get(&w).at("closeout").s("phase"));
    set_fake(None);
    assert_eq!(count, 1, "same accepted snapshot must not be saved a second time after a held stop");
}
