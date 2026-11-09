//! Independent round 7: no input or stage movement while a park is in progress.
use super::*;
fn activity(w: &World, n: usize) -> Vec<String> {
    calls(w)[n..].iter().filter(|c| c.starts_with("agent start") || c.starts_with("agent prompt") || c.starts_with("agent send-keys") || c.starts_with("pane send") || c.starts_with("pane run")).cloned().collect()
}
#[test]
fn round7_delegate_during_checkpoint_must_not_send_or_change_waiting_stage() {
    let w=world(); on(); let t=waiting(&w,91,false,SAFE); let id=t.t("id");
    sw::land::park::park(&id).unwrap();
    assert!(sw::land::park::busy(&task(&id)));
    let n=calls(&w).len();
    let result=sw::actions::start::delegate(&id,vec![]);
    settle(&id); let c=task(&id); let a=activity(&w,n); set_fake(None);
    assert!(a.is_empty() && c.s("stage")==t.s("stage"), "delegate_ok={} before={} after={} closeout={} activity={a:?}",result.is_ok(),t.s("stage"),c.s("stage"),c.at("closeout"));
}
#[test]
fn round7_reopen_during_checkpoint_after_crash_must_not_start() {
    let w=world(); on(); let t=waiting(&w,92,true,SAFE); let id=t.t("id");
    sw::land::park::park(&id).unwrap();
    assert!(sw::land::park::busy(&task(&id)));
    pane(&w,&t,|p| {p["agent"]=Value::Null; p["frontPid"]=Value::Null;});
    let n=calls(&w).len(); let result=sw::delegate::reopen(&id,true);
    let c=task(&id); let a=activity(&w,n); set_fake(None);
    assert!(a.is_empty(), "reopen_ok={} has_agent={:?} stage={} closeout={} activity={a:?}",result.is_ok(),result.as_ref().map(|(_,h,_)|h),c.s("stage"),c.at("closeout"));
}

#[test]
fn round7_busy_shell_after_park_must_receive_no_input() {
    let w=world(); on(); let t=parked(&w,93,true);
    pane(&w,&t,|p| p["busy"]=json!(true));
    let api=sw::flow::task::h(&t); assert!(!api.pane_idle(t.at("link").s("pane")));
    let n=calls(&w).len(); let result=sw::delegate::reopen(t.s("id"),true);
    let a=activity(&w,n); set_fake(None);
    assert!(a.is_empty(), "foreground sleep: reopen_ok={} activity={a:?}",result.is_ok());
}
#[test]
fn round7_unknown_process_state_after_park_must_receive_no_input() {
    let w=world(); on(); let t=parked(&w,94,true);
    w.patch(|s| s["processInfoFails"]=json!(true));
    let api=sw::flow::task::h(&t); assert!(!api.pane_idle(t.at("link").s("pane")));
    let n=calls(&w).len(); let result=sw::delegate::reopen(t.s("id"),true);
    let a=activity(&w,n); set_fake(None);
    assert!(a.is_empty(), "unknown processes: reopen_ok={} activity={a:?}",result.is_ok());
}
#[test]
fn round7_two_interleaved_park_drivers_must_not_send_exit_twice() {
    let w=world(); on(); let t=waiting(&w,95,false,SAFE); let id=t.t("id");
    sw::land::park::park(&id).unwrap();
    let entered=std::cell::Cell::new(false); let other=id.clone();
    let (cwd,at)=(t.t("worktree"),sw::clock::now()+1);
    set_fake(Some(Rc::new(move |_| {
        if !entered.replace(true) { sw::land::park::drive(&other); }
        Ok(vec![Job {id:"j1".into(),status:"queued".into(),accepted:at,cwd:cwd.clone()}])
    })));
    sw::land::park::drive(&id);
    let n=exits(&w,&t); let c=task(&id); set_fake(None);
    assert_eq!(n,1,"interleaved park drivers sent /exit {n} times; closeout={}",c.at("closeout"));
}
