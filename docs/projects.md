# The projects you hold

**Under the filter row, over the list's first heading, a band names the projects you hold**: what you are working on
now, as a map, not as the pile of its tasks. Each project says how many open tasks it has and where the most urgent of
them stand, in that place's colour.

```
 All 31   Needs 3   Working 4   Ready 1   To do 23
 プロジェクト  garden 7 要対応1   kitchen 4 承認待ち1   skills 6 未着手   house 2 未着手
 ▾ needs you
```

- **Open tasks only**: needs you, ready to approve, working, 運用中 and to do. Done and archived ones are not counted.
- **What it says after the count** is the most urgent place one of its tasks stands: `要対応` (needs you), `承認待ち`
  (ready to approve), `作業中` (working), `運用中` ([運用中](ongoing.md)), `未着手` (to do). Where not all of them stand
  there, how many do: `要対応1` is one of the seven.
- **The most urgent first**, then the one with more open tasks, then by name.
- **Every task in view counts, whatever the filter**: under `Working` the list shows fewer rows, not fewer projects. With
  every machine shown, a project's tasks on each machine count together.
- **It shows with two projects or more**: with one, the header names it already. Not in the archive, and not where the
  list part is under 12 lines. It takes a sixth of the list's lines at most; what does not fit is counted at its end
  (`+3`). The web board has the same band under its filter row, wrapped to as many lines as it needs.

## Which project a task is under

A project here is a theme of your work, which may span repos, and one repo may hold several: not a repo, as the top
row's `9 projects` and a project's coordinator are. You name them in one setting of your own, `board.projects`
([settings](settings.md)):

```sh
herdr-projects set board.projects "skills=skill|cleanup garden=lawn|fence|garden kitchen house"
```

- **Each entry is `name=word|word`**, or `name` alone, which is its own word. Names and words have no spaces.
- **A task is under the first entry with a word that is its repo, or that stands in its title** (letters in any case).
  Put the narrow ones first: `skills=skill` before `house`, so that a skill task in the house repo is under skills.
- **A task no entry takes is under its repo.** With `none` (as it comes), every project is a repo. A last entry whose
  word is a repo (`errands=house`) gives what is left in that repo a name of its own.
- Nothing is tagged by hand: a task you add, or a coordinator adds, is under its project from its title at once. A
  task renamed moves.

The board reads the setting as it reads your tasks: a change shows within a second.
