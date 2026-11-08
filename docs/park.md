# Parking a task that waits for you

Off unless you turn it on, and never done by itself: `herdr-projects set workspace.park true`, then
`herdr-projects park <id>` for each task you choose.

A task that waits for your approval (`approval`) or your answer (`asks`) keeps its agent running. Parking ends that
agent, and keeps everything else:

1. Its agent gets the [record-only closeout](closeout.md), the same steps as a done task's: the `/session-close` that
   ships nothing, its save accepted by Agent Memory, its own `safe to Exit`, `/exit`, its process ended and its shell
   at its prompt. Nothing is typed twice.
2. The task stays where it was: its stage, the commit that waits for your approval, its review, its question and the
   words waiting for its agent. Its pane stays open at its shell, and its line ends with `Parked: …`.

While it is under way, the closeout's own turns are never taken for yours (no review goes stale, no approval is
asked again), and nothing else goes in: a reply, an approval, a review sent back, a fix, or the board's own turns
(a check, a conflict) are refused or wait until it is parked or held, and so is a report of yours. Its own agent
reporting itself blocked (it cannot exit) holds the park with those words; the task keeps what it waits on.
Only a worker is parked: a plan writer asking you something is refused.

Once it is parked:

- **Approve** merges as ever. No agent is needed or started; the clean-up after the merge finds the agent already
  gone and types nothing more. A conflict found then has no agent to go to: you are asked.
- **Going into the task, a reply, a review sent back, a fix, a restart** bring its agent back the way an exited agent
  comes back, in its own conversation (`--resume`). A reply or a fix then works as ever: the task is at work again.
  A conversation that cannot be resumed, or a worktree gone, starts nothing, never a fresh one: the task says why
  (going into it too), and the words wait. Another agent than its own in its pane gets nothing: a reply is refused.
  Then archive it, or give up the conversation with `herdr-projects park <id> --forget` and delegate it again.
- The board's own turns (a check failure, a pull request repair) type nothing.
- Once its space is closed after the merge, that round is over: a new round starts as ever.

**What cannot be proved is held**, as for a closeout (`closeout 未確認: …`): the task is then an ordinary one again,
with nothing more typed. It still waits for you as it did, its closeout's own turns not taken for yours; held for new
input (a turn of yours, another session, its work moved), that input counts as ever, and the approval is asked again.
Held before anything went in (a dialog, words in its input box), `park` is refused and leaves nothing behind. An agent gone before it was told to exit is a crash, never a park;
herdr not answering only waits. A held park, or one the task moved on from, counts for nothing at its clean-up: that
closeout starts anew.

Not done: parking by itself (on a timer, when idle, by a layout restore), parking a reviewer, watcher, planner or
coordinator, and bringing a parked agent back by itself.
