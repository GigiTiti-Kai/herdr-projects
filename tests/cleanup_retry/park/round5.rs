//! Independent round 5: a parked worker's shell, terminal identity and externally resumed conversation.
use super::*;

fn activity(w: &World, n: usize) -> Vec<String> {
    calls(w)[n..].iter().filter(|c| c.starts_with("agent start") || c.starts_with("agent prompt") || c.starts_with("agent send-keys") || c.starts_with("pane send") || c.starts_with("pane run")).cloned().collect()
}

#[test]
fn round5_reopen_replaced_terminal_must_type_nothing() {
    let w=world(); on(); let t=parked(&w,81,false);
    pane(&w,&t,|p| {p["terminal_id"]=json!("unrelated-terminal");p["cwd"]=json!(w.root.display().to_string());});
    let n=calls(&w).len();
    let result=sw::delegate::reopen(t.s("id"),true);
    let a=activity(&w,n);set_fake(None);
    assert!(a.is_empty(),"replaced shell: reopen_ok={} has_agent={:?} activity={a:?}",result.is_ok(),result.as_ref().map(|(_,h,_)|h));
}

#[test]
fn round5_reply_replaced_terminal_must_type_nothing_and_keep_words() {
    let w=world(); on(); let t=parked(&w,82,true);let id=t.t("id");
    pane(&w,&t,|p| {p["terminal_id"]=json!("unrelated-terminal");p["cwd"]=json!(w.root.display().to_string());});
    let n=calls(&w).len();
    let result=sw::actions::talk::tell(&id,"Private original-worker answer",false);
    settle(&id);let a=activity(&w,n);set_fake(None);
    assert!(a.is_empty(),"replaced shell: reply_ok={} activity={a:?}",result.is_ok());
}

#[test]
fn round5_reopen_shell_with_unsent_input_must_type_nothing() {
    let w=world(); on(); let t=parked(&w,83,true);
    pane(&w,&t,|p|p["screen"]=json!("user@host:/work$ claude --session-id x\nBye.\nuser@host:/work$ echo unfinished-private-draft"));
    let n=calls(&w).len();
    let result=sw::delegate::reopen(t.s("id"),true);
    let a=activity(&w,n);set_fake(None);
    assert!(a.is_empty(),"unsent shell input: reopen_ok={} activity={a:?}",result.is_ok());
}

#[test]
fn round5_same_session_return_new_human_turn_must_revoke_approval() {
    let w=world(); on(); let t=parked(&w,84,false);let id=t.t("id");
    pane(&w,&t,|p|p["agent"]=json!({"kind":"claude","name":"original-worker","session":t.s("session"),"status":"idle","seq":900,"args":["--resume",t.s("session")]}));
    w.script(json!({"worker":[{"said":"New human work started", "status":"working"}]}));
    sw::flow::task::h(&t).agent_prompt(t.at("link").s("pane"),"Human asks for another edit").unwrap();
    on_pane(&w,t.at("link").s("pane"));let c=task(&id);set_fake(None);
    assert_eq!(c.s("stage"),"working","new human turn swallowed: stage={} requireApproval={} parked={} session={}",c.s("stage"),c.b("requireApproval"),sw::land::park::parked(&c),c.s("session"));
}

#[test]
fn round5_restart_same_session_return_must_end_park() {
    let w=world(); on(); let t=parked(&w,85,true);let id=t.t("id");
    sw::actions::talk::tell(&id,"Continue original work",false).unwrap();
    w.script(json!({"worker":[{"said":"New fix complete"}]}));
    pane(&w,&t,|p|p["agent"]=json!({"kind":"claude","name":"original-worker","session":t.s("session"),"status":"idle","seq":900,"args":["--resume",t.s("session")]}));
    settle(&id);
    let c=task(&id);let result=sw::flow::send::send_own(&id,"Fix check","check",false,Obj::new(),0);set_fake(None);
    assert!(!sw::land::park::parked(&c),"live same-session reply leaves stale park: stage={} send_own_error={:?}",c.s("stage"),result.err().map(|e|e.msg));
}
