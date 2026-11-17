//! Line 2 of a live row no agent of the board's works: what the board measured first, then what it was told. At work
//! by hand: the seats in its folder (`席 crowdsourcing: Hermes 6・作業中1(hermes-main)`, flow/seats.rs), or nothing when
//! its title already says whom it was handed to, else `エージェントなし`; then its message, with how long ago it was
//! said and the panes it names that are gone (`報告1d前（wA9:p1 は今は無い）: …`). Waiting for you: `エージェントなし · …`
//! as before (docs/ongoing.md).
use super::fields::no_agent;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::shown::ongoing::NO_AGENT;

pub fn line(s: &State, r: &Obj, said: Inked) -> Inked {
    // Another machine's row: its herdr is not the one read here.
    if !crate::flow::seats::measured(r) || r.b("host") { return no_agent(said); }
    let head = if r.at("seat").is_object() { seat(r.at("seat")) } else if !super::kind::handed(s, r).is_empty() { Inked::new() } else { Inked::of(NO_AGENT, Ink::Soft) };
    if said.is_empty() { return if head.is_empty() { Inked::of(NO_AGENT, Ink::Soft) } else { head }; }
    let gone = strings(r.at("gonePanes"));
    let note = if gone.is_empty() { String::new() } else { format!("（{} は今は無い）", gone.join("・")) };
    let ago = if r.at("at").is_number() { crate::shown::words::age(crate::clock::now() - r.i("at")) + "前" } else { String::new() };
    let told = Inked::of(format!("報告{ago}{note}: "), Ink::Soft).add(said);
    if head.is_empty() { told } else { head.add(Inked::of(" · ", Ink::Soft)).add(told) }
}
/// The seats of a folder: how many of each kind, which work, which wait for input (red); none at all is red too.
fn seat(v: &Value) -> Inked {
    let repo = v.s("repo");
    let kinds = v.arr("kinds");
    if kinds.is_empty() { return Inked::of(format!("席なし（{repo}）"), Ink::Red); }
    let cap = |k: String| { let mut c = k.chars(); c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default() };
    let mut parts: Vec<String> = kinds.iter().map(|k| format!("{} {}", cap(text(&k[0])), text(&k[1]))).collect();
    let (working, blocked) = (strings(v.at("working")), strings(v.at("blocked")));
    if !working.is_empty() { parts.push(format!("作業中{}({})", working.len(), working.join("・"))); }
    let out = Inked::of(format!("席 {repo}: {}", parts.join("・")), Ink::Soft);
    if blocked.is_empty() { out } else { out.add(Inked::of("・", Ink::Soft)).add(Inked::of(format!("入力待ち{}({})", blocked.len(), blocked.join("・")), Ink::Red)) }
}
