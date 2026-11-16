//! What a hook runs, read for what a hook of the human's own has no business doing: sending over the network, reading
//! secrets, deleting a tree, running code it hides. Read are its command and the files it runs: the files the command
//! names, and those they name in turn (a `require('./x')`, a `path.join(__dirname, 'x.js')`, a script it starts).
//! shortcut: a list of plain words, not a reading of the code: it catches the plain ways, not a hidden one, and not
//! what a program it calls by name (one on PATH) does. Anything it finds stops the trust; a real reading is the human's.
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// What is looked for, with what it means.
const FLAGS: [(&str, &str); 6] = [
    ("sends over the network", r"\b(curl|wget|nc|ncat|socat|telnet|ssh|scp|rsync|ftp)\s"),
    ("sends over the network", r#"require\(\s*['"](node:)?(http|https|http2|net|tls|dgram)['"]"#),
    ("sends over the network", r"\bfetch\(|XMLHttpRequest|WebSocket|/dev/(tcp|udp)/|Invoke-(WebRequest|RestMethod)|\burllib\b|\brequests\.(get|post|put)\(|\bsocket\.|\bhttp\.client"),
    ("reads secrets", r"\.ssh/|id_rsa|id_ed25519|\.netrc|\.aws/|\.git-credentials|auth\.json|credentials|gh auth token|\bbws\b|history\.jsonl|keychain|\.gnupg"),
    ("deletes a tree or history", r"\brm\s+-[a-zA-Z]*[rR]|rmSync\([^)]*recursive|rmdirSync\([^)]*recursive|shutil\.rmtree|\brimraf\b|git\s+(push|reset\s+--hard|clean\s+-)|\bmkfs|\bdd\s+if=|\bshred\b"),
    ("runs code it hides", r#"base64\s+(-d|--decode)|\batob\(|['"]base64['"]|\beval\(|new\s+Function\(|\|\s*(ba|z)?sh\b"#),
];
static RES: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| FLAGS.iter().map(|(w, r)| (*w, Regex::new(r).unwrap())).collect());
static QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"['"]([^'"\n]{1,300})['"]"#).unwrap());
static SCRIPT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\w./~-]+\.(?:js|cjs|mjs|ts|sh|bash|py|rb|pl|ps1)\b").unwrap());
static EXT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.(js|cjs|mjs|ts|tsx|sh|bash|py|rb|pl|ps1)$").unwrap());
/// More files than this read for one hook: not a hook the board judges by itself.
const MOST: usize = 200;

/// What the text has that a hook of the human's own should not: the first thing found, with the line it is on.
pub fn flag(text: &str) -> Option<String> {
    RES.iter().find_map(|(what, re)| re.find(text).map(|m| {
        let from = text[..m.start()].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let to = text[m.start()..].find('\n').map(|i| m.start() + i).unwrap_or(text.len());
        format!("{what} (`{}`)", text[from..to].trim().chars().take(120).collect::<String>())
    }))
}

/// The paths a text names that may be files it runs: in quotes, or a script's name.
fn named(text: &str) -> Vec<String> {
    QUOTED.captures_iter(text).map(|c| c[1].to_string()).chain(SCRIPT.find_iter(text).map(|m| m.as_str().to_string()))
        .chain(text.split_whitespace().map(|w| w.trim_matches(|c| c == '\'' || c == '"').to_string()).filter(|w| w.starts_with('/')))
        .collect()
}
/// A named path as a file, looked for beside `base` and in `root` (a plugin's own folder): with `.js` added for a
/// `require('./x')`.
fn file_of(name: &str, base: &Path, root: &Path, home: &Path) -> Option<PathBuf> {
    let r = root.display().to_string();
    let name = name.replace("${CLAUDE_PLUGIN_ROOT}", &r).replace("$CLAUDE_PLUGIN_ROOT", &r).replace("${PLUGIN_ROOT}", &r).replace("$PLUGIN_ROOT", &r);
    let name = if let Some(rest) = name.strip_prefix("~/") { home.join(rest).display().to_string() } else { name };
    if !(EXT.is_match(&name) || name.starts_with("./") || name.starts_with("../") || name.starts_with('/')) { return None; }
    let places: Vec<PathBuf> = if name.starts_with('/') { vec![PathBuf::from(&name)] } else { vec![base.join(&name), root.join(&name)] };
    places.into_iter().flat_map(|p| [p.clone(), PathBuf::from(format!("{}.js", p.display())), p.join("index.js")]).find(|p| p.is_file()).map(|p| clean(&p))
}
fn clean(p: &Path) -> PathBuf { p.components().fold(PathBuf::new(), |mut out, c| { match c { std::path::Component::ParentDir => { out.pop(); } std::path::Component::CurDir => {} c => out.push(c) } out }) }

/// Every file the command runs, read: the command's own files and theirs in turn. Err: a file that does not read as
/// text, or more files than a hook should have.
pub fn files(command: &str, root: &Path, home: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let mut todo: Vec<PathBuf> = named(command).iter().filter_map(|n| file_of(n, root, root, home)).collect();
    let mut seen: Vec<(PathBuf, String)> = vec![];
    while let Some(f) = todo.pop() {
        if seen.iter().any(|(p, _)| *p == f) { continue; }
        if seen.len() >= MOST { return Err(format!("it runs more than {MOST} files")); }
        let text = std::fs::read_to_string(&f).map_err(|e| format!("{} does not read as text ({e})", f.display()))?;
        let base = f.parent().map(Path::to_path_buf).unwrap_or_default();
        todo.extend(named(&text).iter().filter_map(|n| file_of(n, &base, root, home)));
        seen.push((f, text));
    }
    Ok(seen)
}
