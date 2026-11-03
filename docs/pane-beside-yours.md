# A pane beside yours

**A task's agent can work in a pane split off beside your own panes, so you see every agent of a repo at once.** With
`workspace.place` set to `pane`, a task works as with [`tab`](tab-in-your-space.md), in a space of its repo that you
have open, but its agent does not get a tab of its own there: its pane is split off in a tab you already have.

```
# every task of one repo beside your panes
herdr-projects set workspace.place pane --repo my-app

# everywhere
herdr-projects set workspace.place pane

# one task only
herdr-projects delegate 12 --workspace.place pane
```

## Where its pane goes

- **The space** is found as for `tab`: a space of the task's repo that no task owns, the one you look at first
  ([which space](tab-in-your-space.md#which-space)).
- **The tab** is the one you last looked at in that space, else its first. Never a tab of a task's own (a reviewer's,
  a planner's, or a task's set to `tab`) and never the board's (`herdr-projects`).
- **The pane split** is the biggest one in that tab, along its longer side: a wide pane is split to the right, a tall
  one downwards (a cell is about twice as tall as it is wide). The tab fills as a grid: one pane of yours, then two
  side by side, then one of those split in two, and so on.
- **Nothing jumps:** the split is made with `--no-focus`, so the keyboard stays where it was.
- **No such space open:** the board opens one for the repo, named after its folder, with the task's agent in its
  first pane. The tab is named after no task, and the repo's later tasks split off beside it.
- **A space with no tab of yours** (every tab in it is a task's own): a new tab is opened, herdr numbers it, and the
  repo's later tasks split off in it too.

## What the task has there

- **Its pane is named after it** in the pane's border (`PRJ-12 Agent`, with `herdr pane rename`). Your tab keeps its
  name.
- **Its reviewer and planner still get tabs of their own** in that space (`PRJ-12 Reviewer #1`, `PRJ-12 Planner`), as
  with `tab`: they come and go, and the agent's pane keeps its size meanwhile.
- **Its worktree is its own**, as ever.
- **Started again** (a reply after its terminal was closed, or going into it): split off again the same way, in the
  same space while it is still one of yours.

## What it does to your tab

- It splits one of your panes, which makes that pane smaller. It never types into a pane of yours, never closes one,
  and never renames the tab or the space.
- When the task is done, or archived, only its own pane closes. The tab stays with your panes in it.
- Clean-up waits only for what runs in the task's own pane, not for what runs in yours beside it.

A task done by [several attempts](attempts.md) still gets a space of its own. The code is in
`crates/flow/src/hosted/split.rs`.
