# A record-only closeout before a space closes

Off unless you turn it on: `herdr-projects set workspace.closeout true` (for one repo: `--repo <repo>`).

When a task is done and its space is about to be closed ([when a task is done](how-work-gets-done.md#when-a-task-is-done)),
the board first types this into its working agent, once, as a turn of its own:

```
/session-close 記録だけの session-close。ship・push・merge・PR・再 review・削除はしない。自分が所有し現在の権限で止められる背景処理（監視・cron・接続）だけ止めてから報告。他者・共有・所有不明のものは止めず、巻き込む可能性があれば残して理由付きで保留
```

(`$session-close` for Codex.) The task stays done: that turn is never taken for a finish, so nothing is checked,
reviewed, pushed or merged again, and an approval or a question of another task is not touched. Only the worker's own
pane gets it: no reviewer, Watcher or plan writer.

The space is closed only once Agent Memory's queue (`$XDG_STATE_HOME/agent-memory/queue.sqlite3`, read only, with
`sqlite3`) holds a job accepted since then for that agent's session, filed from the task's worktree, and not given up
on. Until then the task's line says what it waits for, and the clean-up is tried again by itself.

What it keeps on the task (`closeout`): `phase` (`checkpointing` once sent, `saved` once accepted), when it was sent,
the session, the branch tip and the job. A record for another session or tip, or a turn of the agent after the save,
proves nothing: it is looked at again, and the closeout goes in again. The same session and tip is never sent twice.

**What cannot be proved closes nothing and types nothing more.** The space and worktree stay, and the task says why,
starting with `closeout 未確認`:

- no accepted job within `workspace.closeoutWait` (10 minutes), or a queue that cannot be read;
- its agent is not plainly at rest (a dialog, or a status Herdr cannot place): nothing is typed at all;
- its agent's session is not known.

Such a task is no longer tried again by itself: archive it to close its space. Herdr not answering, a turn under way or
a command still running under the agent only make it wait. Archiving a task does not go through the closeout.
