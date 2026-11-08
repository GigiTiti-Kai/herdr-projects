# A record-only closeout before a space closes

Off unless you turn it on: `herdr-projects set workspace.closeout true` (for one repo: `--repo <repo>`).

When a task is done and its space is about to be closed ([when a task is done](how-work-gets-done.md#when-a-task-is-done)),
the board first types this into its working agent, once, as a turn of its own:

```
/session-close 記録だけの session-close。ship・push・merge・PR・再 review・削除はしない。自分が所有し現在の権限で止められる背景処理（監視・cron・接続）だけ止めてから報告。他者・共有・所有不明のものは止めず、巻き込む可能性があれば残して理由付きで保留。終了してよいと判断した時だけ、返事の最後の行に `safe to Exit` とだけ書く（それ以外は書かない）
```

(`$session-close` for Codex.) The task stays done: that turn is never taken for a finish, so nothing is checked,
reviewed, pushed or merged again, and an approval or a question of another task is not touched. Only the worker's own
pane gets it, and a coordinator's before it is stopped or begun anew (`coordinator stop`, `coordinator new`; a
restart keeps its conversation and needs none): no reviewer, Watcher or plan writer. Until a coordinator's closeout is
through, its stop waits; held, the stop is not carried out, the coordinator keeps running and says why, and its
record stays (a stop asked again goes on from it, an `open` in between included: the same save is never asked for
twice).

Each step is proved apart, on its own pass, and nothing is ever typed twice:

1. **Saved.** Agent Memory's queue (`$XDG_STATE_HOME/agent-memory/queue.sqlite3`, read only, with `sqlite3`) holds a
   job accepted since the closeout was sent, for that agent's session, filed from the task's worktree, and not given up
   on. It is looked up again on every pass until the space is closed: a job given up on since proves nothing.
2. **Exit, decided now.** The agent's own last reply, read from its saved conversation, is its reply to the closeout
   (the turn the closeout's own words began, after it was sent, the same reply at every look) and ends with a plain line
   `safe to Exit` of its own, at the line's start (not indented), said once (a decision taken back, quoted, `unsafe`, `not safe` or an Exit blocked is none). Right before, the agent is at
   rest, nothing runs under it, and the session and branch tip are the ones it was sent for. Its processes are noted,
   and `/exit` is typed, once.
3. **The agent gone.** Herdr sees no agent in the pane any more, the processes that were the agent have ended,
   nothing runs at its shell, and its shell's last line is exactly the prompt it showed before the agent's program
   was started (so nothing is typed after it; a launch line scrolled away holds it). Its session, branch, worktree
   HEAD and changes, last reply and save are looked at again then too.
4. **The space closed.** The clean-up as ever ([when a task is done](how-work-gets-done.md#when-a-task-is-done)),
   and `pane_closed` only once Herdr says there is no such pane any more (for a coordinator: else it says so).

What it keeps on the task (`closeout`): `phase` (`sending`, `checkpointing`, `saved`, `exiting`, `closing`,
`agent_exited`, `pane_closed`; what is typed is written down as on its way first, so a record that could not be
written after it is never sent again: it is held),
when it was sent and when `/exit` was, the session, the branch tip and the job. Until it is through, the task's line says
what it waits for, and the clean-up is tried again by itself.

**What cannot be proved closes nothing and types nothing more.** The space and worktree stay, and the task says why,
starting with `closeout 未確認`:

- new input after it was sent: a reply that is not the closeout's, a turn beyond its own, another session, the branch
  or the worktree's own HEAD moved, or its changes changed (by their contents, untracked files included);
- its reply gives no Exit decision (a last line `safe to Exit`), such as an Exit blocked or taken back;
- another terminal or another session than its agent's in the pane, as Herdr shows it (or Herdr not saying which);
- an input box not of the one shape known to be empty: the screen's last box, framed with its corners, holding one
  bare prompt mark and nothing else. Words in it, a placeholder, a rule or another shape type nothing. So an agent
  whose empty box looks otherwise (Codex's unframed one) is never closed out: it waits for you. Every text goes in only on the last look right before it, at the same session and terminal;
- new input after `/exit` went in (words that only name `/exit`, or follow its tag, included), another terminal in its pane by then, or
  its save no longer accepted then;
- no job accepted within `workspace.closeoutWait` (10 minutes) of the send (one accepted later counts as none), or a
  queue that cannot be read;
- its turn still under way, herdr not answering, its saved conversation not readable, or the agent still there after
  `/exit`, once that same wait has passed since the board first looked (before anything was sent, too);
- its agent not plainly at rest (a dialog, or a status Herdr cannot place), or not in its pane before it was told to
  exit (a crash is never a finished closeout);
- a tracked file changed that git is told to overlook is still a change (every file is read from disk itself), and
  so is what is staged only (the index is part of the version);
- its agent's session not known, or its worktree that cannot be read (a file nobody may read, a folder git skips
  with a warning: its version is unknown, never the same one). A file's kind counts too: a link turned into a file of
  the same text is another version.

Such a task is no longer tried again by itself: archive it to close its space. Archiving a task does not go through
the closeout.
