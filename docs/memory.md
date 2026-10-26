# Memory

**A new agent does not start on a machine with too little memory.** Each agent's program (and the builds it runs)
takes memory, and on a small machine (a WSL with 8 GB) one agent too many can freeze all of them. So before the board
starts a program, it reads what this machine has available (`MemAvailable` in `/proc/meminfo`), and with less than
`board.memoryFree` GB (3) the program waits instead.

## What waits

- **A task about to start** (delegated, retried, started by autopilot or a coordinator): it stays in the queue, never
  failed, says `waiting for memory, Q#1` on the board, in `list`, `details` and herdr's sidebar, and its line says how
  much there is: `Memory nearly full on vm-str: 1.8 GB free < 3.0 GB (board.memoryFree); it starts by itself once
  there is room.` The keeper's next pass starts it once there is room, in its place in the queue.
- **A reviewer, an agent started again, or a fallback agent taking over**: it waits where it is, with the same line,
  and starts once there is room. A review's time (`review.timeout`) only starts with its reviewer.
- **Going into a task whose agent exited**: its space opens at once, and its agent is started again in the same
  conversation by itself once there is room, the task's row saying the wait meanwhile.

Nothing that already runs is stopped, and there is no limit on how many agents run (that is `board.agents`).

## The setting

```sh
herdr-projects set board.memoryFree 4   # wait under 4 GB available
herdr-projects set board.memoryFree 0   # never wait
herdr-projects show                     # what it is now
```

On a machine without `/proc/meminfo` (macOS) nothing waits. Waiting for room on a disk is the same kind of wait:
[disk space](disk-space.md).
