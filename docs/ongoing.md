# 運用中, and a row no agent works

**A row shows whether an agent of the board's is on it.** An agent's row has the moon (`◐`), and its bar, its time
left, its review round, its pull request, how long it has worked, and under its name what it is doing now. A task that
is only said to be at work (`herdr-projects report <id> --state working`, by you, a coordinator or a resident seat)
has no agent of the board's: its row has a ring (`○`, amber) where the moon would be, and its line 2
says so first.

```
 ▾ working
   ◐ APP-12 [自動] Speed up the image cache                 ███░░░░░ 30% 9m left  R#2 PR#14  25m
            Measuring the cache misses after the change
   ○ OPS-21 [運用] Note the new rota                                                            12m
            エージェントなし · Asked the team for Friday's slot
 ▾ 運用中
   ○ OPS-7  [運用] Read the shared inbox each morning                                            3h
            エージェントなし · Morning pass: 4 mails, 1 answered
 ▾ to do
   ○ OPS-30 [運用] Not begun                                                                 1d ago
```

- **The board cannot tell who is behind such a row**, or whether anyone is: only that no agent of its own is. So it
  says `エージェントなし` (no agent), never that nobody is on it. Its time on the right is how long ago it was last said
  to be at work (each report starts it again).
- With nothing said, line 2 is `エージェントなし` alone.
- **A row that waits for you says it too**: a question, an OK or an error on a task no agent is on reads
  `エージェントなし · Which floor first?`; on one whose agent asks, only the question. An agent is on a task when one is
  named on it or its pane is linked to it. A queued row says it waits, not this: its agent is on its way.

## 運用中

**A row worked by hand for more than a day stands under its own heading, `運用中`, right after `working`.** Those are the rows whose end is not coming: a seat's daily pass that runs on, a ledger kept by hand. `working`
keeps what is on its way to done: the agents' rows, the reviewer's, the board's own steps, the queued ones, and a row
said to be at work for less than a day.

- **The day is counted from when it began to be worked by hand**: its first `report --state working` out of to do (or
  out of done). Its later reports and its waits for you keep that time; put back in to do and begun again, it counts
  from then. How long it waited in to do never counts. A task already at work by hand before the board kept that time
  counts from when it was added.
- It moves by itself, a day after it began; a row that waits for you, is done or archived never stands there.
- The heading is in the brighter grey (Herdr's `subtext0`). It folds as every heading does
  ([list headings](list-headings.md)).
- Grouped by repo or by machine, where there are no status headings, these rows come after the working ones.
- The filter row's `Working` counts and shows `working` only; the rows under `運用中` are under `All`.
- The task stays `working` everywhere else: in its panel, in another task's `[委任先 …]` tag, in Herdr's sidebar.
- The web board shows the same heading and the same rows.

The `?` menu has the ring's line, under **on a row**. The chip before a name (`[運用]` …) is [the row's kind](row-kinds.md).
