//! The band over the list, under the filter row: the projects you hold (shown/projects.rs), each its name, how many open
//! tasks it has and where its most urgent ones stand (`garden 7 要対応1`), most urgent first. Never in the archive, and
//! only where the list keeps room for its rows: a sixth of it at most, what does not fit counted at the end (`+3`).
use crate::board::state::State;
use crate::shown::ink::{Ink, Inked};
use crate::shown::projects::says;
use crate::shown::states::group_ink;

/// What the band's first line starts with.
pub const LEAD: &str = "プロジェクト";

/// Whether the band shows at all, on the web board too: out of the archive, with two projects or more, or one that an
/// entry of `board.projects` names (a repo's own alone is what the header names already).
pub fn shown(s: &State) -> bool { !s.archive_view && (s.bands.len() >= 2 || s.bands.iter().any(|b| b.named)) }
/// The band in `w` cells, for a list part of `h` lines: none where it does not show. A project's count and word are
/// never cut, only a name too long for a line of its own; on the last line it may take, room stays for `+3`.
pub fn band_lines(s: &State, w: usize, h: usize) -> Vec<Inked> {
    if !shown(s) || h < 12 || w < 40 { return vec![]; }
    let max = (h / 6).max(1);
    let lead = Inked::plain(" ").add(Inked::of(LEAD, Ink::Muted)).add(Inked::plain("  "));
    let pad = lead.width();
    let mut lines = vec![lead];
    for (k, b) in s.bands.iter().enumerate() {
        let name = Inked::plain(b.name.clone()).bold();
        let tail = Inked::of(format!(" {}", b.n), Ink::Muted).add(Inked::of(format!(" {}", says(b)), group_ink(b.at)));
        loop {
            let at = lines.len() - 1;
            let empty = lines[at].width() <= pad;
            let gap = if empty { 0 } else { 3 };
            let keep = if lines.len() == max && k + 1 < s.bands.len() { 5 } else { 0 };
            let room = w.saturating_sub(lines[at].width() + gap + keep);
            if empty || name.width() + tail.width() <= room {
                lines[at] = lines[at].clone().add(Inked::spaces(gap)).add(name.cut(room.saturating_sub(tail.width()).max(2))).add(tail);
                break;
            }
            if lines.len() < max { lines.push(Inked::spaces(pad)); continue; }
            lines[at] = lines[at].clone().add(Inked::of(format!("  +{}", s.bands.len() - k), Ink::Muted));
            return lines;
        }
    }
    lines
}
