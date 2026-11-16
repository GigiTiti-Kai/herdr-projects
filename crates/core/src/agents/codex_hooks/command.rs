//! What a hook's command starts, told from the command as a shell splits it: every program of every part
//! (`a && b; c | d`), read through the programs that run code (`sh -c '…'` read again as a command, the file given to
//! `bash` or `node`). Any part that runs what cannot be read before it runs is refused. A relative name is the seat's
//! folder's, as Codex runs a hook there (codex-rs hooks/src/engine/command_runner.rs, `current_dir(cwd)`, 0.169).
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Shells: `-c` gives them a command, a file their script.
const SHELLS: [&str; 4] = ["sh", "bash", "zsh", "dash"];
/// Programs that run code: `-e`/`-c`/`--eval` gives them the code itself, a file their script.
const CODE: [&str; 8] = ["node", "python", "python3", "ruby", "perl", "deno", "bun", "pwsh"];
static LATER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`|\$\(|\$\{?[A-Za-z_]").unwrap());
/// Code given inline that starts other programs or loads code by a name: which is not read.
static STARTS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"child_process|\bspawn|\bexec\w*\s*\(|subprocess|os\.system|popen|\bsystem\s*\(|\bimport\s*\(").unwrap());

/// The plugin's folder and the human's home as a command names them, put in.
pub fn known(command: &str, root: &Path, home: &Path) -> String {
    let (r, h) = (root.display().to_string(), home.display().to_string());
    command.replace("${CLAUDE_PLUGIN_ROOT}", &r).replace("$CLAUDE_PLUGIN_ROOT", &r).replace("${PLUGIN_ROOT}", &r).replace("$PLUGIN_ROOT", &r).replace("${HOME}", &h).replace("$HOME", &h)
}

/// The command's parts and their words, as a shell splits them: quotes taken off, `;`, `&`, `|` and new lines between
/// parts. Err for a quote left open.
fn parts(s: &str) -> Result<Vec<Vec<String>>, String> {
    let (mut out, mut words, mut word, mut quote, mut any) = (vec![], vec![], String::new(), None::<char>, false);
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => { if let Some(n) = chars.next() { word.push(n); } }
            (Some(_), c) => word.push(c),
            (None, '\'' | '"') => { quote = Some(c); any = true; }
            (None, '\\') => { if let Some(n) = chars.next() { word.push(n); any = true; } }
            (None, c) if c.is_whitespace() || matches!(c, ';' | '&' | '|') => {
                if any || !word.is_empty() { words.push(std::mem::take(&mut word)); any = false; }
                if (c != ' ' && c != '\t') && !words.is_empty() { out.push(std::mem::take(&mut words)); }
            }
            (None, c) => word.push(c),
        }
    }
    if quote.is_some() { return Err("its command leaves a quote open".into()); }
    if any || !word.is_empty() { words.push(word); }
    if !words.is_empty() { out.push(words); }
    Ok(out)
}

/// A name the command gives as a file: absolute, from home, or in the seat's folder.
fn file(w: &str, cwd: &Path, home: &Path) -> Option<PathBuf> {
    let p = if let Some(rest) = w.strip_prefix("~/") { home.join(rest) } else if w.starts_with('/') { PathBuf::from(w) } else { cwd.join(w) };
    p.is_file().then(|| p.components().collect()) // `./x` as the file's own name
}

/// The files the command starts (its scripts). Err: a part runs what cannot be read before it runs.
pub fn runs(command: &str, root: &Path, cwd: &Path, home: &Path) -> Result<Vec<PathBuf>, String> {
    let text = known(command, root, home);
    if LATER.is_match(&text) { return Err("what it runs is only known when it runs (a `$` value or a command inside it)".into()); }
    let mut out = vec![];
    for words in parts(&text)? {
        let words: Vec<&String> = words.iter().skip_while(|w| w.split_once('=').is_some_and(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_alphanumeric() || c == '_'))).collect();
        let Some(first) = words.first() else { continue };
        // A program is one by its name alone, found on PATH; a name with a path (`./node`) is a file like any other.
        let program = first.as_str();
        let (shell, code) = (SHELLS.contains(&program), CODE.contains(&program));
        if !shell && !code {
            out.push(file(first, cwd, home).ok_or_else(|| format!("it runs `{first}`, which the board does not read"))?);
            continue;
        }
        let mut args = words.iter().skip(1);
        let script = loop {
            match args.next().map(|w| w.as_str()) {
                Some("-c") if shell => { out.extend(runs(args.next().map(|w| w.as_str()).unwrap_or(""), root, cwd, home)?); break None; }
                Some("-e" | "--eval" | "-c" | "-Command") if code => {
                    if STARTS.is_match(args.next().map(|w| w.as_str()).unwrap_or("")) { return Err(format!("the code it gives `{program}` starts other programs or loads code by a name")); }
                    break None;
                }
                Some(w) if w.starts_with('-') => continue,
                Some(w) => break Some(w),
                None => return Err(format!("`{program}` reads what it runs from its input, which the board cannot read")),
            }
        };
        if let Some(s) = script { out.push(file(s, cwd, home).ok_or_else(|| format!("it runs {s}, which is not there to read"))?); }
    }
    if out.is_empty() && parts(&text)?.is_empty() { return Err("its command is empty".into()); }
    Ok(out)
}
