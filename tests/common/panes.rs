//! The fake herdr's panes: their processes, layout, splits, moves, labels, the commands run at their shell (an agent started
//! in a read-only mount stays in the pane), what they show, and what the board reports on them.
use super::herdr::{arr, fail, ok, out, FakeHerdr as F, Out};
use serde_json::{json, Value};
use sw::json::*;

/// Words of a shell line: quoted ('it'\''s') or bare.
fn words(line: &str) -> Vec<String> {
    let re = regex::Regex::new(r"'((?:[^']|'\\'')*)'|(\S+)").unwrap();
    re.captures_iter(line).map(|c| match c.get(1) { Some(q) => q.as_str().replace("'\\''", "'"), None => c[2].to_string() }).collect()
}
pub fn pane(st: &mut Value, verb: &str, arg2: &str, argv: &[String], flag: &dyn Fn(&str) -> Option<String>, env_of: &dyn Fn() -> Value) -> Out {
    let find = |st: &Value, id: &str| st.arr("panes").iter().position(|p| p.s("pane_id") == id);
    match verb {
        "process-info" => {
            if st.b("processInfoFails") { return fail("timeout", "timed out"); }
            if flag("--pane").and_then(|id| find(st, &id)).is_some_and(|i| st["panes"][i].b("infoFails")) { return fail("timeout", "timed out"); }
            let Some(pi) = flag("--pane").and_then(|id| find(st, &id)) else { return fail("pane_not_found", "pane not found") };
            let p = &st["panes"][pi];
            let shell = 4194400;
            // The agent is pid 1, with a process under it. `tool`: a command in a group of its own (not listed); in its own
            // group: `helper`, one wired to it (an MCP server); `child`, a command that is not; `wrapped`, the
            // agent's own program under an npm wrapper. `frontPid`: that process in its foreground, whatever else. `shell`: the pane's shell is that real process; `infoFails`: herdr
            // does not answer for this pane
            let under = ["tool", "helper", "child", "wrapped"].iter().any(|k| p.b(k));
            let fg = if p.b("frontPid") { p.i("frontPid") } else if st.b("pids") && p.at("agent").is_object() { p.at("agent").i("pid") } else if under { 1 } else if p.b("busy") { shell + 1 } else { shell };
            let mut front = vec![json!({ "pid": fg, "name": if p.b("frontPid") { "agent" } else if p.b("wrapped") { "node" } else if under { "agent" } else if p.b("busy") { "sleep" } else { "bash" } })];
            if p.b("helper") { front.push(json!({ "pid": 2, "name": "mcp-server" })); }
            if p.b("child") { front.push(json!({ "pid": 3, "name": "sleep" })); }
            if p.b("wrapped") { front.push(json!({ "pid": 3, "name": p.at("agent").at("kind") })); }
            let group = if p.b("frontPid") { fg } else if p.b("busy") { shell + 1 } else if under { fg } else { shell };
            return ok(json!({ "type": "pane_process_info", "process_info": { "pane_id": p.at("pane_id"), "shell_pid": if p.b("job") { 1 } else if p.b("shell") { p.i("shell") } else { shell }, "foreground_process_group_id": group, "foreground_processes": front } }));
        }
        "layout" => {
            let Some(pi) = flag("--pane").and_then(|id| find(st, &id)) else { return fail("pane_not_found", "pane not found") };
            let p = st["panes"][pi].clone();
            let area = json!({ "x": 0, "y": 0, "width": 120, "height": 34 });
            let own = st.at("layouts").at(p.s("pane_id")).clone();
            return ok(json!({ "type": "pane_layout", "layout": if own.is_object() { own } else { json!({ "area": area, "zoomed": false, "focused_pane_id": p.at("pane_id"), "panes": [{ "pane_id": p.at("pane_id"), "focused": true, "rect": area }], "splits": [], "tab_id": p.at("tab_id"), "workspace_id": p.at("workspace_id") }) } }));
        }
        "list" if st.b("paneListFails") => return fail("timeout", "timed out"),
        // as herdr 0.9.3 answers for a space that is closed
        "list" if flag("--workspace").is_some_and(|w| !st.arr("workspaces").iter().any(|x| x.s("workspace_id") == w)) => return fail("workspace_not_found", "workspace not found"),
        "list" => { let w = flag("--workspace"); let ps: Vec<Value> = st.arr("panes").iter().filter(|p| w.as_deref().map(|w| p.s("workspace_id") == w).unwrap_or(true)).map(F::pane_out).collect(); return ok(json!({ "type": "pane_list", "panes": ps })); }
        _ => {}
    }
    let Some(pi) = find(st, arg2) else { return fail("pane_not_found", &format!("pane {arg2} not found")) };
    let p = st["panes"][pi].clone();
    match verb {
        "get" if st.b("paneGetFails") => fail("timeout", "timed out"),
        "get" => ok(json!({ "type": "pane_info", "pane": F::pane_out(&p) })),
        "zoom" => { let on = argv.iter().any(|a| a == "--on"); if on { arr(st, "focus").push(json!(format!("pane {arg2}"))); } ok(json!({ "type": "pane_zoom", "zoom": { "pane_id": arg2, "zoomed": on, "focused_pane_id": arg2 } })) }
        "split" => {
            let wid = p.t("workspace_id");
            let n = st.arr("panes").iter().filter(|x| x.s("workspace_id") == wid).count() + 1;
            let mut id = format!("{wid}:p{n}"); while find(st, &id).is_some() { id.push('x'); }
            let np = st.i("nextP"); st["nextP"] = json!(np + 1);
            let q = json!({ "pane_id": id, "workspace_id": wid, "tab_id": p.at("tab_id"), "terminal_id": format!("term_{np}"), "cwd": flag("--cwd").unwrap_or_else(|| p.t("cwd")), "env": env_of(), "agent": Value::Null });
            arr(st, "panes").push(q.clone());
            ok(json!({ "type": "pane_split", "pane": F::pane_out(&q) }))
        }
        "move" => {
            let to = flag("--workspace").unwrap_or_default();
            let Some(wi) = st.arr("workspaces").iter().position(|w| w.s("workspace_id") == to) else { return fail("workspace_not_found", "the pane was not moved") };
            if st.b("moveFails") { return fail("timeout", "the pane was not moved"); }
            let from = p.t("workspace_id");
            let n = st["workspaces"][wi].i("nextT").max(2);
            let t = F::new_tab(st, wi, &format!("{to}:t{n}"), &flag("--label").unwrap_or_else(|| n.to_string()));
            let count = st.arr("panes").iter().filter(|x| x.s("workspace_id") == to).count() + 1;
            let mut id = format!("{to}:p{count}"); while find(st, &id).is_some() { id.push('x'); }
            st["panes"][pi]["pane_id"] = json!(id); st["panes"][pi]["workspace_id"] = json!(to); st["panes"][pi]["tab_id"] = t.at("tab_id").clone();
            F::drop_empty(st);
            let emptied = !st.arr("workspaces").iter().any(|w| w.s("workspace_id") == from);
            let q = st.arr("panes").iter().find(|x| x.s("pane_id") == id).cloned().unwrap();
            ok(json!({ "type": "pane_move", "move_result": { "changed": true, "closed_workspace_id": if emptied { json!(from) } else { Value::Null }, "created_tab": F::tab_out(st, &t), "pane": F::pane_out(&q) } }))
        }
        "close" => { arr(st, "panes").remove(pi); F::drop_empty(st); ok(json!({ "type": "ok" })) }
        "rename" => { st["panes"][pi]["label"] = json!(argv[3..].join(" ")); ok(json!({ "type": "pane_info", "pane": F::pane_out(&st["panes"][pi].clone()) })) }
        // A command run at the pane's shell. One that starts an agent program in a read-only mount
        // (`env HERDR_PROJECTS_AGENT=<name> bwrap … -- <kind> <args>`, or `… sandbox-exec -p <profile> <kind> <args>`) leaves
        // that agent in the pane.
        "run" => {
            let w = words(&argv[3..].join(" "));
            let at = w.iter().position(|x| x == "sandbox-exec").map(|i| i + 2).or_else(|| w.iter().position(|x| x == "--"));
            let name = w.iter().find_map(|x| x.strip_prefix("HERDR_PROJECTS_AGENT=")).unwrap_or("").to_string();
            if let (true, Some(at), false) = (w.iter().any(|x| x == "bwrap" || x == "sandbox-exec"), at, p.at("agent").is_object()) {
                let seq = st.i("seq") + 1; st["seq"] = json!(seq);
                let kind = w.get(at + 1).cloned().unwrap_or_default();
                st["panes"][pi]["agent"] = json!({ "kind": kind, "name": name, "status": "idle", "seq": seq, "args": w[at + 2..], "pid": 5000000 + seq, "boxed": true });
                let d = st.at("dialogs").i(&kind);
                if d > 0 { st["dialogs"][&kind] = json!(d - 1); st["panes"][pi]["agent"]["status"] = json!("blocked"); }
            } else {
                // The shell takes the line and draws its prompt again. `lineEditorLag`: a line editor (ble.sh) that takes
                // it only after that many reads of the pane.
                st["panes"][pi]["pending"] = json!({ "line": argv[3..].join(" "), "reads": st.i("lineEditorLag") });
                // `multiline`: that many lines a line editor holds as a command of several lines, its Enter read as one
                // more line (ble.sh): C-j runs it, C-c drops it.
                if st.i("multiline") > 0 {
                    st["multiline"] = json!(st.i("multiline") - 1);
                    let (cwd, line) = (st["panes"][pi].t("cwd"), argv[3..].join(" "));
                    st["panes"][pi]["pending"] = json!({ "line": line, "multiline": true });
                    st["panes"][pi]["screen"] = json!(format!("user@host:{cwd}$ {line}\n\n-- MULTILINE -- (RET or C-m: insert a newline, C-j: run)"));
                }
            }
            ok(json!({ "type": "ok" }))
        }
        "read" => {
            // `leftHeld`: the first pane read holds that line from an earlier start as a command of several lines.
            if st.at("leftHeld").is_string() && !p.at("agent").is_object() {
                let (cwd, line) = (p.t("cwd"), st.t("leftHeld"));
                st["leftHeld"] = Value::Null;
                st["panes"][pi]["pending"] = json!({ "line": line, "multiline": true });
                st["panes"][pi]["screen"] = json!(format!("user@host:{cwd}$ {line}\n\n-- MULTILINE -- (RET or C-m: insert a newline, C-j: run)"));
                return out(st["panes"][pi].s("screen"));
            }
            let a = p.at("agent");
            let pending = p.at("pending");
            // `paneWidth`: a pane that narrow, its long rows wrapped, and only the last `--lines` rows read.
            if pending.b("multiline") {
                let w = st.i("paneWidth").max(0) as usize;
                if w == 0 { return out(p.s("screen")); }
                let rows: Vec<String> = p.s("screen").lines().flat_map(|l| { let c: Vec<char> = l.chars().collect(); if c.is_empty() { vec![String::new()] } else { c.chunks(w).map(|r| r.iter().collect()).collect::<Vec<String>>() } }).collect();
                let n = flag("--lines").and_then(|n| n.parse::<usize>().ok()).unwrap_or(rows.len()).min(rows.len());
                return out(&rows[rows.len() - n..].join("\n"));
            }
            if pending.is_object() {
                let (cwd, line) = (p.t("cwd"), pending.t("line"));
                // the first of those reads finds the line not drawn yet: only the prompt
                if pending.i("reads") > 0 { st["panes"][pi]["pending"]["reads"] = json!(pending.i("reads") - 1); let drawn = if pending.i("reads") == st.i("lineEditorLag") { "" } else { &line }; return out(&format!("{}user@host:{cwd}$ {drawn}", p.s("screen"))); }
                st["panes"][pi]["pending"] = Value::Null; st["panes"][pi]["screen"] = json!(format!("{}user@host:{cwd}$ {line}\nuser@host:{cwd}$ \n", p.s("screen")));
                return out(st["panes"][pi].s("screen"));
            }
            if a.i("boot") > 0 { st["panes"][pi]["agent"]["boot"] = json!(a.i("boot") - 1); let mut cmd = vec![a.t("kind")]; cmd.extend(a.arr("args").iter().map(text)); return out(&format!("user@host:{}$ {}\n", p.s("cwd"), cmd.join(" "))); }
            out(if a.b("screen") { a.s("screen") } else { p.s("screen") })
        }
        "send-keys" => {
            arr(st, "paneKeys").push(json!({ "pane": arg2, "keys": argv[3..] }));
            let cwd = p.t("cwd");
            if p.at("pending").b("multiline") {
                let line = p.at("pending").t("line");
                // `heldStays`: a line editor that does not run it on C-j either
                if argv[3..].iter().any(|k| k == "control+j") && !st.b("heldStays") { st["panes"][pi]["pending"] = Value::Null; st["panes"][pi]["screen"] = json!(format!("user@host:{cwd}$ {line}\nuser@host:{cwd}$ \n")); }
                if argv[3..].iter().any(|k| k == "control+c") { st["panes"][pi]["pending"] = Value::Null; st["panes"][pi]["screen"] = json!(format!("user@host:{cwd}$ {line}^C\nuser@host:{cwd}$ \n")); }
            }
            ok(json!({ "type": "ok" }))
        }
        // The interrupt character clears a command its shell waits to see the end of.
        "send-text" => {
            if argv.get(3).map(|t| t == "\u{3}").unwrap_or(false) && p.s("screen").trim_end().ends_with('>') { st["panes"][pi]["screen"] = json!(format!("{}^C\nuser@host:{}$ ", p.s("screen"), p.s("cwd"))); }
            arr(st, "typed").push(json!({ "pane": arg2, "text": argv.get(3) }));
            // A number typed on Codex's hooks review moves its choice (`›`) to that line, as Codex 0.169 does.
            let screen = st["panes"][pi]["agent"].t("screen");
            if let Some(n) = argv.get(3).filter(|t| t.len() == 1 && t.chars().all(|c| c.is_ascii_digit())).filter(|_| screen.contains("Hooks need review")) {
                let moved: Vec<String> = screen.lines().map(|l| { let l = l.replacen("› ", "  ", 1); if l.trim_start().starts_with(&format!("{n}. ")) { l.replacen("  ", "› ", 1) } else { l } }).collect();
                st["panes"][pi]["agent"]["screen"] = json!(moved.join("\n"));
            }
            ok(json!({ "type": "ok" }))
        }
        "report-metadata" => {
            // `tokenRoom`: how many tokens the pane still has room for, the other sources' taken (herdr keeps 32 in all).
            if st["panes"][pi].at("tokenRoom").as_u64().is_some_and(|n| argv.iter().filter(|a| *a == "--token").count() as u64 > n) { return fail("metadata_token_limit", "pane metadata may contain at most 32 tokens"); }
            arr(st, "metadata").push(json!({ "pane": arg2, "args": argv[3..] }));
            if let Some(d) = flag("--display-agent") { st["panes"][pi]["display_agent"] = json!(d); }
            if argv.iter().any(|a| a == "--clear-display-agent") { if let Some(o) = st["panes"][pi].as_object_mut() { o.remove("display_agent"); } }
            ok(json!({ "type": "ok" }))
        }
        _ => fail("unknown_command", &format!("fake herdr does not know: pane {verb}")),
    }
}
