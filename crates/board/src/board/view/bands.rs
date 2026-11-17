//! The band over the list, under the filter row: the projects you hold (shown/projects.rs), each its name, how many open
//! tasks it has and where its most urgent ones stand (`応募運用 7 要対応1`), most urgent first. Only with two projects
//! or more, never in the archive, and only where the list keeps room for its rows: a sixth of it at most, what does
//! not fit counted at the end (`+3`).
use crate::board::state::State;
use crate::shown::ink::{Ink, Inked};
use crate::shown::projects::says;
use crate::shown::states::group_ink;

/// What the band's first line starts with.
pub const LEAD: &str = "プロジェクト";

/// Whether the band shows at all, on the web board too: two projects or more, out of the archive.
pub fn shown(s: &State) -> bool { !s.archive_view && s.bands.len() >= 2 }
/// The band in `w` cells, for a list part of `h` lines: none where it does not show.
pub fn band_lines(s: &State, w: usize, h: usize) -> Vec<Inked> {
    if !shown(s) || h < 12 || w < 24 { return vec![]; }
    let max = (h / 6).max(1);
    let lead = Inked::plain(" ").add(Inked::of(LEAD, Ink::Muted)).add(Inked::plain("  "));
    let pad = lead.width();
    let mut lines = vec![lead];
    for (k, b) in s.bands.iter().enumerate() {
        let chip = Inked::plain(b.name.clone()).bold().add(Inked::of(format!(" {}", b.n), Ink::Muted)).add(Inked::of(format!(" {}", says(b)), group_ink(b.at)));
        let last = lines.len() - 1;
        let empty = lines[last].width() <= pad;
        let gap = if empty { 0 } else { 3 };
        // On the last line the band may take, room is kept to say how many are left after this one.
        let keep = if lines.len() == max && k + 1 < s.bands.len() { 5 } else { 0 };
        if empty || lines[last].width() + gap + chip.width() + keep <= w { lines[last] = lines[last].clone().add(Inked::spaces(gap)).add(chip).cut(w); }
        else if lines.len() < max { lines.push(Inked::spaces(pad).add(chip).cut(w)); }
        else { lines[last] = lines[last].clone().add(Inked::of(format!("  +{}", s.bands.len() - k), Ink::Muted)).cut(w); break; }
    }
    lines
}
