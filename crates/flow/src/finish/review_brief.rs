//! What the reviewer is told, and reading the verdict it writes back.
use crate::automations::told::automation_rule;
use crate::board_rule::reviewer_board_rule;
use crate::err::Res;
use crate::flow::task::review_dir;
use crate::gate;
use crate::json::*;
use crate::research::{is_research, review_rule};
use crate::review_page::link_mode;

/// The required `risky` answer of a work review (not of a plan's look): what the human must see before a yes.
pub const RISKY_RULE: &str = "Also answer, in `risky` (true or false, required), whether the change does anything risky: something hard to undo or that reaches past the repository. That is: deleting or rewriting data, a migration, secrets, credentials or sign-in, sending anything to an outside service, money or billing, or what runs by itself in production or on the human's machine. When true, name each risky change in `riskyChanges`, one sentence each; when false, leave it empty. A verdict without `risky` does not count. Do not write about it in `text`: the board adds its own line from these fields.";
/// The fixed last sentence the board adds to a review's text from `risky`, as the human's auto-ship reads it
/// (dotfiles bin/auto-ship: the phrase once, and the last sentence, split at 「。」 with no spaces, exactly 「危ない変更:なし」).
pub fn risky_end(text: &str, risky: &[String]) -> String {
    let line = if risky.is_empty() { "危ない変更:なし".to_string() } else { format!("危ない変更:あり（{}）", risky.join(" / ")) };
    let t = text.trim_end();
    format!("{t}{}{line}", if t.is_empty() || t.ends_with(['。', '．']) { "" } else { "。" })
}
/// What the brief is about: the work's own words, its change, whether it lands, the settings, the round before.
pub struct Brief<'a> { pub text: &'a str, pub change: Option<&'a Value>, pub to_land: bool, pub ch: &'a Obj, pub earlier: Option<&'a [Value]>, pub check: &'a str }
pub fn review_brief(t: &Obj, b: &Brief) -> String {
    let dir = review_dir(t).display().to_string();
    let share = if link_mode() == "tailscale" {
        "If you start something for the human to try (a dev server, a page), bind it to 127.0.0.1 and share it privately on their tailnet: run `tailscale serve status` first so you replace nothing, then `tailscale serve --bg --https=<a free port> <your port>`; the link is https://<this machine's tailnet name>:<that port>/. Never use tailscale funnel or anything public. Leave it running."
    } else { "If you start something for the human to try (a dev server, a page), bind it to 127.0.0.1, leave it running, and give its http://127.0.0.1:<port>/ link." };
    let earlier = b.earlier.unwrap_or(&[]);
    let open: Vec<&Value> = earlier.iter().filter(|f| !f.b("board") && !f.b("preexisting")).collect();
    let mut old: Vec<&Value> = vec![];
    for f in earlier.iter().filter(|f| f.b("preexisting")) { if !old.iter().any(|x| gate::same_fault(x, f)) { old.push(f); } }
    let known = if old.is_empty() { String::new() } else { format!("Already known to the human (they were there before this change): do not report these again, in these words or in others.\n{}", old.iter().map(|f| format!("- {}", gate::finding_line(f))).collect::<Vec<_>>().join("\n")) };
    let plan_rule = crate::plan::review_rule(t);
    let head = b.change.map(|c| c.t("head")).unwrap_or_default();
    let desc: String = t.s("description").chars().take(3000).collect();
    let said: String = b.text.chars().take(2000).collect();
    let where_ = if is_research(t) { review_rule(t) } else {
        format!("{} Read the changed files and what surrounds them. Then actually exercise the change: run it, run its tests, open it in a browser if it has a screen, try the awkward inputs. Check it against what was asked: does it do all of it, correctly? Does it break anything nearby? Ignore taste and style nits.",
            if b.to_land { "You are in the repository, on the branch with the change, with all your tools." } else { "You are in the folder where the work was done, with all your tools. There is no diff: find what the agent made or changed from what it says below." })
    };
    let evidence = if b.ch.b("review.evidence") { format!("Curate evidence that genuinely helps the human judge this work. Choose a small set of distinct, useful items (aim for at most six); no duplicates, scratch files, exhaustive output dumps, or decorative reports. A single navigable review page will combine the work summary, your verdict and only the evidence you explicitly list in your verdict. Do not make your own wrapper page. Each caption should name what matters; each detail should explain what that item proves, or how to try it.\n- Something with a screen (a page, an app, a game, a chart): screenshots of it working, and of anything broken.\n- Something that moves or is interactive: a short screen recording, if you are able to make one.\n- Something a person can use: a link to try it themselves.\n- Logic, a library, a script: the output of the tests or commands you ran, saved as a text file.\n- A tiny or obvious change (a typo, a rename, a one-line fix): no artifact is needed. Do not manufacture any.\nSave evidence files in {dir}/ (the board's own folder for this review, outside the folder you are in), using plain file names. Do not include secrets or private credentials in evidence.\n{share}") } else { String::new() };
    let change_line = if b.to_land { let c = b.change.cloned().unwrap_or(Value::Null); format!("The change is everything on this branch since it started{}. The commit under review is {}. See all of it yourself, it is not pasted here: `git diff {}`", if c.b("stat") { format!(" ({})", c.s("stat")) } else { String::new() }, c.s("head"), c.s("range")) } else { String::new() };
    let later = if b.earlier.is_some() {
        format!("This is a later round: the agent was sent back with the findings of the round before, listed below. Your job now is narrower. Check that each one is fixed, and that the fixes broke nothing. Return each one that is still not fixed, with its `id`; leave out the ones that are fixed. You may add a new finding (with no id) only if it is critical. Anything smaller that you notice only now is left out: do not report it. The one exception is a fault that was already there before this change: report it, marked `preexisting`: it is a note for the human and never sends the work back.\n{}{}",
            if open.is_empty() { "Earlier findings: none of the task's own.".to_string() } else { format!("Earlier findings:\n{}", open.iter().map(|f| format!("- {} {}", f.s("id"), gate::finding_line(f))).collect::<Vec<_>>().join("\n")) },
            if known.is_empty() { String::new() } else { format!("\n{known}") })
    } else { known.clone() };
    let verdict = format!("Last of all, save your verdict as JSON in {dir}/verdict.json, then stop. The board waits for that file. Its shape: {{\"commit\": \"{}\", \"meets\": <true or false>, \"text\": \"<two or three plain sentences: your overall judgement>\", \"risky\": <true or false>, \"riskyChanges\": [\"<when risky: one risky change, one sentence>\", ...], \"findings\": [{{{}\"severity\": \"<critical, major, minor or trivial>\", \"subject\": \"<one sentence: what is wrong>\", \"where\": \"<the file or place>\", \"proof\": \"<what you ran or saw; may be empty>\", \"preexisting\": <true or false>}}, ...], \"checks\": [\"<what you actually exercised and the result>\", ...], \"limitations\": [\"<important areas you could not verify>\", ...], {}\"artifacts\": [{{\"file\": \"<file name you saved in that folder>\" or \"url\": \"<link to try it>\", \"caption\": \"<what it shows, a few words>\", \"detail\": \"<why it matters or what to try>\"}}, ...]}}  (findings and artifacts may be empty lists; do not invent findings, checks or proof)",
        if head.is_empty() { "<the commit you reviewed>".to_string() } else { head.clone() },
        if b.earlier.is_some() { "\"id\": \"<the earlier finding's id; leave out for a new one>\", " } else { "" },
        if plan_rule.is_empty() { "" } else { "\"offPlan\": [\"<where the build left the approved plan, one sentence each>\", ...], " });
    [
        "You are an adversarial reviewer. Another agent says the task below is finished. Assume it is not until the evidence says so: your job is to find what is wrong, missing or risky, and to prove what works.".to_string(),
        where_, plan_rule,
        if b.check.is_empty() { String::new() } else if t.b("checkShort") { format!("This repo's check is `{}`. On this round the board ran its shorter form, `{}` (what the change since the full check reaches), and that passed on this commit; the full check runs on GitHub before the merge. Run tests only through these.", b.check, t.s("checkShort")) } else { format!("This repo's check is `{}`; it passed on this commit. Run tests only through it.", b.check) },
        "You are here to review, not to fix. Do not change, commit, push or merge anything in the repository. Whatever you alter in the code is discarded when you finish.".into(),
        reviewer_board_rule(), crate::temp::rule(t), automation_rule(t), crate::autopilot::allow::told(t), evidence,
        format!("Task: {}", t.s("title")), if desc.is_empty() { String::new() } else { format!("Details:\n{desc}") },
        format!("What the agent says it did:\n{said}"), change_line,
        format!("First answer one fixed question, true or false: {} All of it, and working. That answer is `meets`. It decides nothing by itself: whatever makes you answer false, report it as a finding with the severity it deserves. Only an answer of false with no finding at all is counted as a critical finding.", gate::QUESTION),
        "Then report what you found as a list of findings, one fault each. A finding has a `subject` (one sentence saying what is wrong), a `where` (the file or the place), a `proof` (what you actually ran or saw that shows it; leave it empty if you did not show it), `preexisting` (true when the fault was already there before this change and the task did not cause it: such a finding is a note for the human, it never sends the work back and nothing else is done with it) and a `severity`. Choose the severity by what the fault does, never by whether you would like this merged:\n- critical: only one of these four. A fault a user of this would hit in ordinary use; data that is lost or wrong; something unsafe; or a requirement the task states plainly that is not built. Nothing else is critical: not a missing picture or recording, not a line of a document, not a test or proof that cannot be had on this machine, not something the task did not plainly ask for.\n- major: a real fault in an unusual case; or a claim that is not proven (a fix not shown to work, a test that cannot catch the bug it is for); or a document that states something false.\n- minor: correct but could be better: wording, naming, a small inaccuracy, a narrow gap.\n- trivial: a note. Nothing to do.".into(),
        format!("You do not decide whether this {} or goes back, and you give no score: the board decides from your findings by a fixed rule. A critical finding with an empty proof is counted as major. If nothing qualifies, return an empty list and say so. Do not invent findings to have something to report.", if b.to_land { "merges" } else { "is accepted" }),
        RISKY_RULE.into(),
        later, verdict,
    ].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n")
}
/// The board's rules applied to whatever the reviewer wrote. None: a verdict for another commit (ignored).
pub fn parse_verdict(raw: &str, head: &str, earlier: Option<&[Value]>, used: &[String]) -> Res<Option<Obj>> {
    let j: Value = serde_json::from_str(raw)?;
    if !j.at("meets").is_boolean() || !j.at("text").is_string() || j.s("text").trim().is_empty() || (j.has("findings") && !j.at("findings").is_array()) { crate::fail!("no valid verdict"); }
    // The verdict must name the commit it reviewed; a verdict for another commit is ignored.
    if !head.is_empty() { let c = j.t("commit").trim().to_lowercase(); if c.len() < 7 || !head.to_lowercase().starts_with(&c) { return Ok(None); } }
    let sentences = |xs: &Value| -> Vec<String> { xs.as_array().map(|a| a.iter().map(|x| crate::text::squash(&text(x))).filter(|s| !s.is_empty()).take(8).collect()).unwrap_or_default() };
    // `risky`, when given (a work review must, review_once.rs): the board's own last sentence then ends `text`.
    let risky = j.at("risky").as_bool();
    // Every one named, none cut off or dropped; named under a false answer still counts: risky is never assumed away.
    let rc = j.at("riskyChanges");
    if j.has("riskyChanges") && !rc.as_array().is_some_and(|a| a.iter().all(|x| x.is_string())) { crate::fail!("no valid verdict: `riskyChanges` is a list of sentences"); }
    let changes: Vec<String> = rc.as_array().map(|a| a.iter().map(|x| crate::text::squash(x.as_str().unwrap_or(""))).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
    if j.has("risky") && (risky.is_none() || (risky == Some(true) && changes.is_empty())) { crate::fail!("no valid verdict: `risky` is true or false, and a true one names its `riskyChanges`"); }
    let said = crate::text::squash(j.s("text"));
    let said = if risky.is_some() { risky_end(&said, &changes) } else { said };
    let (findings, left) = gate::settle(j.at("meets").as_bool(), &gate::read_findings(j.at("findings")), earlier, used);
    let mut out = obj(json!({ "meets": j.at("meets"), "findings": findings, "left": left, "text": said, "checks": sentences(j.at("checks")), "limitations": sentences(j.at("limitations")),
        "listed": j.at("artifacts"), "artifacts": [], "at": crate::clock::now(), "head": if head.is_empty() { Value::Null } else { json!(head) } }));
    if j.at("offPlan").is_array() { out.put("offPlan", sentences(j.at("offPlan"))); }
    if risky.is_some() { out.put("risky", !changes.is_empty()); out.put("riskyChanges", changes); }
    Ok(Some(out))
}
