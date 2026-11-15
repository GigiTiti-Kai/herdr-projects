//! A row's kind chip (`[調査]`, `[自動]`, `[運用]`) before its name and whom it was handed to (`[委任先 PRJ-26・作業中]`)
//! after it, both read off what the task already has; their dim ink; and their lines in the `?` menu (view/kind.rs).
use crate::common;
use common::board::open;
use common::{short, world};
use serde_json::{json, Value};
use std::sync::Arc;
use sw::board::state::{Rec, State};
use sw::board::view::fields::{bare_title, name_ink, row_title};
use sw::board::view::kind::{chip, handed};
use sw::json::*;
use sw::shown::ink::Ink;

fn rec(v: Value) -> Rec { Arc::new(obj(v)) }
fn state(items: Vec<Rec>) -> State {
    let mut s = State::new(Default::default(), Default::default());
    s.data.keys = [("proj", "PRJ"), ("ops", "OPS"), ("docs", "DOC")].iter().map(|(r, k)| (r.to_string(), k.to_string())).collect();
    s.data.items = items;
    s
}

#[test]
fn the_chip_is_read_off_research_a_worker_setting_a_worktree_or_its_branch_and_is_運用_otherwise() {
    let kind = |v: Value| chip(&obj(v));
    assert_eq!(kind(json!({ "id": "ops/138", "title": "Kept by hand", "settings": {} })), "[運用]");
    assert_eq!(kind(json!({ "id": "proj/26", "title": "Run it", "settings": { "worker.agent": "claude" } })), "[自動]");
    assert_eq!(kind(json!({ "id": "docs/70", "title": "Guard", "settings": {}, "worktree": "/w/docs/work-70" })), "[自動]");
    // Finished: the worktree is gone, its branch stays.
    assert_eq!(kind(json!({ "id": "proj/25", "title": "Done one", "settings": { "merge.by": "you" }, "worktree": null, "branch": "work/25-x" })), "[自動]");
    // Research only wins, as `[research]` says; another setting under worker. too.
    assert_eq!(kind(json!({ "id": "proj/3", "title": "Look", "research": true, "worktree": "/w/p/3" })), "[調査]");
    assert_eq!(kind(json!({ "id": "proj/4", "title": "Pick", "research": false, "settings": { "worker.model": "x" } })), "[自動]");
}

#[test]
fn a_task_its_message_hands_to_another_is_named_with_where_that_one_stands() {
    let target = rec(json!({ "id": "proj/26", "title": "Run it", "state": "working", "stage": "checking" }));
    let ready = rec(json!({ "id": "ops/246", "title": "Map", "state": "idle", "stage": "todo" }));
    let s = state(vec![target, ready]);
    // A row's own message is `raw` (its `msg` is the shown note).
    let row = |raw: &str| obj(json!({ "id": "ops/201", "title": "[順番待ち] Wait for it", "raw": raw, "msg": "shown note" }));
    assert_eq!(handed(&s, &row("PRJ-26に委任")), "[委任先 PRJ-26・作業中]");
    assert_eq!(handed(&s, &row("決めた: 案A。OPS-246・OPS-247に委任")), "[委任先 OPS-246・未着手, OPS-247]", "one not on the board has no state");
    assert_eq!(handed(&s, &row("ZZZ-38(elsewhere/38)に仕様ごと委任済み")), "", "a key no repo has names nothing");
    assert_eq!(handed(&s, &row("PRJ-26(proj/26)に仕様ごと委任済み")), "[委任先 PRJ-26・作業中]");
    // Numbers that are not what it was handed to name nothing.
    assert_eq!(handed(&s, &row("S2→S1 の順に(S2=DOC-7, S1=OPS-42)。S2のみ委任")), "");
    assert_eq!(handed(&s, &row("PRJ-26 を見ている")), "");
    assert_eq!(handed(&s, &row("別のボードに委任")), "");
    // Never itself, and not another machine's row (its keys are that machine's).
    assert_eq!(handed(&s, &obj(json!({ "id": "ops/201", "raw": "OPS-201に委任" }))), "");
    assert_eq!(handed(&s, &obj(json!({ "id": "far:ops/201", "rid": "ops/201", "host": "far", "raw": "PRJ-26に委任" }))), "");
}

