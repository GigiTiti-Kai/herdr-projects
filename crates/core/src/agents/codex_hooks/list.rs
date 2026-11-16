//! What Codex itself says about the hooks it runs in a folder: its app-server's `hooks/list` (Codex 0.169), each hook
//! with its source, event, command, file and trust status, the same list its "Review hooks" screen shows. Only read:
//! nothing is trusted here. A test answers for it (`set_fake`), so no program is started then.
use crate::json::*;
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct Hook { pub key: String, pub source: String, pub plugin: String, pub event: String, pub command: String, pub file: PathBuf, pub status: String, pub enabled: bool, pub managed: bool }
impl Hook {
    /// One the dialog asks about: Codex has no trust for it (`untrusted`), or not for what it is now (`modified`).
    pub fn pending(&self) -> bool { self.enabled && !self.managed && (self.status == "untrusted" || self.status == "modified") }
}

type Fake = Rc<dyn Fn(&Path) -> Result<Value, String>>;
thread_local! { static FAKE: RefCell<Option<Fake>> = const { RefCell::new(None) }; }
/// A stand-in for Codex's app-server: it gets the folder, and answers its entry of `hooks/list` (or why it could not).
pub fn set_fake(f: Option<Fake>) { FAKE.with(|x| *x.borrow_mut() = f); }
/// How long Codex is given to answer: it reads its config, plugins and every hooks file first.
const LIST_MS: u64 = 30000;

/// The hooks Codex runs in `cwd`, as its own list says.
pub fn hooks(cwd: &Path) -> Result<Vec<Hook>, String> {
    let entry = match FAKE.with(|x| x.borrow().clone()) { Some(f) => f(cwd)?, None => ask(cwd)? };
    let errors: Vec<String> = entry.arr("errors").iter().map(|e| format!("{}: {}", e.s("path"), e.s("message"))).collect();
    if !errors.is_empty() { return Err(format!("Codex could not read some of its hooks ({})", errors.join("; "))); }
    Ok(entry.arr("hooks").iter().map(|h| Hook {
        key: h.t("key"), source: h.t("source"), plugin: h.t("pluginId"), event: h.t("eventName"), command: h.t("command"),
        file: PathBuf::from(h.s("sourcePath")), status: h.t("trustStatus"),
        enabled: h.at("enabled").as_bool().unwrap_or(false), managed: h.at("isManaged").as_bool().unwrap_or(false),
    }).collect())
}

/// One `hooks/list` asked of a Codex app-server started for it on stdio, then ended.
fn ask(cwd: &Path) -> Result<Value, String> {
    let mut child = Command::new(crate::agents::kinds::program("codex")).arg("app-server").current_dir(cwd)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| format!("Codex's app-server did not start ({e})"))?;
    let lines = [
        json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "clientInfo": { "name": "herdr-projects", "version": env!("CARGO_PKG_VERSION") } } }),
        json!({ "jsonrpc": "2.0", "method": "initialized" }),
        json!({ "jsonrpc": "2.0", "id": 1, "method": "hooks/list", "params": { "cwds": [cwd.display().to_string()] } }),
    ];
    let mut stdin = child.stdin.take();
    if let Some(i) = stdin.as_mut() { for l in &lines { let _ = writeln!(i, "{l}"); } let _ = i.flush(); }
    let out = child.stdout.take();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let Some(out) = out else { return };
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Ok(v) = serde_json::from_str::<Value>(&line) { if v.at("id") == &json!(1) { let _ = tx.send(v); return; } }
        }
    });
    let got = rx.recv_timeout(Duration::from_millis(LIST_MS));
    drop(stdin); // its input kept open until the answer: an app-server whose input ends may go before it answers
    let _ = child.kill();
    let _ = child.wait();
    let v = got.map_err(|_| format!("Codex's app-server did not answer hooks/list within {} s", LIST_MS / 1000))?;
    if v.has("error") { return Err(format!("Codex's app-server refused hooks/list: {}", v.at("error").t("message"))); }
    v.at("result").arr("data").first().cloned().ok_or_else(|| "Codex's app-server listed no folder".to_string())
}
