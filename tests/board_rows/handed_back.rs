//! Whom a row no agent of the board's works handed to, when its message only names the task and that task names it back
//! (a coordinator writes its parent's number into what it hands on); none it took back; and the short tag: those not
//! done with where they stand, the done ones counted, all done in the colour of done (view/kind.rs).
use serde_json::{json, Value};
use std::sync::Arc;
use sw::board::state::{Rec, State};
use sw::board::view::fields::{name_ink, row_title};
use sw::board::view::kind::handed;
use sw::json::*;
use sw::shown::ink::Ink;

fn rec(v: Value) -> Rec { Arc::new(obj(v)) }
fn state(items: Vec<Rec>) -> State {
    let mut s = State::new(Default::default(), Default::default());
    s.data.keys = [("proj", "PRJ"), ("ops", "OPS")].iter().map(|(r, k)| (r.to_string(), k.to_string())).collect();
    s.data.items = items;
    s
}
/// OPS-201, at work by hand, saying `raw`.
fn ledger(raw: &str) -> Obj { obj(json!({ "id": "ops/201", "title": "Keep the ledger", "state": "working", "stage": "progress", "raw": raw, "msg": raw })) }
fn back(id: &str, state: &str, stage: &str, title: &str) -> Rec { rec(json!({ "id": id, "title": title, "description": "", "state": state, "stage": stage })) }

#[test]
fn a_task_its_message_names_that_names_it_back_is_whom_it_handed_to() {
    let s = state(vec![
        back("proj/39", "idle", "todo", "Try GitHub Actions (OPS-201)"),
        back("proj/26", "working", "checking", "Run it"),
        back("proj/27", "idle", "todo", "Split OPS-2010 off"),
        rec(json!({ "id": "proj/28", "title": "Map", "description": "From OPS-201, P3.", "state": "idle", "stage": "todo" })),
    ]);
    assert_eq!(handed(&s, &ledger("PRJ-39で試行中")), "[委任先 PRJ-39・未着手]");
    assert_eq!(handed(&s, &ledger("PRJ-28を見る")), "[委任先 PRJ-28・未着手]", "by its description too");
    assert_eq!(handed(&s, &ledger("PRJ-26で試行中")), "", "one that does not name it back is no more than named");
    assert_eq!(handed(&s, &ledger("PRJ-27で試行中")), "", "OPS-2010 is not OPS-201");
    // Only on a row at work or waiting for you: a task to do that names one it waits for has not handed it anything.
    let todo = obj(json!({ "id": "ops/201", "title": "Keep the ledger", "state": "idle", "stage": "todo", "raw": "PRJ-39の後" }));
    assert_eq!(handed(&s, &todo), "");
    let asks = obj(json!({ "id": "ops/201", "title": "Keep the ledger", "state": "blocked", "kind": "question", "stage": "progress", "raw": "PRJ-39で試すか?" }));
    assert_eq!(handed(&s, &asks), "[委任先 PRJ-39・未着手]");
}

#[test]
fn none_it_took_back_is_named_where_the_number_or_the_hand_over_is_followed_by_a_no() {
    let s = state(vec![back("proj/39", "idle", "todo", "Try GitHub Actions (OPS-201)"), back("proj/40", "idle", "todo", "OPS-201 P2")]);
    for no in [
        "PRJ-39に委任を取り消した。手で続ける。", "PRJ-39(merge.by you)に委任を取り消した。", "PRJ-39・OPS-2(v1.2)に委任を取り消した。",
        "PRJ-39にv1.2で委任しない。手で続ける", "PRJ-39(auto-teikeikun/39)は取り消した。手で続ける", "PRJ-39への委任は取り消した。手で続ける",
        "PRJ-39を委任しない。手で続ける", "PRJ-39は取り消した。手で続ける", "PRJ-39に委任済み、手では回さない", "PRJ-39に委任をやめた。その後 PRJ-39 で再開",
    ] {
        assert_eq!(handed(&s, &ledger(no)), "", "{no}");
    }
    // The takings back a hand-over by `に委任` already names nothing for, now with a task that names it back.
    for no in ["PRJ-39に委任しない。手で続ける", "PRJ-39に委任せず手で回す", "PRJ-39に委任しません", "PRJ-39に委任していません。手で続ける", "PRJ-39に委任してはいけない。手で続ける", "PRJ-39に委任することは拒否された。手で続ける", "PRJ-39に委任しなかった", "PRJ-39に委任を取り消した", "PRJ-39に委任をやめた", "PRJ-39に委任するのは禁止", "PRJ-39に委任することは、拒否された。手で続ける", "PRJ-39に委任することは、禁止されている。手で続ける", "PRJ-39に委任済み、手では回さない"] {
        assert_eq!(handed(&s, &ledger(no)), "", "{no}");
    }
    assert_eq!(handed(&s, &ledger("PRJ-39に委任した。返事はまだない")), "[委任先 PRJ-39・未着手]", "the no is in the next sentence");
    assert_eq!(handed(&s, &ledger("OPS-1に委任しない。PRJ-39で試行中")), "[委任先 PRJ-39・未着手]", "the no is another number's");
    assert_eq!(handed(&s, &ledger("PRJ-40は取り消した。PRJ-39で試行中")), "[委任先 PRJ-39・未着手]");
    // What is left: a `.` between the number and its no ends the sentence there.
    assert_eq!(handed(&s, &ledger("PRJ-39 v1.2 は取り消した")), "[委任先 PRJ-39・未着手]");
}

#[test]
fn those_not_done_are_named_with_where_they_stand_the_done_ones_counted_and_all_done_is_in_the_colour_of_done() {
    let s = state(vec![
        back("proj/39", "working", "progress", "OPS-201 P1"), back("proj/40", "done", "done", "OPS-201 P2"), back("proj/41", "done", "done", "OPS-201 P3"),
    ]);
    assert_eq!(handed(&s, &ledger("P1=PRJ-39, P2=PRJ-40, P3=PRJ-41")), "[委任先 PRJ-39・作業中 +完了2]");
    assert_eq!(handed(&s, &ledger("P2=PRJ-40に委任, P3=PRJ-41")), "[委任先 2件すべて完了]", "by `に委任` and named back alike");
    let row = ledger("PRJ-40・PRJ-41に委任");
    let title = row_title(&s, &row);
    assert_eq!(title, "[運用] Keep the ledger [委任先 2件すべて完了]");
    let inked = name_ink(&row, &title);
    let ink = |c: Ink| inked.0.iter().filter(|p| p.c == c).map(|p| p.t.as_str()).collect::<String>();
    assert_eq!((ink(Ink::Teal).as_str(), ink(Ink::Muted).as_str()), ("[委任先 2件すべて完了]", "[運用]  "));
}