#[test]
fn the_row_reads_its_chip_first_and_its_tags_after_all_dimmed_and_research_rows_keep_their_tag() {
    let target = rec(json!({ "id": "proj/26", "title": "Run it", "state": "working", "stage": "checking", "settings": { "worker.agent": "claude" } }));
    let s = state(vec![target.clone()]);
    let ledger = obj(json!({ "id": "ops/201", "title": "[順番待ち] Wait for it", "raw": "PRJ-26に委任", "settings": {} }));
    let title = row_title(&s, &ledger);
    assert_eq!(title, "[運用] [順番待ち] Wait for it [委任先 PRJ-26・作業中]");
    assert_eq!(bare_title(&s, &ledger), "[順番待ち] Wait for it [委任先 PRJ-26・作業中]", "the web board has the chip on its own");
    let inked = name_ink(&ledger, &title);
    let muted: String = inked.0.iter().filter(|p| p.c == Ink::Muted).map(|p| p.t.as_str()).collect();
    let plain: String = inked.0.iter().filter(|p| p.c == Ink::Plain).map(|p| p.t.as_str()).collect();
    assert_eq!((muted.as_str(), plain.as_str()), ("[運用]  [委任先 PRJ-26・作業中]", "[順番待ち] Wait for it"));
    assert_eq!(row_title(&s, &target), "[自動] Run it");
    let research = obj(json!({ "id": "proj/7", "title": "Look into it", "research": true }));
    assert_eq!(row_title(&s, &research), "[調査] Look into it [research]");
    let inked = name_ink(&research, &row_title(&s, &research));
    assert_eq!(inked.0.iter().filter(|p| p.c == Ink::Plain).map(|p| p.t.as_str()).collect::<String>(), "Look into it");
}

#[test]
fn on_the_board_a_ledger_row_shows_its_chip_and_whom_it_handed_to_whole_also_where_its_name_is_cut() {
    let w = world();
    let target = w.add("Move the logs to their new place");
    let ledger = w.add("[順番待ち] ログの保存先を新しい場所に移す");
    let n = short(&target);
    sw::store::tasks::patch(&ledger, |t| { t.put("msg", format!("{n}に委任")); }).unwrap();
    let mut s = open(&w.root, 40);
    let text = s.screen(120).join("\n");
    let line = text.lines().find(|l| l.contains("[運用] [順番待ち]")).unwrap_or_default();
    // With the task panel open the name is cut, and the tag of whom it was handed to stays whole.
    assert!(line.contains("[運用] [順番待ち] ログの保存先を") && line.contains(&format!("… [委任先 {n}・未着手]")), "{text}");
    s.press(&["esc"]);
    let line = s.screen(160).into_iter().find(|l| l.contains("[運用] [順番待ち]")).unwrap_or_default();
    assert!(line.contains(&format!("[運用] [順番待ち] ログの保存先を新しい場所に移す [委任先 {n}・未着手]")), "{line}");
}

#[test]
fn the_menu_says_what_the_chip_and_the_handed_tag_mean_one_line_each() {
    let w = world();
    w.add("Fix the page");
    let mut s = open(&w.root, 60);
    s.rows = 80; s.press(&["?"]);
    let text = s.screen(160).join("\n");
    assert!(text.contains("[自動]  ") && text.contains("[自動] an agent works it in a worktree, [調査] research only, [運用] worked by hand or by a resident seat"), "{text}");
    assert!(text.contains("handed to another task (its message says so): that task's number and where it stands"), "{text}");
}
