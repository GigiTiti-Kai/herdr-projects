//! What `approve --expect-review N --expect-head SHA` holds a task to: the review and the commit the caller judged
//! before it said yes (auto-ship). Either one moved since, and nothing is merged.
use crate::err::Res;
use crate::json::*;
use crate::store::reviews::review_number;
use crate::store::tasks::Task;

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
}
