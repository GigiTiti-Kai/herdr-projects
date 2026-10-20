# A tab in your space

**A task can work in a tab of a herdr space you already have open, instead of a space of its own.** Each task usually
gets a herdr space of its own, and herdr's agent list shows it as a group of its own. With `workspace.place` set to
`tab`, the task's agent opens a tab in your space of the same repo instead, and is listed with that space.

```
# every task of one repo in a tab of your space
herdr-projects set workspace.place tab --repo my-app

# one task only
herdr-projects delegate 12 --workspace.place tab
```

## Which space

A space of the task's repo that no task owns: the one you are looking at, else the one named after the repo (herdr names
a space after its first folder), else any other. A space opened at a worktree of the repo counts too. No such space
open: the task gets a space of its own, as with `space`. A task done by [several attempts](attempts.md) always gets one: its attempts share it.
One that was already in a tab of yours when its attempts were made (`attempts.by` raised while its plan waited) keeps
them there: tabs `PRJ-12 Agent A`, `PRJ-12 Agent B` and `PRJ-12 Picker` in your space, and after the pick only the other
attempts' own panes close.

## What the task has there

- **Its worktree is its own**, as ever: the same branch, check and clean-up as a task in a space of its own.
- **Its tabs are named after it**: `PRJ-12 Agent`, `PRJ-12 Reviewer #1`, `PRJ-12 Planner`, so you can tell them from
  yours.
- **Started again** (a reply after its terminal was closed, or going into it): a new tab in the same space, or in
  another found as at the start when that one is gone.

## What it never does to your space

- It never splits, types into or closes a pane of yours, and never closes the space or gives it another name.
- When the task is done, or archived, only its own panes close, and with them its tabs. A pane you split off inside its
  tab is yours: it stays, and so does that tab.
- Clean-up waits for what runs in the task's own tabs, not for what runs in yours.
