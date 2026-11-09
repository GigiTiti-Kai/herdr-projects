//! The reviewer's round 2 park probes: the next round approved after it finishes, and a restart or a delegation
//! that finds another session in a parked task's pane.
use super::*;

fn activity(w: &World, n: usize) -> Vec<String> {
    calls(w)[n..].iter().filter(|c| c.starts_with("agent start") || c.starts_with("agent prompt") || c.starts_with("agent keys") || c.starts_with("pane send")).cloned().collect()
}
fn foreign(w: &World, t: &Obj) {
    pane(w,t,|p| { p["agent"]=json!({"kind":"claude","name":"unrelated","session":"different-session","status":"idle","seq":900,"args":[]}); });
}

#[test]
fn round2_f5_next_round_can_be_approved_after_it_finishes() {
    let w=world(); on();
    sw::store::settings::change::set_value("workspace.closeout",&json!(true),&sw::store::settings::lookup::Scope::you()).unwrap();
    let t=parked(&w,61,false);let id=t.t("id");
    sw::actions::finish_line::approve(&id,None,false).unwrap();settle(&id);
    sw::land::cleanup_job(&id,"merged");
    assert!(task(&id).b("cleaned"));
    w.script(json!({"worker":[{"files":{"second-round.txt":"new work\n"},"commit":"second round","note":"second round done","said":"second round done"}]}));
    delegate(&id); let c=settle(&id);
    assert_eq!(c.s("stage"),"approval","next round failed before approval: {}",c.s("msg"));
    let r=sw::actions::finish_line::approve(&id,None,false);
    assert!(r.is_ok(),"F5 second approval rejected: {}; park_record={}; busy={}",r.unwrap_err().msg,c.at("closeout"),sw::land::park::busy(&c));
    set_fake(None);
}

#[test]
fn round2_f1_restart_rechecks_identity_after_reply_was_queued() {
    let w=world();on();let t=parked(&w,62,true);let id=t.t("id");
    sw::actions::talk::tell(&id,"Private answer for the original worker",false).unwrap();
    foreign(&w,&t);
    let n=calls(&w).len();
    settle(&id);
    let a=activity(&w,n);
    assert!(a.is_empty(),"F1 queued restart sent into a replacement session: {a:?}");
    set_fake(None);
}

#[test]
fn round2_f1_delegate_rechecks_parked_session() {
    let w=world();on();let t=parked(&w,63,true);let id=t.t("id");
    foreign(&w,&t);
    let n=calls(&w).len();
    let r=sw::actions::start::delegate(&id,vec![]);
    settle(&id);
    let a=activity(&w,n);
    assert!(a.is_empty(),"F1 redelegate ok={} activity={a:?}",r.is_ok());
    set_fake(None);
}
