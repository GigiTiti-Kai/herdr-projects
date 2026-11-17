# A row's kind, and where it was handed to

**Every task row says what kind of task it is**, in a dimmed chip before its name. Nobody tags a task by hand: the
board reads the kind off what the task already has.

```
 ▾ working
   ◐ APP-26 [自動] Move the logs to their new place                  ███░░░░░ 40%   2h
   ◐ OPS-38 [運用] Weekly invoices, worked by hand                                  5h
   ◐ OPS-41 [運用] [順番待ち] Move the logs… [委任先 APP-26・作業中]                1d
   ◐ APP-31 [調査] Why is the build slow? [research]                                4m
```

| Chip | The task | Read off |
| --- | --- | --- |
| `[調査]` | research only: its result is a report | the setting `research.only`, as `[research]` says ([research](research.md)) |
| `[自動]` | an agent works it in a worktree of its own | a `worker.*` setting of its own, or a worktree, or its branch (which stays once the worktree is removed) |
| `[運用]` | neither: a ledger you work through, a resident seat | everything else |

The chip is part of the name: where a row is tight, it counts toward the 40 cells a name keeps before the columns on
the right give way, so on a narrow list a column goes up to 7 cells sooner than it did. `[research]` stays as it was:
a research-only row has both. A task added and not started yet, with no `worker.*`
setting, is `[運用]` until an agent is started on it.

## Where it was handed to

**A task whose message says it was handed to another task names that task and where it stands**, after its name:
`[委任先 APP-26・作業中]`. That is how a ledger row (say OPS-41) shows, without opening anything, how the work it
handed to another repo's task (APP-26) is going.

The board reads it off the task's own message (the `--msg` its agent or its coordinator last gave): the task numbers right before
`に委任` (a few words may stand between, as in `に仕様ごと委任済み`).

| The message | Shown |
| --- | --- |
| `APP-26に委任` | `[委任先 APP-26・作業中]` |
| `OPS-46・OPS-47に委任` | `[委任先 OPS-46・未着手, OPS-47・完了]` |
| `APP-38(app/38)に仕様ごと委任済み` | `[委任先 APP-38・完了]` |
| `S2=DOC-7 … S2のみ委任` | nothing: no number stands right before `に委任` |
| `APP-26に委任することは、拒否された。手で続ける` | nothing: its sentence (up to the next `。`, `!` or `?`) says no after `委任`: `ない`, `ません`, `ず`, `禁止`, `拒否`, `取り消`, `やめ` … |
| `APP-26に委任した。返事はまだない` | `[委任先 APP-26・作業中]`: the no is in the next sentence |

A wrong target is worse than none: a sentence such as `APP-26に委任済み、手では回さない` names nothing either.

A task is named by the key its repo goes by now: a message written while the repo went by another key (`OLD-26`)
still names that task, as every command takes it, and the tag says its key of now (`[委任先 APP-26・…]`).

Where it stands is the other task's group on the board: 作業中 (working), 要対応 (needs you), 承認待ち (ready to
approve), 完了 (done), 未着手 (to do), or アーカイブ済み. A number of a task the board does not list (archived and out
of view, or no repo has its key) is named without a state. A row of another machine names none: its numbers are that
machine's. A task of the same repo is named too. Where the name is cut, the tag stays whole and the title gives way.

When the message changes, so does the tag: a later message that no longer says `に委任` takes it off.

Both are in the `?` menu, under **on a row**.

A row said to be at work with no agent of the board's on it (most `[運用]` rows) has `○` for a mark and says
`エージェントなし` first, as any row no agent is on does; a day after it began to be worked by hand it stands under its own
heading, `運用中` ([運用中](ongoing.md)).
