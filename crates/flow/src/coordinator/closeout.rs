//! A coordinator's record-only closeout before it is stopped or begun anew (`workspace.closeout`, off unless you turn it
//! on): the same steps as a worker's (land/closeout.rs), its record (`closeout`) kept with the coordinator's own. A
//! restart (the same conversation) and an open need none. Until it is through the stop waits; held, it is not carried
//! out, and the coordinator says why.
use super::co::Co;
use super::state::{change, home};
use crate::herdr::Herdr;
use crate::json::*;
pub use crate::land::closeout::{pane_gone, Gate};
use crate::land::closeout::{on, step, Who};

pub fn gate(co: &Co, c: &Obj, what: &str) -> Gate {
    if !["stop", "new"].contains(&what) || !on(None) || !c.b("pane") || !c.b("run") { return Gate::Go; }
    let w = Who { api: Herdr::new(Some(co.socket.clone())), pane: c.t("pane"), session: c.t("session"), transcript: c.t("transcript"), agent: c.at("run").t("kind"), cwd: home(co), tip: String::new(), events: 0, wait_ms: crate::store::settings::values::ms(crate::store::settings::lookup::setting("workspace.closeoutWait", None).as_str().unwrap_or("10m")) };
    let save = |patch: Value, whole: bool| change(co, |c| { if whole { c.put("closeout", patch.clone()); } else { for (k, v) in patch.as_object().cloned().unwrap_or_default() { c.obj_mut("closeout").insert(k, v); } } true }).is_ok();
    step(&w, c.at("closeout"), &save)
}
