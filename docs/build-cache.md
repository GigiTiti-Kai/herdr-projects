# The build cache

**Every Rust task on the board shares one compile cache, and writes less into its own `target/`.** A worktree's build
output is written, deleted when the task ends, and written again by the next task, and on a virtual disk (WSL's
`ext4.vhdx`) what is deleted does not give the space back. So a task should write as little as it can.

Where the cache is installed, a task in any repo with a `Cargo.toml` is given, for its agent, reviewers, setup and check:

| Given | What it does |
| --- | --- |
| `RUSTC_WRAPPER` and `SCCACHE_*` | libraries already compiled once on this machine are taken from the cache |
| `CARGO_INCREMENTAL=0` | no incremental notes in `target/`: the largest saving, and a repo's own crates can be cached too |
| `CARGO_PROFILE_DEV_DEBUG=line-tables-only` | smaller test programs; backtraces keep their lines, a debugger sees no variables |

The two `CARGO_` lines are left out for a repo whose `Cargo.toml` has `[profile.…]` of its own: it chose how it builds.
This repo is one: its `quick` builds lean on incremental notes ([testing](testing.md)).

## Where it is, how big, and how to empty it

- **Where:** `~/.cache/herdr-projects/sccache` (or under `XDG_CACHE_HOME`), outside every worktree, with sccache itself
  beside it (`sccache-0.18.0/`, pinned and checked against its sha256) and its socket (`sccache.sock`).
- **Limit:** 10 GB, oldest dropped first. A build with `SCCACHE_CACHE_SIZE=100M` stayed at 94 MB. herdr-agent-quota and
  herdr, built for tests and for release, took about 550 MB. `SCCACHE_CACHE_SIZE` set for the board wins; a new
  limit counts once the server starts again.
- **Is it hit?** In this repo: `sh scripts/build-cache.sh --stats`.
- **Turn it off:** `HERDR_PROJECTS_NO_CACHE=1` for the board gives a task none of the above. A `RUSTC_WRAPPER` of your own
  wins. To get one setting back in a task's shell: `export CARGO_INCREMENTAL=1`.
- **Empty it** (the next builds fill it again; nothing else depends on it):

```sh
SCCACHE_SERVER_UDS=~/.cache/herdr-projects/sccache.sock ~/.cache/herdr-projects/sccache-0.18.0/sccache --stop-server
rm -rf ~/.cache/herdr-projects/sccache
```

The sweep never touches it: only the limit keeps it small.

## Its server

One server for all repos, which stays up: a task's first build can come hours after the board started it. The first
task of a machine in a Rust repo installs sccache as it starts (a download of up to two minutes, once); after that a
start only asks whether the server answers. It is started one at a time on purpose: twelve compiles that each found no
server started about nine servers between them, every round of 25.

## What it saves

Bytes written by the whole build (`/proc/<pid>/io`: `write_bytes` less `cancelled_write_bytes`, plus the cache's own
server), for `cargo test --no-run` and `cargo build --release` in a new worktree, with the cache already filled:

| Build | Before | Cache alone | Cache and smaller build |
| --- | --- | --- | --- |
| herdr-agent-quota | 2.3 GB, 162 s | 2.2 GB, 71 s | 1.2 GB, 60 s |
| herdr | 4.8 GB, 381 s | 4.8 GB, 360 s | 2.1 GB, 344 s |

**The cache alone saves time, not disk:** a hit is still written into `target/`. The smaller build is what writes less.
Its price is the rebuild after an edit: herdr's `cargo test --no-run` took 86 s against 21 s, writing 0.3 GB against
1.5 GB each time.

In this repo a new worktree starts from a kept build ([testing](testing.md)), hard-linked where the file system has no
clones. A task (setup, one edit, the program and the test program built) wrote 1.5 GB with a plain copy and 0.9 GB with
links; a kept build stayed byte for byte the same after worktrees rebuilt the crates linked from it, as rustc replaces a
file and never writes into one.

## Two builds at once

Two builds of herdr-agent-quota at the same moment both passed, with the cache filled (43 s each, 27 s alone) and empty
(74 s each). Neither waits for the other but for cargo's `package cache` lock for a moment as it starts, which two
builds with no cache did as well. A shared `CARGO_TARGET_DIR` was left out: there two builds take turns.

A `CARGO_TARGET_DIR` that differs from build to build means no hits at all: it is one of the `CARGO_` variables the cache
keys each compile on. A build that clears the environment (`env -i`) gets none of this either.
