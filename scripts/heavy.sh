#!/bin/sh
# Runs one heavy job (the whole check, the workspace setup's build, a cargo build or test of your own) under the
# machine's shared lock `heavy-run` where it is installed: one heavy job at a time across every repo and worktree, under
# its memory caps (docs/heavy-jobs.md). Without heavy-run, or already inside one (HEAVY_RUN_ACTIVE), the job runs
# directly: no second lock or cap. `sh scripts/heavy.sh cargo build --profile quick`.
#
# heavy-run starts the job as a systemd user unit, in the home folder and with none of this environment, and
# systemd-run rewrites `$` in its arguments: the environment, folder and arguments travel on the unit's input (never on
# disk or a command line, where they would show), and the job itself gets /dev/null. The unit outlives a killed
# caller, so a watcher in it ends the unit when this script goes away. A job the cap stopped ends with systemd-run's
# `Finished with result: oom-kill`: not a failed test, never run again unchanged.
set -u
hr=$(command -v heavy-run 2>/dev/null || true)
[ -n "$hr" ] || [ ! -x "$HOME/.local/bin/heavy-run" ] || hr=$HOME/.local/bin/heavy-run
if [ -n "${HEAVY_RUN_ACTIVE:-}" ] || [ -z "$hr" ]; then exec "$@"; fi
# The rest is bash's (printf %q, a wait that a Ctrl-C ends): heavy-run is a bash program, so where it is, bash is.
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"

# shellcheck disable=SC2016,SC3050,SC3001 # $(cat) and the job's own lines are the unit's bash's; this is bash already
{
  export -p
  # A caller's empty HEAVY_RUN_ACTIVE would hide the unit's own: a nested job would then wait for this job's lock.
  echo 'export HEAVY_RUN_ACTIVE=1'
  printf 'cd -- %q || exit 2\n' "$PWD"
  printf 'caller=%q\n' "$$"
  printf 'set --'; printf ' %q' "$@"; printf '\n'
  cat <<'EOF'
# A caller stopped while it waited for the lock has nobody left to report to.
kill -0 "$caller" 2>/dev/null || exit 143
unit=/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup 2>/dev/null | head -n 1)
# Only a unit of heavy-run's own is ever ended, never the group of whatever else ran this (a stand-in in the tests).
case "$unit" in */heavy-run-*.service) ;; *) unit="" ;; esac
( while kill -0 "$caller" 2>/dev/null; do sleep 1; done; [ -z "$unit" ] || echo 1 > "$unit/cgroup.kill" ) &
watcher=$!
"$@" < /dev/null
rc=$?
kill "$watcher" 2>/dev/null
# Read here, while the unit lives: the `Memory peak` systemd-run prints after it is a few hundred K whatever it used.
mib() { awk '{ printf "%d", $1 / 1048576 }' "$unit/$1" 2>/dev/null; }
[ -z "$unit" ] || echo "heavy: rc=$rc memory.peak=$(mib memory.peak)MiB swap.peak=$(mib memory.swap.peak)MiB" >&2
exit "$rc"
EOF
} | setsid -w "$hr" -- bash -c 'eval "$(cat)"' "$@" > >(cat) 2> >(cat >&2) & # the trailing arguments only name the job in jobs.log
# heavy-run holds the lock. In a session of its own, a stop of the caller's process group (the board's, or Ctrl-C)
# does not reach it: it keeps the lock until the watcher has ended the unit. `wait` lets this script stop at once.
# Its output goes through two `cat`s of the caller's group, never straight to the caller's own: a heavy-run still
# waiting for the lock when the caller was stopped would hold that open (the board reads it to its end) until its turn.
trap 'exit 130' INT
wait "$!"
