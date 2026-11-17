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
            エージェントなし · 報告12m前: Asked the team for Friday's slot
 ▾ 運用中
   ○ OPS-7  [運用] Read the shared inbox each morning                                            3h
            エージェントなし · 報告3h前: Morning pass: 4 mails, 1 answered
 ▾ to do
   ○ OPS-30 [運用] Not begun                                                                 1d ago
```

- **The board cannot tell who is behind such a row**, or whether anyone is: only that no agent of its own is. So it
  says `エージェントなし` (no agent), never that nobody is on it. Its time on the right is how long ago it was last said
  to be at work (each report starts it again).
- **What it was told comes after how long ago it was told**: `報告3h前: Asked the team for Friday's slot`, the same time
  as on the right, so that a message written days ago is not read as what is going on now. A pane the message names
  (`wA9:p1`) that herdr no longer has is said too: `報告1d前（wA9:p1 は今は無い）: …`. Only once herdr was read on this
  machine, and only for a pane that is gone: herdr hands a closed pane's id out again, so one that is there proves
  nothing.
- **A row whose title says whom it was handed to** (`[委任先 …]`, [the row's kind](row-kinds.md#where-it-was-handed-to))
  does not say `エージェントなし`: its work is in that task, as its first line says.
- With nothing said, line 2 is `エージェントなし` alone.
- **A row that waits for you says it too**, without the time or the seats: a question, an OK or an error on a task no
  agent is on reads `エージェントなし · Which floor first?`; on one whose agent asks, only the question. An agent is on a
  task when one is named on it or its pane is linked to it. A queued row says it waits, not this: its agent is on its
  way.

## Seats

**A row a resident seat works can say how that seat stands.** Which seats work which rows is not in any task, so you
say it once, in the setting `board.seats`: a repo, and the words of the titles of the rows its seats work
(`crowdsourcing=[応募]`). A row at work by hand whose title has one of the words then says, in place of
`エージェントなし`, how many agents herdr lists in that repo's folder (or under it) now, of each kind, and which of them
work:

```
   ○ HCM-138 [運用] [応募] A/B分けの3役…                                                          1d
            席 crowdsourcing: Hermes 6・作業中1(hermes-main) · 報告1d前（wA9:p1 は今は無い）: 23:50 wA9:p1〜p3…
```

- A seat is named by its name in herdr, or its pane (`wEN:p4`) when it has none. One that waits for input is said in red:
  `入力待ち1(hermes-cw)`. One at rest is counted, not named.
- None at all is `席なし（crowdsourcing）`, in red: the setting says seats work it, and none is there.
- The board's own agents, reviewers and coordinators in that folder are no seat: by their terminal, as herdr never hands
  one out again.
- It says where the seats are, not that they work this row: the board cannot tell which row a seat works.
- herdr not read (not running, or the row is another machine's): `エージェントなし`, as before. It changes within a
  second or two of a seat starting or ending its turn.

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
