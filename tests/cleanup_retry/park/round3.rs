//! The reviewer's round 3 park probes: a restart that finds another session keeps the words and presses nothing, and
//! the next round after a parked one is approved and closed out by itself.
use super::*;

fn activity(w: &World, n: usize) -> Vec<String> {
    calls(w)[n..].iter().filter(|c| c.starts_with("agent start") || c.starts_with("agent prompt") || c.starts_with("agent keys") || c.starts_with("pane send")).cloned().collect()
}
fn foreign(w: &World, t: &Obj) {
    pane(w,t,|p| { p["agent"]=json!({"kind":"claude","name":"unrelated","session":"different-session","status":"idle","seq":900,"args":[]}); });
}
#[test]
fn round3_probe_foreign_restart_must_keep_undelivered_inbox() {
    let w=world();on();let t=parked(&w,71,true);let id=t.t("id");
    let words="Private answer for the original worker";
    sw::actions::talk::tell(&id,words,false).unwrap();
    assert!(sw::store::inbox::pending_text(&id).iter().any(|m|m.s("text")==words));
    foreign(&w,&t);let n=calls(&w).len();
    settle(&id);
    let pending=sw::store::inbox::pending_text(&id);
    assert!(activity(&w,n).is_empty());
    set_fake(None);
    assert!(pending.iter().any(|m|m.s("text")==words),"Undelivered reply lost: pending={pending:?}; stage={}; failure={}",task(&id).s("stage"),task(&id).at("failure").s("detail"));
}
#[test]
fn round3_probe_next_approval_with_default_closeout_off() {
    let w=world();on();
    assert_eq!(sw::store::settings::lookup::setting("workspace.closeout",None),json!(false));
    let t=parked(&w,72,false);let id=t.t("id");
    sw::actions::finish_line::approve(&id,None,false).unwrap();settle(&id);
    sw::land::cleanup_job(&id,"merged");
    assert!(task(&id).b("cleaned"));
    let record=task(&id).at("closeout").clone();
    w.script(json!({"worker":[{"files":{"second-round-off.txt":"new work\n"},"commit":"second round","note":"second round done","said":"second round done"}]}));
    delegate(&id);let c=settle(&id);
    assert_eq!(c.s("stage"),"approval","next round failed: {}",c.s("msg"));
    let r=sw::actions::finish_line::approve(&id,None,false);
    set_fake(None);
    assert!(r.is_ok(),"F5: closeout-off cleanup_record={record}; current={}; second approve error={}",c.at("closeout"),r.unwrap_err().msg);
}
#[test]
fn round3_probe_foreign_restart_must_not_interrupt_continuation_prompt() {
    let w=world();on();let t=parked(&w,73,true);let id=t.t("id");
    sw::actions::talk::tell(&id,"Original worker only",false).unwrap();
    foreign(&w,&t);pane(&w,&t,|p|p["screen"]=json!("unrelated shell command\nquote> "));
    let n=calls(&w).len();settle(&id);let a=activity(&w,n);
    set_fake(None);
    assert!(a.is_empty(),"F1 guard failure handling still inputs to foreign pane: {a:?}");
}
#[test]
fn round3_probe_next_round_cleanup_must_start_new_closeout() {
    let w=world();on();
    sw::store::settings::change::set_value("workspace.closeout",&json!(true),&sw::store::settings::lookup::Scope::you()).unwrap();
    let t=parked(&w,74,false);let id=t.t("id");
    sw::actions::finish_line::approve(&id,None,false).unwrap();settle(&id);
    sw::land::cleanup_job(&id,"merged");assert!(task(&id).b("cleaned"));
    w.script(json!({"worker":[{"files":{"second-round-cleanup.txt":"new work\n"},"commit":"second round","note":"second round done","said":"second round done"},{"said":SAFE}]}));
    delegate(&id);let c=settle(&id);assert_eq!(c.s("stage"),"approval");
    let session=c.t("session");
    pane(&w,&c,|p| { p["frontPid"]=json!(5074);p["agent"]["session"]=json!(session);p["agent"]["screen"]=json!("Done.\n╭──\n│ ❯ \n╰──"); });
    let n=calls(&w).len();
    sw::actions::finish_line::approve(&id,None,false).unwrap();let c=settle(&id);
    let acts=activity(&w,n);set_fake(None);
    assert_eq!(c.s("stage"),"done");
    assert!(acts.iter().any(|a|a.starts_with("agent prompt") && a.contains("session-close")),"New round did not get its own closeout: current_session={session}; closeout={}; msg={}; activity={acts:?}",c.at("closeout"),c.s("msg"));
}
