# The whole check on GitHub

**In this repo the board's whole check runs on GitHub Actions, not on this machine.** Its `check.command` is
`sh scripts/check-cloud.sh` (`.herdr-projects.json`). It pushes the task's branch, waits for the run of
`.github/workflows/ci.yml` on that commit and passes or fails with it. The machine's cores and memory stay free for
the agents. The shorter check of a fix round (`check.again`, [shorter checks](shorter-checks.md)) still runs here.

## What it waits for

- **The whole run.** That is the Linux job, which runs the whole check, and the macOS job. `gh run watch` cannot wait
  for one job alone, so it does not try.
- **One fixed run.** It looks for the run of `ci.yml` that the push started: the same commit, the same branch, event
  `push`. Its id, attempt and workflow are kept from then on.
- **Only a success of that very run counts.** The run must have ended `success` on the commit the check started from,
  as the same attempt and workflow, and that commit must still be checked out when it ends.
- **No longer than the board allows.** The board hands it `check.timeout` (`HERDR_PROJECTS_CHECK_TIMEOUT`, in
  seconds). It ends a minute before that, with its own word, so the board never has to stop it.

## What comes back

| The run | The check | The task |
|---|---|---|
| passed | passes, with the run's address | goes on to its review |
| a step failed | fails: each failed job and step, the last 60 lines of its log, the run's address | goes back to its agent, as a failed check does |
| GitHub gave no answer | fails on one line: `The check could not run on GitHub: <why> · <address>` | waits for you, and is not sent back |

"No answer" covers a run that never started, was cancelled or skipped, or ended with no step failed. It also covers gh
not reaching GitHub, the branch failing to push, the branch moving on during the run, and the time running out. None of
these says anything about the code, so none of them uses up one of the agent's `review.rounds`.

To go on:

- Run it again: `herdr-projects retry <id>`. A run that has ended stays the answer for its commit, so for one that
  ended (cancelled, say) run it again on GitHub first: `gh run rerun <run> -R <owner>/<repo>`, which the line names.
  The retry then waits for that new attempt.
- Or run the whole check on this machine instead:
  `herdr-projects set check.command "CHECK_LOCAL=1 sh scripts/check-cloud.sh" --repo <repo>`, then retry. Undo it with
  `herdr-projects unset check.command --repo <repo>`.

## The made tables

The check writes the made tables (`docs/keys.md`, `docs/settings.md`, `docs/agents.md`) when they are out of date.
Nothing written on GitHub is sent back. So the Linux job has one more step, `The made tables are committed`, which
fails when the check changed any file in git and names those files. That failure goes back to the agent like any
other: it runs `npm run check` in its worktree and commits what it wrote.

## Who it runs for

Only the board's own check goes to GitHub. The board alone hands a check its progress file
(`HERDR_PROJECTS_PROGRESS`), and the script uses that to tell. Run by an agent or by you, or with `CHECK_LOCAL=1`,
`sh scripts/check-cloud.sh` is the whole check on this machine (`scripts/check.sh --all`), so it never pushes.
It refuses a folder with changes that are not committed, because the run checks the commit, not the folder.

While it waits, the task shows `Waiting for CI · GitHub run <id>` with the time spent
([the check's progress](check-progress.md)). The pull request's own checks before a merge are shown as they always
were (`merge.checks: github`, [how work gets done](how-work-gets-done.md)).

## One heavy job at a time

Where the machine has the shared lock `heavy-run` installed, the whole check and the shorter one run under it, one at a
time on the whole machine and under its memory caps. Until the lock is free the task shows
`Waiting for its turn`. A check of a few areas by name (`npm run check board-panel`) does not take it. Waiting for
GitHub does not hold it either.
