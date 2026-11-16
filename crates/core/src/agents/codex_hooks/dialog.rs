//! Codex's "Hooks need review" screen at its start (Codex 0.169, its tui/src/startup_hooks_review.rs), as herdr reads
//! it: how many hooks it counts, and which line of its list is "Trust all and continue". Any other screen is not it.
use regex::Regex;
use std::sync::LazyLock;

static COUNT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(\d+) hooks? (?:is|are) new or changed\.").unwrap());
static TRUST: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^[\s│]*(›)?\s*(\d)\.\s+Trust all and continue").unwrap());

/// How many hooks the screen says are new or changed. None when the screen is not Codex's hooks review.
/// The dialog is all of it, at the bottom of the screen: its title, its count and its three lines, so a screen that only
/// shows those words higher up (an agent quoting them) is not it.
pub fn count(screen: &str) -> Option<usize> {
    let lines: Vec<&str> = screen.lines().filter(|l| !l.trim().is_empty()).collect();
    let bottom = lines[lines.len().saturating_sub(10)..].join("\n");
    if !["Hooks need review", "1. Review hooks", "3. Continue without trusting"].iter().all(|w| bottom.contains(w)) { return None; }
    trust_line(&bottom)?;
    COUNT.captures(&bottom)?.get(1)?.as_str().parse().ok()
}

/// The number of the screen's "Trust all and continue" line, and whether it is the chosen one (`›` in front of it).
pub fn trust_line(screen: &str) -> Option<(String, bool)> {
    let c = TRUST.captures(screen)?;
    Some((c[2].to_string(), c.get(1).is_some()))
}
