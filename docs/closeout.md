# A record-only closeout before a space closes

Off unless you turn it on: `herdr-projects set workspace.closeout true` (for one repo: `--repo <repo>`).

When a task is done and its space is about to be closed ([when a task is done](how-work-gets-done.md#when-a-task-is-done)),
the board first types this into its working agent, once, as a turn of its own:

```
/session-close 記録だけの session-close。ship・push・merge・PR・再 review・削除はしない。自分が所有し現在の権限で止められる背景処理（監視・cron・接続）だけ止めてから報告。他者・共有・所有不明のものは止めず、巻き込む可能性があれば残して理由付きで保留
```

(`$session-close` for Codex.) The task stays done: that turn is never taken for a finish, so nothing is checked,
reviewed, pushed or merged again, and an approval or a question of another task is not touched. Only the worker's own
pane gets it, and a coordinator's before it is stopped or begun anew (`coordinator stop`, `coordinator new`; a
restart keeps its conversation and needs none): no reviewer, Watcher or plan writer. Until a coordinator's closeout is
through, its stop waits; held, the stop is not carried out, the coordinator keeps running and says why.

Each step is proved apart, on its own pass, and nothing is ever typed twice:

1. **Saved.** Agent Memory's queue (`$XDG_STATE_HOME/agent-memory/queue.sqlite3`, read only, with `sqlite3`) holds a
   job accepted since the closeout was sent, for that agent's session, filed from the task's worktree, and not given up
   on. It is looked up again on every pass until the space is closed: a job given up on since proves nothing.
2. **Exit, decided now.** The agent's own last reply, read from its saved conversation, is its reply to the closeout
   (begun after it was sent, the same reply at every look) and says `safe to Exit`. Right before, the agent is at
   rest, nothing runs under it, and the session and branch tip are the ones it was sent for. Its processes are noted,
   and `/exit` is typed, once.
3. **The agent gone.** Herdr sees no agent in the pane any more, the processes that were the agent have ended, and
   nothing runs at its shell.
4. **The space closed.** The clean-up as ever ([when a task is done](how-work-gets-done.md#when-a-task-is-done)).

What it keeps on the task (`closeout`): `phase` (`checkpointing`, `saved`, `closing`, `agent_exited`, `pane_closed`),
when it was sent and when `/exit` was, the session, the branch tip and the job. Until it is through, the task's line says
what it waits for, and the clean-up is tried again by itself.

**What cannot be proved closes nothing and types nothing more.** The space and worktree stay, and the task says why,
starting with `closeout 未確認`:

- new input after it was sent: a reply that is not the closeout's, a turn beyond its own, another session, the branch
  moved;
- its reply gives no Exit decision (`safe to Exit`), such as an Exit blocked;
- no accepted job within `workspace.closeoutWait` (10 minutes), or a queue that cannot be read;
- its turn still under way, herdr not answering, its saved conversation not readable, or the agent still there after
  `/exit`, once that same wait has passed since the board first looked (before anything was sent, too);
- its agent not plainly at rest (a dialog, or a status Herdr cannot place), or not in its pane before it was told to
  exit (a crash is never a finished closeout);
- its agent's session not known.

Such a task is no longer tried again by itself: archive it to close its space. Archiving a task does not go through
the closeout.
