//! Where each part of the screen stands, for a selection (run/mouse.rs): a drag that starts in a part keeps to the text
//! of that part, so a box's edges, the padding inside it and what stands beside it are not copied.
use crate::board::state::Pane;
use ratatui::layout::Rect;

/// Cells `x` and lines `y`, each from the first up to but not with the last.
fn rect(x: (usize, usize), y: (usize, usize)) -> Rect { Rect::new(x.0 as u16, y.0 as u16, x.1.saturating_sub(x.0) as u16, y.1.saturating_sub(y.0) as u16) }
/// The parts of a screen of `width` × `rows`. `top`: the lines of the list or the page. `list_w`: the width of the list
/// where the task panel stands beside it. `chat`: the lines of the framed box, and the cells inside it left and right.
pub fn of(width: usize, rows: usize, top: (usize, usize), list_w: Option<usize>, chat: Option<(usize, usize, usize)>) -> Vec<Pane> {
    let whole = |y| Pane { frame: rect((0, width), y), text: rect((0, width), y) };
    let mut out = vec![whole((0, top.0))];
    match list_w {
        Some(w) => {
            out.push(Pane { frame: rect((0, w + 1), top), text: rect((0, w), top) });
            // After the rule and its space, every line of the panel starts with a space (Kit::out).
            out.push(Pane { frame: rect((w + 1, width), top), text: rect((w + 4, width), top) });
        }
        None => out.push(whole(top)),
    }
    match chat {
        Some((from, to, pad)) => {
            out.push(whole((top.1, from)));
            out.push(Pane { frame: rect((0, width), (from, to)), text: rect((pad + 1, width.saturating_sub(pad + 1)), (from + 1, to.saturating_sub(1))) });
            out.push(whole((to, rows)));
        }
        None => out.push(whole((top.1, rows))),
    }
    out.retain(|p| !p.text.is_empty());
    out
}
/// The parts as they stand while a screen laid out for `drawn` is shown in `shown` (cells and lines), its first `skip`
/// lines cut off (layout.rs: screen). The window's new edges take over: a frame on the old ones runs on to them, and no
/// text lies past them.
pub fn shown_in(panes: &mut Vec<Pane>, drawn: (usize, usize), shown: (usize, usize), skip: usize) {
    let cut = |r: Rect| { let (top, bottom) = ((r.top() as usize).saturating_sub(skip), (r.bottom() as usize).saturating_sub(skip)); (r.left() as usize, r.right() as usize, top, bottom) };
    for p in panes.iter_mut() {
        let (l, r, t, b) = cut(p.frame);
        p.frame = rect((l, if r == drawn.0 { shown.0 } else { r.min(shown.0) }), (t, if b + skip == drawn.1 { shown.1 } else { b.min(shown.1) }));
        let (l, r, t, b) = cut(p.text);
        p.text = rect((l.min(shown.0), r.min(shown.0)), (t.min(shown.1), b.min(shown.1)));
    }
    panes.retain(|p| !p.text.is_empty());
}
