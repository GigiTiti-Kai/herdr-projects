//! What a hook's command starts, told from the command as a shell splits it: every program of every part
//! (`a && b; c | d`), read through the programs that run code (`sh -c '…'` read again as a command, the file given to
//! `bash` or `node`). Any part that runs what cannot be read before it runs is refused. A relative name is the seat's
//! folder's, as Codex runs a hook there (codex-rs hooks/src/engine/command_runner.rs, `current_dir(cwd)`, 0.169).
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Shells: `-c` gives them a command, a file their script; no other option is taken.
const SHELLS: [&str; 4] = ["sh", "bash", "zsh", "dash"];
/// Programs that run a script file given first, with no option before it. Only `node` also takes code (`-e`).
const SCRIPTS: [&str; 7] = ["node", "python", "python3", "ruby", "perl", "deno", "bun"];
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
        // Only what the board knows a command's shape by: no variable set for it (`PATH=.`), a program by its bare
        // name (one with a path, `./node`, is a file like any other), and no option it does not judge (`--require`).
        let Some(first) = words.first() else { continue };
        if let Some((k, _)) = first.split_once('=').filter(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_alphanumeric() || c == '_')) {
            return Err(format!("it sets {k} for what it runs, which the board does not judge"));
        }
        let program = first.as_str();
        if !SHELLS.contains(&program) && !SCRIPTS.contains(&program) {
            out.push(file(first, cwd, home).ok_or_else(|| format!("it runs `{first}`, which the board does not read"))?);
            continue;
        }
        let (opt, rest) = (words.get(1).map(|w| w.as_str()), words.get(2).map(|w| w.as_str()).unwrap_or(""));
        match opt {
            Some("-c") if SHELLS.contains(&program) => out.extend(runs(rest, root, cwd, home)?),
            Some("-e" | "--eval") if program == "node" => {
                if STARTS.is_match(rest) { return Err(format!("the code it gives `{program}` starts other programs or loads code by a name")); }
                // Anything after its code that is an option is still `node`'s own (`--require ./x`).
                if let Some(o) = words.iter().skip(3).find(|w| w.starts_with('-')) { return Err(format!("it gives `{program}` the option {o}, which the board does not judge")); }
            }
            Some(o) if o.starts_with('-') => return Err(format!("it gives `{program}` the option {o}, which the board does not judge")),
            Some(s) => out.push(file(s, cwd, home).ok_or_else(|| format!("it runs {s}, which is not there to read"))?),
            None => return Err(format!("`{program}` reads what it runs from its input, which the board cannot read")),
        }
    }
    if out.is_empty() && parts(&text)?.is_empty() { return Err("its command is empty".into()); }
    Ok(out)
}
