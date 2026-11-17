//! The band over the list: the projects you hold (`board.projects`), each with how many open tasks it has and where its
//! most urgent ones stand, under the filter row and over the first heading; on the web board the same
//! (shown/projects.rs, board/view/bands.rs).
use crate::common;
use common::board::{open, web, Board};
use common::{world_with, Opts};
use serde_json::json;
use sw::actions::filing::{report, Report};
use sw::actions::settings::{settings_command, Where};
use sw::json::*;
use sw::shown::projects::{bands, project_of, says, Band};

fn v(a: &[&str]) -> Vec<String> { a.iter().map(|s| s.to_string()).collect() }
fn row(id: &str, title: &str, state: &str) -> Obj { obj(json!({ "id": id, "title": title, "state": state, "stage": if state == "working" { "working" } else { "todo" } })) }
fn say(id: &str, state: &str) { report(id, Report { state: state.into(), msg: "".into(), ..Default::default() }).unwrap(); }
fn screen(s: &mut Board, cols: usize) -> String { s.screen(cols).join("\n") }
fn line(text: &str, p: &str) -> usize { text.lines().position(|l| l.contains(p)).unwrap_or_else(|| panic!("{p}: {text}")) }

#[test]
fn a_task_is_under_the_first_project_with_a_word_that_is_its_repo_or_in_its_title_else_under_its_repo() {
    let rules = v(&["skills=skill|cleanup", "garden=Lawn", "home", "tools=shed"]);
    let of = |id: &str, title: &str| project_of(&rules, &row(id, title, "todo"));
    assert_eq!(of("tools/1", "Clean up the SKILL sheet"), "skills", "the first entry wins, letters in any case");
    assert_eq!(of("tools/2", "Mow the lawn"), "garden");
    assert_eq!(of("home/3", "Paint the door"), "home", "a name alone is its own word, and a word may be the repo");
    assert_eq!(of("tools/4", "Oil the saw"), "tools");
    assert_eq!(of("homestead/9", "Oil the saw"), "homestead", "the repo is matched whole: home is not homestead");
    assert_eq!(of("proj/5", "Pay the bill"), "proj", "nothing matches: its repo");
    assert_eq!(project_of(&[], &row("proj/6", "Lawn", "todo")), "proj", "with no projects set, by repo");
    // Another machine's row by its own id there.
    assert_eq!(project_of(&rules, &obj(json!({ "id": "vm-a:proj/7", "rid": "home/7", "title": "Fix it", "state": "todo" }))), "home");
}

#[test]
fn the_band_counts_the_open_tasks_and_says_the_most_urgent_place_first() {
    let rules = v(&["skills=skill", "garden=lawn"]);
    let rows = [
        row("proj/1", "Skill one", "todo"), row("proj/2", "Skill two", "todo"),
        row("proj/3", "Lawn edge", "working"), obj(json!({ "id": "proj/4", "title": "Lawn seed", "state": "blocked", "stage": "asks" })), row("proj/5", "Lawn feed", "todo"),
        row("proj/6", "Pay the bill", "working"),
        row("proj/7", "Skill three", "done"), obj(json!({ "id": "proj/8", "title": "Skill four", "state": "todo", "archivedAt": 1 })),
    ];
    let b = bands(&rules, rows.iter(), 0);
    let band = |name: &str, n: usize, at: &'static str, at_n: usize| Band { name: name.into(), n, at, at_n };
    assert_eq!(b, vec![band("garden", 3, "needs you", 1), band("proj", 1, "working", 1), band("skills", 2, "to do", 2)], "done and archived ones are not counted");
    assert_eq!(b.iter().map(says).collect::<Vec<_>>(), ["要対応1", "作業中", "未着手"], "a count where not all of them stand there");
}

fn garden() -> common::World {
    let w = world_with(Opts { repo: true, settings: json!({ "board.projects": "skills=skill|cleanup garden=lawn|fence" }), ..Default::default() });
    for t in ["Skill sheet one", "Skill sheet two", "Cleanup the drive"] { w.add(t); }
    let fence = w.add("Paint the fence");
    say(&fence, "working");
    let lawn = w.add("Mow the lawn");
    say(&lawn, "blocked");
    w.add("Pay the bill");
    w
}

#[test]
fn on_the_board_the_band_stands_under_the_filter_row_over_the_first_heading_whatever_the_filter() {
    let w = garden();
    let mut s = open(&w.root, 40); s.s.side = false;
    let text = screen(&mut s, 100);
    let band = line(&text, "プロジェクト");
    assert_eq!(band, line(&text, "All 6 ") + 1, "{text}");
    assert!(band < line(&text, "▾ needs you"), "{text}");
    let l = text.lines().nth(band).unwrap();
    assert!(l.contains("garden 2 要対応1   skills 3 未着手   proj 1 未着手"), "most urgent first, then the one with more: {l}");
    // Its words in the colour of where they stand.
    let page = web(&w, "board", json!({}));
    let first = &page.at("list").arr("projects")[0];
    assert_eq!((first.s("name"), first.i("n"), first.s("says"), first.s("color")), ("garden", 2, "要対応1", "red"), "{page}");
    // Another filter shows fewer rows, not fewer projects.
    while s.s.show != 3 { s.press(&["tab"]); }
    assert!(screen(&mut s, 100).contains("garden 2 要対応1   skills 3 未着手   proj 1 未着手"));
    // The setting changed: the next read shows it. By repo, every task is under the one: no band.
    let set = |v: &str| settings_command("set", &["set".into(), "board.projects".into()], &Where { task: None, repo: None, automation: None, coord: None }, Some(json!(v))).unwrap();
    set("garden=lawn|fence|skill");
    s.load();
    assert!(screen(&mut s, 100).contains("garden 4 要対応1   proj 2 未着手"), "{}", screen(&mut s, 100));
    set("none");
    s.load();
    assert!(!screen(&mut s, 100).contains("プロジェクト"));
}

#[test]
fn one_project_has_no_band() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    w.add("Pay the bill");
    w.add("Fix the tap");
    let mut s = open(&w.root, 40);
    assert!(!screen(&mut s, 100).contains("プロジェクト"), "every task under the one repo: the header names it");
    assert_eq!(web(&w, "board", json!({})).at("list").arr("projects").len(), 0);
}

#[test]
fn a_band_too_long_for_its_room_says_how_many_are_left() {
    // Twelve projects at 60 columns: as many lines as a sixth of the list part, and what is left counted.
    let names: Vec<String> = (1..=12).map(|n| format!("p{n:02}")).collect();
    let w = world_with(Opts { repo: true, settings: json!({ "board.projects": names.join(" ") }), ..Default::default() });
    for n in &names { w.add(&format!("{n} chore")); }
    let mut s = open(&w.root, 30); s.s.side = false;
    let text = screen(&mut s, 60);
    let first = line(&text, "プロジェクト");
    let band: Vec<&str> = text.lines().skip(first).take_while(|l| !l.contains("▾ ")).collect();
    assert!(band.len() >= 2 && band.iter().all(|l| sw::text::width(l) <= 60), "{text}");
    assert!(band.last().unwrap().trim_end().ends_with(&format!("+{}", 12 - band.iter().map(|l| l.matches(" 未着手").count()).sum::<usize>())), "{text}");
    assert_eq!(web(&w, "board", json!({})).at("list").arr("projects").len(), 12, "the web board's band wraps: all of them");
}
