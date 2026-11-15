//! What `approve --expect-review N --expect-head SHA` holds a task to: the review and the commit the caller judged
//! before it said yes (auto-ship). Either one moved since, and nothing is merged: not at the yes, and not later, when a
//! repair or main merged in would put another commit or another review under that yes.
use crate::err::Res;
use crate::json::*;
use crate::store::reviews::review_number;
use crate::store::tasks::{one_line, Task};
use std::path::Path;

/// Nothing expected: `approve` as it always was.
#[derive(Default)]
pub struct Expect { pub review: Option<i64>, pub head: Option<String> }

impl Expect {
    pub fn any(&self) -> bool { self.review.is_some() || self.head.is_some() }
    /// Refused unless the task waits to merge, on that review (its round, as `details` and `list --json` say it) and
    /// that reviewed commit (its full sha, or at least 7 of its first characters).
    pub fn check(&self, t: &Task) -> Res<()> {
        if !self.any() { return Ok(()); }
        let id = t.s("id");
        if t.s("stage") != "approval" { crate::fail!("{id} was not merged: it is not waiting for an approval to merge (stage {}).", t.s("stage")); }
        if let Some(n) = self.review.filter(|n| *n != review_number(t)) { crate::fail!("{id} was not merged: its review is round {} now, not {n}. Look at it again, then approve.", review_number(t)); }
        let head = t.s("reviewedHead");
        if let Some(h) = self.head.as_deref().filter(|h| h.len() < 7) { crate::fail!("{id} was not merged: --expect-head {h} is too short to name one commit: give 7 characters of it or more."); }
        if let Some(h) = self.head.as_deref().filter(|h| !head.starts_with(&h.to_lowercase())) {
            crate::fail!("{id} was not merged: its reviewed commit is {} now, not {h}. Look at it again, then approve.", if head.is_empty() { "none" } else { head });
        }
        Ok(())
    }
    /// The same against what is there, beyond the board's record: the branch in its worktree and the pull request's
    /// head on GitHub must both still be the reviewed commit. Asked before the task's lock, as gh may take a while; the
    /// merge itself asks GitHub again.
    pub fn live(&self, t: &Task) -> Res<()> {
        if !self.any() { return Ok(()); }
        let (id, reviewed) = (t.s("id"), t.s("reviewedHead"));
        let moved = |what: &str, now: &str| -> Res<()> { if now != reviewed { crate::fail!("{id} was not merged: {what} is at {now} now, not at its reviewed commit {reviewed}. Look at it again, then approve."); } Ok(()) };
        let tree = Path::new(t.s("worktree"));
        if t.b("worktree") && tree.exists() { moved("its branch", &crate::flow::git::head_of(tree))?; }
        if t.b("pr") {
            let out = crate::flow::git::gh(&["pr", "view", &crate::flow::git::pr_number(t.s("pr")), "--json", "headRefOid"], &crate::store::repos::folder_of(id), 60000);
            let now = serde_json::from_str::<Value>(&out.stdout).map(|v| v.t("headRefOid")).unwrap_or_default();
            if now.is_empty() { crate::fail!("{id} was not merged: GitHub did not say its pull request's head, so it cannot be held to the reviewed commit: {}", one_line(out.stderr.trim(), 300)); }
            moved("its pull request", &now)?;
        }
        Ok(())
    }
    /// What the approved merge stays held to (`mergeIntent.expect`): the reviewed commit and the review's round now.
    pub fn bound(&self, t: &Task) -> Value { json!({ "head": t.at("reviewedHead"), "review": review_number(t) }) }
}
/// An approved merge held to what was judged, once what would land is another commit (`head`) or its review another
/// round: the yes no longer stands, and the task waits for approval again with the new ones (pipeline_end.rs).
pub fn broken(t: &Task, head: &str) -> bool { let e = t.at("mergeIntent").at("expect"); e.is_object() && (e.s("head") != head || e.i("review") != review_number(t)) }
