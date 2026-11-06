//! Dragging over the board selects inside one part of the screen: what is copied is the part's text, without the
//! edges of a box, the cells that fill a line and what stands beside it. The drag goes through the real selection, on
//! the cells the drawer wrote; it stops short of letting go, which only sends the same text to the terminal.
use crate::common;
use common::board::{open, Board};
use common::{world_with, Opts};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use serde_json::json;
use sw::board::run::{draw, mouse::Selecting};

fn at(kind: MouseEventKind, (x, y): (usize, usize)) -> MouseEvent { MouseEvent { kind, column: x as u16, row: y as u16, modifiers: KeyModifiers::NONE } }
/// The drag on the screen as it stands, and the highlighted screen beside what it would copy.
fn drag(b: &mut Board, from: (usize, usize), to: (usize, usize)) -> (String, Buffer) {
    let sc = sw::board::view::screen(&mut b.s, &b.theme, b.cols, b.rows);
    let mut buf = Buffer::empty(Rect::new(0, 0, b.cols as u16, b.rows as u16));
    draw::draw(&b.theme, &sc.lines, &mut buf);
    let mut sel = Selecting::default();
    sel.capture(&buf, &b.s.panes);
    sel.mouse(&at(MouseEventKind::Down(MouseButton::Left), from), 0);
    sel.mouse(&at(MouseEventKind::Drag(MouseButton::Left), to), 0);
    sel.paint(&mut buf);
    (sel.text(), buf)
}
fn row_of(lines: &[String], text: &str) -> usize { lines.iter().position(|l| l.contains(text)).unwrap_or_else(|| panic!("no {text} in\n{}", lines.join("\n"))) }
fn chat_board(side: bool) -> (common::World, Board) {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude" }), ..Default::default() });
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", w.base.join("home-coord")) };
    let mut b = open(&w.root, 40);
    b.s.side = side;
    b.s.data.can_dispatch = true; b.s.chat_open = true;
    b.s.data.coord = sw::coordinator::look::Look { state: "idle".into(), readable: true, said: "Run these:\n\n  cd /home/hadas/projects/herdr\n  git switch dev".into(), ..Default::default() };
    (w, b)
}

#[test]
fn a_drag_over_the_chat_copies_its_text_without_the_box_or_the_cells_that_fill_it() {
    let (_w, mut b) = chat_board(false);
    let lines = b.lines();
    let (top, last) = (row_of(&lines, "Run these:"), row_of(&lines, "git switch dev"));
    assert!(lines[top].starts_with('│') && lines[top].ends_with('│'), "{}", lines[top]);
    // From the left edge of the box to past its right one, as a hand does it.
    let (text, buf) = drag(&mut b, (0, top), (119, last));
    assert_eq!(text, "Run these:\n\n  cd /home/hadas/projects/herdr\n  git switch dev");
    let lit = |x: u16, y: usize| buf[(x, y as u16)].modifier.contains(Modifier::REVERSED);
    assert!(!lit(0, last) && !lit(119, top) && lit(3, last), "the highlight stays off the edges");
    // Past the box, up or down: it stops at the text.
    let edge = (0..top).rev().find(|n| lines[*n].starts_with('╭')).unwrap();
    assert_eq!(drag(&mut b, (5, last), (5, edge)).0, "Run these:\n\n  cd /home/hadas/projects/herdr\n  g");
    let hints = b.rows - 1;
    let (down, _) = drag(&mut b, (3, top), (119, hints));
    assert!(down.starts_with("Run these:") && !down.contains('│') && !down.contains('╰') && !down.contains("all keys"), "{down}");
}

#[test]
fn a_drag_over_the_task_panel_leaves_the_list_and_the_rule_out() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    let id = w.add("Panel task");
    let mut b = open(&w.root, 40);
    b.s.sel = id; b.load();
    let lines = b.lines();
    let (top, last) = (row_of(&lines, "State"), row_of(&lines, "Next "));
    let list = lines[top].find('│').map(|n| lines[top][..n].chars().count()).unwrap();
    let (text, _) = drag(&mut b, (list, top), (119, last));
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), last - top + 1, "{text}");
    assert!(rows[0].starts_with("State ") && rows.iter().all(|l| !l.contains('│') && !l.starts_with(' ') && l.trim_end() == *l), "{text}");
    // The list beside it: down its lines, with neither the rule nor the panel.
    let (left, _) = drag(&mut b, (1, 1), (119, 2));
    assert!(!left.contains('│') && !left.contains("State"), "{left}");
}

#[test]
fn a_drag_on_a_window_of_eight_cells_does_not_panic() {
    let (_w, mut b) = chat_board(false);
    b.cols = 8;
    let last = b.rows - 2;
    drag(&mut b, (0, 0), (7, last));
    drag(&mut b, (7, last), (0, 0));
}
