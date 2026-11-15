# A row's kind, and where it was handed to

**Every task row says what kind of task it is**, in a dimmed chip before its name. Nobody tags a task by hand: the
board reads the kind off what the task already has.

```
 ▾ working
   ◐ PRJ-26 [自動] heavy-run for the tests and the build              ███░░░░░ 40%   2h
   ◐ HCM-138 [運用] [応募] three seats: search, draft, send                          5h
   ◐ HCM-201 [運用] [順番待ち] heavy-run for herdr-projects… [委任先 PRJ-26・作業中]    1d
   ◐ PRJ-31 [調査] Why is the build slow? [research]                                 4m
```

| Chip | The task | Read off |
| --- | --- | --- |
| `[調査]` | research only: its result is a report | the setting `research.only`, as `[research]` says ([research](research.md)) |
| `[自動]` | an agent works it in a worktree of its own | a `worker.*` setting of its own, or a worktree, or its branch (which stays once the worktree is removed) |
| `[運用]` | neither: a ledger you work through, a resident seat | everything else |

`[research]` stays as it was: a research-only row has both. A task added and not started yet, with no `worker.*`
setting, is `[運用]` until an agent is started on it.

## Where it was handed to

**A task whose message says it was handed to another task names that task and where it stands**, after its name:
`[委任先 PRJ-26・作業中]`. That is how a ledger row (say HCM-201) shows, without opening anything, how the work it
handed to another repo's task (PRJ-26) is going.

The board reads it off the task's own message (the `--msg` its agent or its coordinator last gave): the task numbers right before
`に委任` (a few words may stand between, as in `に仕様ごと委任済み`).

| The message | Shown |
| --- | --- |
| `PRJ-26に委任` | `[委任先 PRJ-26・作業中]` |
| `HCM-246・HCM-247に委任` | `[委任先 HCM-246・未着手, HCM-247・完了]` |
| `TKN-38(auto-teikeikun/38)に仕様ごと委任済み` | `[委任先 TKN-38・完了]` |
| `P5=FLS-70 … P5のみ委任` | nothing: no number stands right before `に委任` |

Where it stands is the other task's group on the board: 作業中 (working), 要対応 (needs you), 承認待ち (ready to
approve), 完了 (done), 未着手 (to do), or アーカイブ済み. A number of a task the board does not list (archived and out
of view, or no repo has its key) is named without a state. A row of another machine names none: its numbers are that
machine's.

When the message changes, so does the tag: a later message that no longer says `に委任` takes it off.

Both are in the `?` menu, under **on a row**.
