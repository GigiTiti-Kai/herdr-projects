# One heavy job at a time

Where the machine has the shared lock `heavy-run` installed (`~/.local/bin/heavy-run`), the heavy jobs of this repo run
under it: one at a time on the whole machine, across every repo and worktree, each under its memory caps (6G of RAM
and 2G of swap, heavy-run's own defaults). `scripts/heavy.sh` is the one way in. Where heavy-run is not installed (CI,
another machine), or inside a job that already holds the lock (`HEAVY_RUN_ACTIVE`), a job runs as it always did.

- **The whole check and the shorter one** (`npm run check -- --all`, `--since`) take it by themselves. Until the lock is
  free the task shows `Waiting for its turn`. Waiting for GitHub does not hold it ([the check on GitHub](check-on-github.md)).
- **The workspace setup's build** (`scripts/setup.sh`) takes it for the build alone: the build cache's server is started
  before it, outside the job, and stays up.
- **A check of a few areas by name** (`npm run check board-panel`) does not take it.
- **A cargo build or test of your own** goes through it: `sh scripts/heavy.sh cargo build --profile quick`.

A job is a systemd user unit of its own. Stopped by the board or with Ctrl-C, it ends with its caller, and the lock is
held until it has. At the end it says what it used: `heavy: rc=0 memory.peak=812MiB swap.peak=0MiB` (the `Memory peak`
systemd-run prints after that line is not what the job used). A job the cap stopped ends with
`Finished with result: oom-kill`: that is not a failed test, so it is not run again unchanged. The lock's own record
of every job, and of every wait for it, is `~/.local/state/memwatch/jobs.log`.
