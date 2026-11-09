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


#[test]
fn round8_f15_slow_queue_query_must_not_allow_two_exit_sends() {
    let w=world(); on(); let t=waiting(&w,101,false,SAFE); let id=t.t("id");
    sw::land::park::park(&id).unwrap();
    let entered=std::cell::Cell::new(false); let other=id.clone();
    let (cwd,at)=(t.t("worktree"),sw::clock::now()+1);
    set_fake(Some(Rc::new(move |_| {
        if !entered.replace(true) {
            sw::clock::advance(120001);
            sw::land::park::drive(&other);
        }
        Ok(vec![Job {id:"j1".into(),status:"queued".into(),accepted:at,cwd:cwd.clone()}])
    })));
    sw::land::park::drive(&id);
    let n=exits(&w,&t); let c=task(&id); set_fake(None);
    assert_eq!(n,1,"F15: slow in-flight query permits overlapping drivers; exit_count={n}; phase={}; held={}; stage={}",c.at("closeout").s("phase"),c.at("closeout").b("held"),c.s("stage"));
}

#[test]
fn round8_f16_reopen_after_crash_then_drive_must_hold_without_starting() {
    let w=world(); on(); let t=waiting(&w,102,true,SAFE); let id=t.t("id");
    sw::land::park::park(&id).unwrap();
    pane(&w,&t,|p| {p["agent"]=Value::Null; p["frontPid"]=Value::Null;});
    let n=calls(&w).len(); let (_,has,_)=sw::delegate::reopen(&id,true).unwrap();
    sw::land::park::drive(&id);
    let c=task(&id); let a=activity(&w,n); set_fake(None);
    assert!(!has && a.is_empty(),"crash reopened: has={has} activity={a:?}");
    assert!(c.at("closeout").b("held") && !sw::land::park::parked(&c) && c.s("msg").contains("before it was told to exit"),"crash not held: {}",c.at("closeout"));
    assert_eq!(c.s("stage"),t.s("stage"));
}

#[test]
fn round8_f14_reply_busy_or_unknown_shell_must_type_nothing() {
    let w=world(); on(); let t=parked(&w,103,true); let id=t.t("id");
    pane(&w,&t,|p|p["busy"]=json!(true));
    let n=calls(&w).len();
    let r1=sw::actions::talk::tell(&id,"Original worker only",false);
    pane(&w,&t,|p|p["busy"]=json!(false));
    w.patch(|s|s["processInfoFails"]=json!(true));
    let r2=sw::actions::talk::tell(&id,"Original worker only",false);
    let a=activity(&w,n);set_fake(None);
    assert!(r1.is_err() && r2.is_err() && a.is_empty(),"busy_ok={} unknown_ok={} activity={a:?}",r1.is_ok(),r2.is_ok());
}

#[test]
fn round8_f14_queued_restart_must_recheck_processes_and_keep_inbox() {
    let w=world(); on(); let t=parked(&w,104,true); let id=t.t("id");
    sw::actions::talk::tell(&id,"Original worker only",false).unwrap();
    pane(&w,&t,|p|p["busy"]=json!(true));
    let n=calls(&w).len();settle(&id);let a=activity(&w,n);
    let pending=sw::store::inbox::pending_text(&id);set_fake(None);
    assert!(a.is_empty(),"queued restart: {a:?}");
    assert!(pending.iter().any(|m|m.s("text")=="Original worker only"),"queued reply lost: {pending:?}");
}


struct AtExitRead {
    inner: Rc<common::herdr::FakeHerdr>,
    id: String,
    fired: std::cell::Cell<bool>,
    human_turn: bool,
}
impl sw::herdr::cli::Fake for AtExitRead {
    fn run(&self, args: &[String], input: &str) -> sw::herdr::cli::Ran {
        if args.first().map(String::as_str)==Some("agent") && args.get(1).map(String::as_str)==Some("read")
            && task(&self.id).at("closeout").s("phase")=="exiting" && !self.fired.replace(true) {
            let old = task(&self.id).at("parkDriver").clone();
            sw::clock::advance(2000);
            sw::land::park::drive(&self.id);
            let c=task(&self.id);
            assert_ne!(c.at("parkDriver"), &old, "takeover did not happen");
            assert!(c.at("closeout").b("held"), "second driver must hold uncertain send");
            if self.human_turn {
                sw::actions::talk::tell(&self.id,"New human work: keep this session open",false).unwrap();
            }
        }
        self.inner.run(args,input)
    }
    fn kids(&self, pid:i64)->Vec<i64>{ self.inner.kids(pid) }
    fn wiring(&self, pid:i64, group:&[i64])->sw::herdr::procs::Wiring{self.inner.wiring(pid,group)}
    fn command(&self,pid:i64)->Option<String>{self.inner.command(pid)}
    fn alive(&self,pid:i64)->Option<bool>{self.inner.alive(pid)}
    fn kill(&self,pid:i64){self.inner.kill(pid)}
}
fn revoked_during_exit_read(human_turn:bool) {
    let w=world(); on(); let t=waiting(&w,109+human_turn as i64,false,SAFE); let id=t.t("id");
    sw::land::park::park(&id).unwrap();
    let (cwd, at)=(t.t("worktree"),sw::clock::now()+1);
    let queried=std::cell::Cell::new(false);
    set_fake(Some(Rc::new(move |_| {
        if !queried.replace(true) {sw::clock::advance(119000);}
        Ok(vec![Job{id:"j1".into(),status:"queued".into(),accepted:at,cwd:cwd.clone()}])
    })));
    w.script(json!({"worker":[{"said":"New work in progress","status":"idle"}]}));
    let probe=Rc::new(AtExitRead{inner:w.fake.clone(),id:id.clone(),fired:std::cell::Cell::new(false),human_turn});
    sw::herdr::cli::set_fake(Some(probe.clone()));
    sw::land::park::drive(&id);
    let c=task(&id);let n=exits(&w,&t);let triggered=probe.fired.get();
    sw::herdr::cli::set_fake(Some(w.fake.clone()));set_fake(None);
    assert!(triggered,"probe never reached post-reservation exit read");
    assert_eq!(n,0,"F15: revoked driver still sent /exit; human_turn={human_turn}; exit_count={n}; phase={}; held={}; stage={}; prompts={:?}",c.at("closeout").s("phase"),c.at("closeout").b("held"),c.s("stage"),prompts(&w,t.at("link").s("pane")));
}
#[test]
fn round9_f15_revoked_driver_must_not_send_after_exit_reservation() {revoked_during_exit_read(false);}
#[test]
fn round9_f15_revoked_driver_must_not_exit_new_human_turn() {revoked_during_exit_read(true);}
