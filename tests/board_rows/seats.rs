//! Line 2 of a row at work by hand: the seats `board.seats` names for it as herdr lists them now, or nothing where its
//! title says whom it was handed to, else エージェントなし; then its message with how long ago it was said and the panes it
//! names that herdr no longer has. A row that waits for you says エージェントなし as before (flow/seats.rs,
//! view/measured.rs).
use crate::common;
use common::board::{open, web, Board};
use common::{short, world, World};
use serde_json::{json, Value};
use sw::actions::filing::{report, Report};
use sw::json::*;
use sw::store::settings::change::set_value;
use sw::store::settings::lookup::Scope;

const NOW: i64 = 1_800_000_000_000;
fn say(id: &str, state: &str, msg: &str) { report(id, Report { state: state.into(), msg: msg.into(), ..Default::default() }).unwrap(); }
fn line(s: &mut Board, name: &str) -> String {
    let text = s.screen(220);
    let at = text.iter().position(|l| l.contains("[運用]") && l.contains(name)).unwrap_or_else(|| panic!("{name}: {}", text.join("\n")));
    text[at + 1].split('│').next().unwrap_or("").trim_matches(['▌', ' ']).to_string()
}
/// A pane of herdr's own in `cwd`, with an agent of `kind` that is `status`, named `name` ("" for none).
fn seat(w: &World, pane: &str, term: &str, cwd: &str, kind: &str, status: &str, name: &str) {
    let agent = if name.is_empty() { json!({ "kind": kind, "status": status }) } else { json!({ "kind": kind, "status": status, "name": name }) };
    w.patch(|st| st["panes"].as_array_mut().unwrap().push(json!({ "pane_id": pane, "workspace_id": "wEN", "tab_id": "wEN:t1", "terminal_id": term, "cwd": cwd, "agent": agent })));
}

#[test]
fn a_row_at_work_by_hand_says_the_seats_in_its_folder_then_how_old_its_report_is_and_which_panes_it_names_are_gone() {
    let w = world();
    sw::clock::set(Some(NOW));
    let root = w.root_str();
    let apply = w.add("[応募] Apply for the jobs");
    let other = w.add("Note the rota");
    say(&apply, "working", "wA9:p1 Hermes稼働中, wEN:p2 waits");
    say(&other, "working", "Asked about Friday");
    // With no setting, no seats: as before, with how long ago it was said.
    let mut s = open(&w.root, 40);
    assert_eq!(line(&mut s, "Apply for the jobs"), "エージェントなし · 報告0s前（wA9:p1・wEN:p2 は今は無い）: wA9:p1 Hermes稼働中, wEN:p2 waits");
    set_value("board.seats", &json!("proj=[応募]"), &Scope::you()).unwrap();
    s.load();
    assert_eq!(line(&mut s, "Apply for the jobs"), "席なし（proj） · 報告0s前（wA9:p1・wEN:p2 は今は無い）: wA9:p1 Hermes稼働中, wEN:p2 waits", "nobody sits there: said so, in red");
    seat(&w, "wEN:p1", "term_a", &root, "hermes", "working", "hermes-main");
    seat(&w, "wEN:p2", "term_b", &format!("{root}/sub"), "claude", "blocked", "asker");
    seat(&w, "wEN:p4", "term_c", &root, "hermes", "idle", "");
    seat(&w, "wEN:p5", "term_d", "/elsewhere", "hermes", "working", "far");
    // One the board started in that folder is no seat: by its terminal.
    seat(&w, "wEN:p6", "term_own", &root, "claude", "working", "sw-proj-9");
    sw::store::tasks::patch(&other, |t| { t.put("terms", json!(["term_own"])); }).unwrap();
    sw::clock::advance(26 * 3600 * 1000);
    s.load();
    let said = line(&mut s, "Apply for the jobs");
    assert_eq!(said, "席 proj: Hermes 2・Claude 1・作業中1(hermes-main)・入力待ち1(asker) · 報告1d前（wA9:p1 は今は無い）: wA9:p1 Hermes稼働中, wEN:p2 waits");
    // A row the setting does not name says it has no agent, and how old its report is.
    assert_eq!(line(&mut s, "Note the rota"), "エージェントなし · 報告1d前: Asked about Friday");
    // The web board says the same.
    let page = web(&w, "board", json!({}));
    let rows: Vec<Value> = page.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").to_vec()).collect();
    let row = rows.iter().find(|r| r.s("title").contains("Apply for the jobs")).unwrap_or_else(|| panic!("{page}"));
    let pieces = row.arr("says");
    assert_eq!(pieces.iter().map(|p| p.t("t")).collect::<String>(), said);
    assert_eq!(pieces.iter().filter(|p| p.s("c") == "red").map(|p| p.t("t")).collect::<String>(), "入力待ち1(asker)");
}

#[test]
fn a_row_whose_title_says_whom_it_handed_to_drops_エージェントなし_and_one_that_waits_for_you_keeps_it() {
    let w = world();
    sw::clock::set(Some(NOW));
    let ledger = w.add("Sort the leaks");
    let n = short(&ledger);
    let child = w.add(&format!("P1 of {n}"));
    let asks = w.add("Pick a floor");
    say(&ledger, "working", &format!("P1={}で試行中", short(&child)));
    say(&asks, "blocked", "Which floor first?");
    let mut s = open(&w.root, 40);
    let text = s.screen(220).join("\n");
    assert!(text.contains(&format!("Sort the leaks [委任先 {}・未着手]", short(&child))), "{text}");
    assert_eq!(line(&mut s, "Sort the leaks"), format!("報告0s前: P1={}で試行中", short(&child)));
    assert_eq!(line(&mut s, "Pick a floor"), "エージェントなし · Which floor first?");
}
