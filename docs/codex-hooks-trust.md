# Codex's hooks dialog

Codex asks before it runs a hook it has not trusted yet, or one that changed since you trusted it:

```
  Hooks need review
  3 hooks are new or changed.
  Hooks can run outside the sandbox after you trust them.
› 1. Review hooks
  2. Trust all and continue
  3. Continue without trusting (hooks won't run)
```

A seat nobody watches (a reviewer, a worker just started) waits there until the review times out. A plugin update, or
Codex making its plugin cache again, is enough to bring it back for hooks you trusted before.

## What the board does

When a seat's Codex shows this dialog at its start (the reviewer's, or a worker's), before the seat has been given
its brief, and the dialog is the whole bottom of its screen, the board judges the hooks itself
and answers only when every one of them passes:

1. It asks Codex for the hooks of the seat's folder (`codex app-server`, `hooks/list`): each hook's event, command, file,
   where it comes from, and whether Codex trusts it. Codex's "Review hooks" screen is never opened.
2. The hooks Codex counts as new or changed must be as many as the screen says. Any other number: nothing is pressed.
3. Each of them must be yours:
   - **a plugin you allowed** (`board.trustHookPlugins`, as `id=git address`, Ponytail at first): enabled in Codex's
     config.toml, installed from that address, its checkout a clone of it with no change of its own and on its default
     branch, and every file the hook runs inside it and on disk, byte for byte (no filter or `--skip-worktree` of the
     checkout in between), as at that commit;
   - **or a file of a repo you allowed** (`board.trustHookRepos`, `owner/name` on GitHub, `GigiTiti-Kai/dotfiles` at
     first): the hooks file and every file the hook runs tracked there, and on disk the same as on the repo's default
     branch at origin. Nothing in the repo the seat works in is trusted by the board: that is the work under review, and
     its git could be made to say anything. Nor is a hook in a file git does not keep (`~/.codex/hooks.json`).
4. What each hook runs is read, and only plain shapes pass. Each part of the command (`a && b; c`) is a program by
   its bare name with nothing set before it (`PATH=. node …` does not pass), or a file. A shell takes `-c '…'` (read
   as a command again) or a script; `node` takes `-e '…'` or a script, with no other option before or after
   (`--require` does not pass); `python3`, `ruby` and the like take a script only. A name with a path (`./node`) is a
   file, and a name without one is the seat's folder's, as Codex runs the hook there. The files it names are read, and
   those they name in turn. Not trusted: a part known only when it runs (a `$` value other than the plugin's folder or
   `$HOME`, a command inside it), a program called by name (`make hook`, `rm`), a file that is not there, code given to
   `node` that starts programs, imports, or loads a file by a path (give `node` the file instead, so it is read), a module loaded by name that is not one of Node's own (it would be found
   where the hook runs), a network module loaded any way (`require(`node:http`)`, `import('https')`, Ruby's `net/http`), a removal with
   `recursive` in it, a file it loads that is not there to read as text (a native `.node`), `fetch`, `curl`,
   secrets (`~/.ssh`, `auth.json`, tokens, `history.jsonl`), deleting a tree or history, or hidden code (`eval`,
   base64 into a shell).
5. All passed: the board types the number of "Trust all and continue", checks that line is the chosen one, and presses
   enter; then checks the dialog is gone. Codex writes the trust in its own config.toml, as when you press it.

Anything else, and the board presses nothing: the seat waits for you (needs you). A reviewer's row says why
(`The reviewer's Codex asks to trust its hooks, and the board did not: …`), and its review waits however long you take:
`review.timeout` counts again only once you answered. A worker's row says it needs you.

Each judgement, trusted or not, is one line of `hooks-trust.log` in the board's data folder: the seat, its pane and
folder, and for each hook its event, where it comes from and what was read, or the reason it was not trusted.

The board never turns the check off and never starts Codex with `--dangerously-bypass-hook-trust`.

## Its limits

The reading in step 4 looks for plain words, not for what the code does: it stops the plain ways of doing those things,
not a hidden one, and not what a program the hook calls by name does. It can also stop a hook that only names such a
thing (a guard script that lists `git push --force` to block it): that one waits for you. The trust rests on where the
hook comes from first; add to the two lists only what you would trust without reading. What Codex's own environment
hands the hook (`NODE_OPTIONS`, a `PATH` that holds the seat's folder) is not the hook's, and not judged. Which commit is origin's default
branch is read from the local copy of origin's branches, which a program on this machine could move; asking GitHub
itself would close that.

## Claude Code

The same way of judging fits Claude Code's two questions: its folder dialog (the seat's folder in a repo you allowed, or
a worktree of one) and a question about hooks (the same two lists, and the same reading of `~/.claude/settings.json` and
the repo's `.claude/settings.json`). Nothing of it is built for Claude yet. Before a seat with full access starts, the
board already marks its repo as trusted in `~/.claude.json`, so the folder dialog does not show for it; for a seat with
less access it stays yours. A Claude seat stopped on a question about hooks has not been seen.
