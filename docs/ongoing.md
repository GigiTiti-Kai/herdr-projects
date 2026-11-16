# 運用中, and a row no agent works

**A row shows whether an agent of the board's is on it.** An agent's row has the moon (`◐`), and its bar, its time
left, its review round, its pull request, how long it has worked, and under its name what it is doing now. A task that
is only said to be at work (`herdr-projects report <id> --state working`, by you, a coordinator or a resident seat
such as Hermes) has no agent of the board's: its row has a ring (`○`, amber) where the moon would be, and its line 2
says so first.

```
 ▾ working
   ◐ APP-29 [自動] Check the hooks' trust                   ████░░░░ 50% 6m left  R#6 PR#30  40m
            Looking at F9's risky read, and running the tests again
   ○ OPS-54 [運用] Hand the hook to the seat                                                     6m
            エージェントなし · Pulled; now the hook and the wording
 ▾ 運用中
   ○ OPS-138 [運用] Walk the job boards every hour                                               20h
            エージェントなし · Round 7: 12 read, 2 kept
 ▾ to do
   ○ OPS-60 [運用] Not begun                                                                 1d ago
```

- **The board cannot tell who is behind such a row**, or whether anyone is: only that no agent of its own is. So it
  says `エージェントなし` (no agent), never that nobody is on it. Its time on the right is how long ago it was last said
  to be at work (each report starts it again).
- With nothing said, line 2 is `エージェントなし` alone.

## 運用中

**A row no agent of the board's works, added more than a day ago, stands under its own heading, `運用中`, right after
`working`.** Those are the rows whose end is not coming: a seat's round that runs on, a ledger kept by hand. `working`
keeps what is on its way to done: the agents' rows, the reviewer's, the board's own steps, the queued ones, and a row
said to be at work for less than a day.

- The day is counted from when the task was added. A task that waited long in to do goes under `運用中` as soon as it is
  said to be at work.
- It moves by itself, a day after it was added; a row that waits for you, is done or archived never stands there.
- The heading is in the brighter grey (Herdr's `subtext0`). It folds as every heading does
  ([list headings](list-headings.md)).
- Grouped by repo or by machine, where there are no status headings, these rows come after the working ones.
- The filter row's `Working` counts and shows `working` only; the rows under `運用中` are under `All`.
- The task stays `working` everywhere else: in its panel, in another task's `[委任先 …]` tag, in Herdr's sidebar.
- The web board shows the same heading and the same rows.

The `?` menu has the ring's line, under **on a row**. The chip before a name (`[運用]` …) is [the row's kind](row-kinds.md).
