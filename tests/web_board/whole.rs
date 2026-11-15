//! A task under "needs you" says in red what it waits for you about, and the web board shows all of it, wrapped to the
//! list's width however narrow, as the terminal board does (row_wrap.rs); what other rows say keeps its two lines.
use super::*;

const LONG: &str = "HCM-213 の確認: 実測判定の保留上限をどうするか。A推奨: 3回まで保留して4回目で見送る。B: 上限なしで毎回本人に聞く。C: 1回で見送る。最後の一文字まで読める。";

#[test]
fn a_long_red_message_on_the_web_board_is_never_clamped_and_other_rows_are_as_they_were() {
    let w = world();
    let asks = w.add("Long question");
    sw::actions::filing::report(&asks, sw::actions::filing::Report { state: "blocked".into(), kind: "question".into(), msg: LONG.into(), ..Default::default() }).unwrap();
    let busy = w.add("Write notes");
    sw::actions::filing::report(&busy, sw::actions::filing::Report { state: "working".into(), msg: "Drafting".into(), progress: Some(json!(40)), ..Default::default() }).unwrap();
    let s = web(&w, "board", json!({}));
    let rows = rows_of(&s);
    let row = |id: &str| rows.iter().find(|r| r.s("id") == id).unwrap().clone();
    let (red, other) = (row(&asks), row(&busy));
    assert_eq!(said(red.at("says")), LONG, "every word of it is sent to the page");
    assert!(red.b("whole"), "its red message is drawn whole");
    assert!(!other.b("whole"), "a working task's line keeps its two lines");
    let css = include_str!("../../crates/web/src/webui/page/board.css");
    assert!(css.contains(".sw-row.is-whole .sw-says { display: block; overflow: visible; -webkit-line-clamp: none; }"));
    assert!(include_str!("../../crates/web/src/webui/page/list.js").contains(r#"${r.whole ? " is-whole" : ""}"#));
}
