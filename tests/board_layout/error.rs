//! An error on the message line, whole: however long, it wraps to every line it needs (a result keeps to three), and
//! in a window too short for it the text box and its hints still stand whole on the window's last lines.
use crate::common;
use common::board::open;
use common::*;
use sw::shown::ink::Ink;

#[test]
fn a_long_error_wraps_whole_and_the_text_box_and_its_hints_still_stand() {
    let w = world();
    for i in 1..=30 { w.add(&format!("Task number {i}")); }
    let mut s = open(&w.root, 40); s.s.side = false;
    // About six lines at 120 columns: twice what a message used to keep.
    let words: Vec<String> = (1..=120).map(|i| format!("w{i:03}")).collect();
    s.s.flash(format!("The merge failed: {}. Nothing was merged.", words.join(" ")), Ink::Red);
    let lines = s.screen(120);
    assert_eq!(lines.len(), 40);
    let said = regex::Regex::new(r"\s+").unwrap().replace_all(&lines.join(" "), " ").into_owned();
    assert!(said.contains(&format!("The merge failed: {}. Nothing was merged. (esc dismisses)", words.join(" "))), "every word, and the key that dismisses it:\n{}", lines.join("\n"));
    assert!(regex::Regex::new(r"^[ ▌] \S.*( · |all keys)").unwrap().is_match(&lines[39]), "the hints are the last line: {:?}", lines[39]);
    assert!(lines.iter().all(|l| sw::text::width(l) <= 120));
    // A result that is not an error keeps to three lines.
    s.press(&["esc"]); assert!(!s.text().contains("w001"));
    s.s.flash(words.join(" "), Ink::Muted);
    let lines = s.screen(120);
    assert_eq!(lines.iter().filter(|l| l.contains(" w0") || l.contains(" w1")).count(), 3);
    // Longer than the window: what does not fit goes from its end; the text box and the hints stay.
    s.press(&["down"]);
    s.s.flash(format!("{} ", words.join(" ")).repeat(4), Ink::Red);
    for rows in [12, 8] {
        s.rows = rows;
        let lines = s.screen(120);
        assert_eq!(lines.len(), rows);
        assert!(regex::Regex::new(r"^[ ▌] \S.*( · |all keys)").unwrap().is_match(&lines[rows - 1]), "{rows} rows: the hints are the last line:\n{}", lines.join("\n"));
        assert!(lines.iter().any(|l| l.contains("w001")), "{rows} rows: the error's start shows:\n{}", lines.join("\n"));
        assert!(lines.iter().any(|l| regex::Regex::new(r"^(─+|╰─+╯)$").unwrap().is_match(l)), "{rows} rows: the text box shows:\n{}", lines.join("\n"));
    }
}
