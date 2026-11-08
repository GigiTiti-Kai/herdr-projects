//! The reviewer's park probes (round 1): another session in its pane, its worktree gone, a plan writer, the agent's own
//! report, a new round after its work merged, a reopen that cannot resume, a review sent back and a fix.
use super::*;

fn activity(w: &World, n: usize) -> Vec<String> {
    calls(w)[n..].iter().filter(|c| c.starts_with("agent start") || c.starts_with("agent prompt") || c.starts_with("agent keys") || c.starts_with("pane send")).cloned().collect()
}

#[test]
fn reviewer_missing_worktree_must_stop_before_reopen() {
    let w = world(); on();
    let t = parked(&w, 41, false);
    std::fs::rename(t.s("worktree"), format!("{}.missing", t.s("worktree"))).unwrap();
    let n = calls(&w).len();
    let r = sw::delegate::reopen(t.s("id"), true);
    let acts = activity(&w, n);
    assert!(acts.is_empty(), "missing worktree: reopen_ok={} has_agent={:?}; activity={acts:?}; fallback cwd={}", r.is_ok(), r.as_ref().map(|(_,has,_)|has), sw::store::repos::cwd_of(&task(t.s("id"))).display());
    set_fake(None);
}

#[test]
fn reviewer_replaced_session_must_receive_no_reply() {
    let w = world(); on();
    let t = parked(&w, 42, true);
    pane(&w, &t, |p| { p["agent"] = json!({"kind":"claude", "name":"unrelated", "session":"different-session", "status":"idle", "seq":900, "args":[]}); });
    let n = calls(&w).len();
    let r = sw::actions::talk::tell(t.s("id"), "Private answer for the original worker", false);
    let acts = activity(&w, n);
    assert!(acts.is_empty(), "replaced session: send_ok={}; activity={acts:?}", r.is_ok());
    set_fake(None);
}

#[test]
fn reviewer_closeout_blocked_report_must_preserve_approval_and_original_question() {
    let w = world(); on();
    let t = waiting(&w, 43, false, SAFE);
    sw::land::park::park(t.s("id")).unwrap();
    sw::actions::filing::report(t.s("id"), sw::actions::filing::Report { state:"blocked".into(), kind:"question".into(), msg:"Shared monitor is still running; cannot exit".into(), own:"agent", ..Default::default() }).unwrap();
    let c = task(t.s("id"));
    assert_eq!(c.s("stage"), t.s("stage"), "closeout report changed task: before={} after={} msg={}", t.s("stage"),c.s("stage"),c.s("msg"));
    set_fake(None);
}

#[test]
fn reviewer_a_planner_asking_a_question_must_not_get_closeout() {
    let w = world_with(Opts { repo:true, settings:json!({"plan.approve":"you", "plan.agent":"claude", "review.when":"never"}), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_SANDBOX", "bwrap"); }
    w.patch(|s| s["pids"] = json!(true));
    on();
    w.script(json!({"worker":[{"said":"Which architecture should the plan use?"},{"said":SAFE}]}));
    let id=w.add("Planning question"); delegate(&id);
    let t=settle(&id);
    assert!(sw::plan::planner(&t), "not a planner fixture: {t:?}");
    sw::actions::filing::report(&id, sw::actions::filing::Report {state:"blocked".into(),kind:"question".into(),msg:"Which architecture?".into(),own:"agent",..Default::default()}).unwrap();
    let t=task(&id);
    pane(&w,&t,|p|{p["frontPid"]=json!(4044);p["agent"]["session"]=json!(t.s("session"));p["agent"]["screen"]=json!("Done.\n╭──\n│ ❯ \n╰──");});
    let n=calls(&w).len();
    let r=sw::land::park::park(&id);
    let acts=activity(&w,n);
    assert!(acts.is_empty(), "planner={} park_ok={}; activity={acts:?}",sw::plan::planner(&t),r.is_ok());
    set_fake(None);
}

#[test]
fn reviewer_missing_transcript_reopen_must_explain_why() {
    let w=world(); on(); let t=parked(&w,45,false);
    std::fs::remove_dir_all(std::path::Path::new(&std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join("projects")).unwrap();
    let (_,has,c)=sw::delegate::reopen(t.s("id"),true).unwrap();
    assert!(!has);
    assert!(c.s("msg").contains("cannot be resumed"),"reopen did not say why it failed: {}", c.s("msg"));
    set_fake(None);
}

#[test]
fn reviewer_review_sendback_resumes_same_session() {
    let w=world(); on(); let t=parked(&w,46,false); let id=t.t("id");
    update(&id,|c|{c.put("review",json!({"text":"Repair the missing branch", "score":2}));Ok(Change::Write)}).unwrap();
    let n=calls(&w).len();
    sw::actions::finish_line::send_review(&id).unwrap();
    let c=settle(&id); let acts=activity(&w,n);
    assert!(acts.iter().any(|c|c.starts_with("agent start") && c.contains(&format!("--resume {}",t.s("session")))), "{acts:?}");
    assert!(!sw::land::park::parked(&c)); set_fake(None);
}

#[test]
fn reviewer_fix_resumes_same_session() {
    let w=world(); on(); let t=parked(&w,47,false);let id=t.t("id");
    update(&id,|c|{c.put("failure",json!({"stage":"check","title":"Check failed","detail":"Repair it", "agentCanFix":true}));Ok(Change::Write)}).unwrap();
    let n=calls(&w).len();sw::actions::finish_line::send_failure(&id).unwrap(); let c=settle(&id);let acts=activity(&w,n);
    assert!(acts.iter().any(|c|c.starts_with("agent start") && c.contains(&format!("--resume {}",t.s("session")))), "{acts:?}");
    assert!(!sw::land::park::parked(&c));set_fake(None);
}

#[test]
fn reviewer_new_round_after_parked_work_is_merged_can_start() {
    let w=world();on();
    sw::store::settings::change::set_value("workspace.closeout",&json!(true),&sw::store::settings::lookup::Scope::you()).unwrap();
    let t=parked(&w,48,false);let id=t.t("id");
    sw::actions::finish_line::approve(&id,None,false).unwrap();settle(&id);
    sw::land::cleanup_job(&id,"merged");
    assert!(task(&id).b("cleaned"));
    w.script(json!({"worker":[{"status":"working"}]}));
    delegate(&id);let c=settle(&id);
    assert_eq!(c.s("stage"),"working","new round did not start: stage={} msg={} failure={}",c.s("stage"),c.s("msg"),c.at("failure").s("title"));
    set_fake(None);
}
