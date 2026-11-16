//! What a hook runs, read for what a hook of the human's own has no business doing: sending over the network, reading
//! secrets, deleting a tree, running code it hides. Read are its command and the files it runs: the files the command
//! names, and those they name in turn (a `require('./x')`, a `path.join(__dirname, 'x.js')`, a script it starts).
//! shortcut: a list of plain words, not a reading of the code: it catches the plain ways, not a hidden one, nor which
//! programs a file (not the command: command.rs) starts. Anything it finds stops the trust; a real reading is the human's.
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// What is looked for, with what it means: secrets first, as `~/.ssh` also names a program.
const FLAGS: [(&str, &str); 7] = [
    ("reads secrets", r"\.ssh/|id_rsa|id_ed25519|\.netrc|\.aws/|\.git-credentials|auth\.json|credentials|gh auth token|\bbws\b|history\.jsonl|keychain|\.gnupg"),
    ("sends over the network", r"\b(curl|wget|nc|ncat|socat|telnet|ssh|scp|sftp|rsync|ftp)\b"),
    ("sends over the network", r#"['"`](node:)?(http|https|http2|net|tls|dgram|dns|undici)['"`]"#), // a module, however it is loaded (`require (…)`, `import(…)`, `from …`)
    ("sends over the network", r"\bfetch\s*\(|XMLHttpRequest|WebSocket|/dev/(tcp|udp)/|Invoke-(WebRequest|RestMethod)|\burllib\b|\brequests\.(get|post|put)\(|\bsocket\.|\bimport\s+socket\b|\bhttp\.client|net/http|Net::HTTP|smtplib"),
    ("deletes a tree or history", r#"\brm\b[^\n]{0,80}?(\s|['"])(-[a-zA-Z]*[rR][a-zA-Z]*\b|--recursive)|\.rm\([^)]*recursive|\bfind\b[^\n]*\s-delete\b"#),
    ("deletes a tree or history", r"rmSync\([^)]*recursive|rmdirSync\([^)]*recursive|shutil\.rmtree|\brimraf\b|git\s+(push|reset\s+--hard|clean\s+-)|\bmkfs|\bdd\s+if=|\bshred\b"),
    ("runs code it hides", r#"base64\s+(-d|--decode)|\batob\s*\(|['"]base64['"]|\beval\s*\(|\bFunction\s*\(|\|\s*(ba|z)?sh\b"#),
];
static RES: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| FLAGS.iter().map(|(w, r)| (*w, Regex::new(r).unwrap())).collect());
static QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"'([^'\n]{1,300})'|"([^"\n]{1,300})"|`([^`\n]{1,300})`"#).unwrap());
static SCRIPT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\w./~-]+\.(?:js|cjs|mjs|ts|sh|bash|py|rb|pl|ps1)\b").unwrap());
static EXT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.(js|cjs|mjs|ts|tsx|sh|bash|py|rb|pl|ps1)$").unwrap());
/// More files than this read for one hook: not a hook the board judges by itself.
const MOST: usize = 200;

/// Node's own modules a hook may load by name; any other name is found where it runs (a seat's `node_modules`) and
/// not read. Those that send over the network are in FLAGS.
const BUILTIN: [&str; 22] = ["fs", "fs/promises", "path", "os", "crypto", "util", "events", "stream", "string_decoder", "vm", "url", "readline", "assert", "buffer", "zlib", "timers", "process", "worker_threads", "perf_hooks", "querystring", "module", "child_process"];
static LOADS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?m)(?:\b(?:require|import)\s*\(\s*|\bfrom\s*|^\s*import\s+)['"`]([\w@./:-]+)['"`]"#).unwrap());

static REMOVES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(rm|rmdir|rmSync|rmdirSync|rimraf)\s*\(").unwrap());
static COMPUTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\b(?:require|import)\s*\(\s*[^'"`\s)]|\bcreateRequire\b|\bModule\._load\b|\bprocess\.dlopen\b"#).unwrap());
/// The names a text has in quotes.
pub fn quoted(text: &str) -> Vec<String> { QUOTED.captures_iter(text).filter_map(|c| c.get(1).or(c.get(2)).or(c.get(3)).map(|m| m.as_str().to_string())).collect() }
/// The modules a text loads by a name written in it.
pub fn loads(text: &str) -> Vec<String> { LOADS.captures_iter(text).map(|c| c[1].to_string()).collect() }

/// What the text has that a hook of the human's own should not: the first thing found, with the line it is on.
pub fn flag(text: &str) -> Option<String> {
    let found = RES.iter().find_map(|(what, re)| re.find(text).map(|m| {
        let from = text[..m.start()].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let to = text[m.start()..].find('\n').map(|i| m.start() + i).unwrap_or(text.len());
        format!("{what} (`{}`)", text[from..to].trim().chars().take(120).collect::<String>())
    }));
    if found.is_some() { return found; }
    if REMOVES.is_match(text) && text.contains("recursive") { return Some("deletes a tree or history (a removal with `recursive`)".into()); }
    if let Some(name) = loads(text).into_iter().find(|n| !(n.starts_with('.') || n.starts_with('/') || BUILTIN.contains(&n.strip_prefix("node:").unwrap_or(n)))) {
        return Some(format!("loads the module `{name}` by its name, found where it runs and not read"));
    }
    None
}

/// The paths a text names that may be files it runs: in quotes, or a script's name.
fn named(text: &str) -> Vec<String> {
    QUOTED.captures_iter(text).filter_map(|c| c.get(1).or(c.get(2)).or(c.get(3)).map(|m| m.as_str().to_string())).chain(SCRIPT.find_iter(text).map(|m| m.as_str().to_string()))
        .chain(text.split_whitespace().map(|w| w.trim_matches(|c| c == '\'' || c == '"').to_string()).filter(|w| w.starts_with('/')))
        .collect()
}
/// Every file a named path may be, beside each of `bases`: with `.js` added for a `require('./x')`. All of them, as
/// which one runs is the program's choice.
fn files_of(name: &str, bases: &[&Path], home: &Path) -> Vec<PathBuf> {
    let name = if let Some(rest) = name.strip_prefix("~/") { home.join(rest).display().to_string() } else { name.to_string() };
    if !(EXT.is_match(&name) || name.starts_with("./") || name.starts_with("../") || name.starts_with('/')) { return vec![]; }
    let places: Vec<PathBuf> = if name.starts_with('/') { vec![PathBuf::from(&name)] } else { bases.iter().map(|b| b.join(&name)).collect() };
    // Every file node (or a script's own loader) may take for the name, so none of them runs unread.
    let ends = ["", ".js", ".cjs", ".mjs", ".ts", ".d.ts", ".json", ".node"];
    places.into_iter().flat_map(|p| ends.iter().map(|e| PathBuf::from(format!("{}{e}", p.display()))).chain(["index.js", "index.cjs", "index.mjs", "index.json", "index.node"].iter().map(|i| p.join(i))).collect::<Vec<_>>())
        .filter(|p| p.is_file()).map(|p| clean(&p)).collect()
}
fn clean(p: &Path) -> PathBuf { p.components().fold(PathBuf::new(), |mut out, c| { match c { std::path::Component::ParentDir => { out.pop(); } std::path::Component::CurDir => {} c => out.push(c) } out }) }

/// Every file the command runs, read: what it starts (command.rs), the files it names, and theirs in turn. A name is
/// looked for in the seat's folder `cwd` (where Codex runs the hook), and for a plugin's hook that names its plugin's
/// folder, also in `root`; in a file, beside that file too. Err: a part runs what cannot be read, a file that does not
/// read as text, or more files than a hook should have.
pub fn files(command: &str, root: &Path, cwd: &Path, home: &Path, plugin: bool) -> Result<Vec<(PathBuf, String)>, String> {
    let mut todo = super::command::runs(command, root, cwd, home)?;
    let in_root = plugin && (command.contains("PLUGIN_ROOT"));
    let text = super::command::known(command, root, home);
    let bases: Vec<&Path> = if in_root { vec![cwd, root] } else { vec![cwd] };
    let names: Vec<String> = named(&text).into_iter().chain(super::command::words(&text).iter().flat_map(|w| named(w))).collect();
    todo.extend(names.iter().flat_map(|n| files_of(n, &bases, home)));
    let mut seen: Vec<(PathBuf, String)> = vec![];
    while let Some(f) = todo.pop() {
        if seen.iter().any(|(p, _)| *p == f) { continue; }
        if seen.len() >= MOST { return Err(format!("it runs more than {MOST} files")); }
        let text = std::fs::read_to_string(&f).map_err(|e| format!("{} does not read as text ({e})", f.display()))?;
        let base = f.parent().map(Path::to_path_buf).unwrap_or_default();
        let bases: Vec<&Path> = if plugin { vec![&base, cwd, root] } else { vec![&base, cwd] };
        todo.extend(named(&text).iter().flat_map(|n| files_of(n, &bases, home)));
        // A module named by what it works out when it runs (`require(path.join(__dirname, 'pkg'))`): which file that is,
        // or which package folder's entry, is not known here.
        if let Some(m) = COMPUTED.find(&text) { return Err(format!("{} loads a module whose name it works out when it runs (`{}`)", f.display(), m.as_str())); }
        // A folder it loads as a package: node runs what its package.json names, which is not followed here.
        if let Some(n) = loads(&text).into_iter().find(|n| (n.starts_with('.') || n.starts_with('/')) && base.join(n).join("package.json").is_file()) { return Err(format!("{} loads the package folder {n}, whose own entry the board does not follow", f.display())); }
        // A file it loads by a path that is not there to read: whatever node would take for it is not judged.
        if let Some(n) = loads(&text).into_iter().find(|n| (n.starts_with('.') || n.starts_with('/')) && files_of(n, &[&base], home).is_empty()) { return Err(format!("{} loads {n}, which is not there to read", f.display())); }
        seen.push((f, text));
    }
    Ok(seen)
}
