//! A red message under a task, whole at every width: it wraps as far as it needs, the list counts the lines it takes,
//! you move and scroll past it as past any row, and in a low window the rows you are not on give way first.
use crate::common;
use common::board::{open, Board};
use common::*;
use sw::json::*;
use sw::actions::filing::{report, Report};

const END: &str = "最後の一文字まで読める。";
/// Long in both kinds of text: words a line can break between, and Japanese that has no spaces at all.
fn long() -> String { format!("10/21 に発行した今の gh トークンも {}{END}", (1..=15).map(|n| format!("word{n:02} 長い説明の{n}番目")).collect::<Vec<_>>().join(" ")) }
fn asks(id: &str, msg: &str) { report(id, Report { state: "blocked".into(), kind: "question".into(), msg: msg.into(), ..Default::default() }).unwrap(); }
/// The list's words with what lines add taken out: the indents, and any space where a line broke.
fn joined(lines: &[String]) -> String { lines.join("").chars().filter(|c| !c.is_whitespace() && *c != '▌').collect() }
fn bare(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }
/// A board with one long question, the row after it, and plenty more to move through.
fn asked(rows: usize) -> (World, Board, String) {
    let w = world();
    let id = w.add("Long question");
    asks(&id, &long());
    for n in 1..=12 { w.add(&format!("Task number {n}")); }
    let mut s = open(&w.root, rows); s.s.side = false; s.load();
    (w, s, id)
}

#[test]
fn a_long_red_message_shows_whole_at_every_width_and_the_rows_around_it_stay_whole() {
    let (_w, mut s, id) = asked(60);
    s.press(&["down", "down"]); // off the row: this is how it stands in the list, not only under the cursor
    for cols in [200, 160, 120, 100, 80, 79, 60, 44] {
        let lines = s.screen(cols);
        assert_eq!(lines.len(), 60, "{cols} columns");
        assert!(lines.iter().all(|l| sw::text::width(l) <= cols), "{cols} columns:\n{}", lines.join("\n"));
        assert!(joined(&lines).contains(&bare(&long())), "{cols} columns, the whole message:\n{}", lines.join("\n"));
        assert!(!lines.iter().any(|l| l.ends_with('…')), "{cols} columns, nothing cut:\n{}", lines.join("\n"));
        assert!(lines.iter().any(|l| l.contains("Long question")), "{cols} columns, the name stays");
        assert!(lines.iter().any(|l| l.contains("Task number 1")), "{cols} columns, the next row stays");
        let r = s.s.items.iter().find(|r| r.s("id") == id).unwrap().clone();
        let at = s.s.rows.iter().position(|x| sw::board::update::list::step_id(x).as_deref() == Some(id.as_str())).unwrap();
        let width = s.s.list_width - sw::board::update::depth::indents(&s.s.rows, s.s.list_width)[at];
        assert_eq!(sw::board::view::row_wrap::row_height(&s.s, &r, width, cols < 80), s.s.list_ids.iter().filter(|x| **x == id).count(), "{cols} columns: the height the fold counts with is the height drawn");
    }
}

#[test]
fn a_short_red_message_and_what_a_working_task_says_stay_as_they_were() {
    let w = world();
    let short = w.add("Short question"); asks(&short, "Which one?");
    let mut s = open(&w.root, 40); s.s.side = false; s.load();
    let lines = s.screen(120);
    let at = lines.iter().position(|l| l.contains("Short question")).unwrap();
    assert!(lines[at + 1].contains("Which one?"), "{}", lines.join("\n"));
    assert!(!lines[at + 2].contains("Which one?"));
}

#[test]
fn you_move_and_scroll_past_a_long_red_message_and_the_selected_row_stays_whole_and_lit() {
    for (cols, rows) in [(120, 24), (120, 40), (80, 30), (60, 30)] {
        let (_w, mut s, id) = asked(rows);
        let ids = s.s.steps.clone();
        let visit = |s: &mut Board, what: &str| {
            let at = s.s.sel.clone();
            let lines = s.screen(cols);
            let lit = lines.iter().filter(|l| l.starts_with('▌')).count();
            assert_eq!(lines.len(), rows, "{cols}x{rows} {what}");
            let on = s.s.list_ids.iter().filter(|x| **x == at).count();
            assert!(on >= 1, "{cols}x{rows} {what}: {at} is in view:\n{}", lines.join("\n"));
            assert_eq!(lit, on, "{cols}x{rows} {what}: every line of the row is lit, and only those");
        };
        s.s.sel = id.clone();
        visit(&mut s, "on the question");
        for n in 0..ids.len() { s.press(&["down"]); visit(&mut s, &format!("down {n}")); }
        for n in 0..ids.len() { s.press(&["up"]); visit(&mut s, &format!("up {n}")); }
    }
}

#[test]
fn the_wheel_scrolls_a_long_red_message_by_its_whole_row() {
    let (_w, mut s, id) = asked(24);
    s.screen(120);
    for by in [1, 1, 1, 1, 1, -1, -1, -1, -1, -1] {
        s.wheel(5, by);
        let lines = s.screen(120);
        assert_eq!(lines.len(), 24);
        let shown = s.s.list_ids.iter().filter(|x| **x == id).count();
        assert!(shown == 0 || lines.iter().any(|l| l.contains("Long question")), "never the message without its name:\n{}", lines.join("\n"));
        assert!(shown == 0 || lines.iter().any(|l| l.contains("最後の一文字")), "never its name without the message's end:\n{}", lines.join("\n"));
    }
}

#[test]
fn a_click_on_any_line_of_a_red_message_picks_its_task() {
    let (_w, mut s, id) = asked(40);
    let lines = s.screen(120);
    let last = lines.iter().position(|l| l.contains("最後の一文字")).unwrap();
    s.press(&["down", "down"]); s.screen(120);
    s.click(last as u16); s.screen(120);
    assert_eq!(s.s.sel, id);
}

#[test]
fn in_a_low_window_the_row_you_are_not_on_gives_way_and_the_selected_one_never_runs_past_the_list() {
    for cols in [120, 60] {
        let (_w, mut s, id) = asked(14);
        asks(&id, &long().repeat(4)); s.load();
        s.s.sel = id.clone();
        let on = s.screen(cols);
        assert_eq!(on.len(), 14);
        assert!(s.s.list_ids.iter().filter(|x| **x == id).count() < s.s.list_room, "{cols}: a line stays for what is not shown:\n{}", on.join("\n"));
        assert!(on.iter().any(|l| l.contains("Long question")), "{cols}: its name shows:\n{}", on.join("\n"));
        assert!(on.iter().any(|l| l.ends_with('…')), "{cols}: a cut says so:\n{}", on.join("\n"));
        // Off the row, it gives up half the list.
        s.press(&["down"]);
        let off = s.screen(cols);
        assert!(s.s.list_ids.iter().filter(|x| **x == id).count() <= (s.s.list_room / 2).max(2) + 2, "{cols}, its name, its share and its time:\n{}", off.join("\n"));
    }
}
