#!/bin/sh
# The board's whole check, run on GitHub Actions instead of on this machine (docs/check-on-github.md): it pushes the
# task's branch, waits for the whole run of `.github/workflows/ci.yml` on that commit (Linux and macOS) and passes only
# on that run's success, for the commit still checked out. A failed step comes back with its job, step, the end of its
# log and the run's address; a run GitHub gave no answer on (it never started, was cancelled or skipped, gh could not
# reach it, the time ran out) ends on one fixed line, and the board holds the task for you instead of sending it back.
# Run by anyone but the board, or with CHECK_LOCAL=1, it is the whole check here: `scripts/check.sh --all`.
set -u
here=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$here" || exit 2
# Only the board hands a check its progress file (docs/check-progress.md): an agent's own run never pushes.
if [ "${CHECK_LOCAL:-}" = 1 ] || [ -z "${HERDR_PROJECTS_PROGRESS:-}" ]; then exec sh scripts/check.sh --all; fi
# The line the board knows (check_progress::CI_DOWN_LINE): not the code's fault, and nothing was judged.
down="The check could not run on GitHub"
say() { { printf '%s\n' "$1" > "$HERDR_PROJECTS_PROGRESS"; } 2>/dev/null || true; }
quit() { echo "$1"; exit 2; }
url=""; id=""
infra() {
  echo "Nothing was judged: this is not a failure of the code. Try it again with herdr-projects retry <task>; a run that ended stays the answer for this commit, so run it again on GitHub first${id:+ (gh run rerun $id -R $repo)}. Or run the whole check on this machine: herdr-projects set check.command \"CHECK_LOCAL=1 sh scripts/check-cloud.sh\" --repo <repo>, then retry."
  echo "$down: $1${url:+ · $url}"
  exit 3
}
command -v gh >/dev/null 2>&1 || infra "the GitHub CLI (gh) is not installed"
auth=$(gh auth status 2>&1) || infra "gh cannot reach GitHub as you: $(printf '%s' "$auth" | grep -v '^ *$' | head -n 2 | tr '\n' ' ')"
# The repo as origin names it: written there, before git's own url rewrites. Never said with a login in it.
origin=$(git config --get remote.origin.url | sed -e 's#^https://[^/@]*@#https://#')
repo=$(printf '%s' "$origin" | sed -e 's#^https://github\.com/##' -e 's#^ssh://git@github\.com/##' -e 's#^git@github\.com:##' -e 's#\.git$##')
case "$repo" in */*/*|*:*|"") infra "origin is not a GitHub repo ($origin)" ;; */*) ;; *) infra "origin is not a GitHub repo ($repo)" ;; esac
branch=$(git symbolic-ref -q --short HEAD) || quit "The check on GitHub checks a branch: HEAD is not on one."
# The run checks the commit, not the folder: what is not committed would not be checked.
[ -z "$(git status --porcelain)" ] || quit "The check on GitHub checks the commit, and this folder has changes that are not in it: commit them first.
$(git status --short | head -n 20)"
sha=$(git rev-parse HEAD)
# It ends by itself a minute before the board would stop it (check.timeout, handed in by the board), with its own word.
limit=${HERDR_PROJECTS_CHECK_TIMEOUT:-900}
end=$(( $(date +%s) + (limit > 120 ? limit - 60 : limit / 2) ))
poll=${CHECK_CLOUD_POLL:-15}
late() { [ "$(date +%s)" -ge "$end" ]; }
# The branch is the board's own: what was pushed before is overwritten only while origin still holds what was seen here.
say '{"state":"running","ci":"pushing the branch"}'
pushed=$(git push -q --force-with-lease origin "HEAD:refs/heads/$branch" 2>&1) || infra "the branch could not be pushed: $(printf '%s' "$pushed" | tail -n 3 | tr '\n' ' ')"
# The run of this commit's push, fixed from here on: its id, its attempt and its workflow.
say '{"state":"running","ci":"waiting for GitHub to start the run"}'
while :; do
  run=$(gh run list -R "$repo" --workflow ci.yml --commit "$sha" --branch "$branch" --event push --limit 1 --json databaseId,attempt,workflowName --jq '.[0] // empty | "\(.databaseId)|\(.attempt)|\(.workflowName)"' 2>&1) && [ -n "$run" ] && break
  late && infra "no run of ci.yml started on GitHub for $sha on $branch${run:+ (gh: $(printf '%s' "$run" | tail -n 1))}"
  sleep "$poll"
done
IFS='|' read -r id attempt workflow <<EOF
$run
EOF
url="https://github.com/$repo/actions/runs/$id"
echo "Checking on GitHub: $url"
say "{\"state\":\"running\",\"ci\":\"GitHub run $id\"}"
while :; do
  seen=$(gh run view "$id" -R "$repo" --json status,conclusion,headSha,attempt,event,workflowName,url --jq '[.status, .conclusion, .headSha, (.attempt | tostring), .event, .workflowName, .url] | join("|")' 2>&1) && case "$seen" in completed\|*) break ;; esac
  late && infra "the run did not finish within the check's time (check.timeout)${seen:+, last seen: $(printf '%s' "$seen" | tail -n 1)}"
  sleep "$poll"
done
IFS='|' read -r _status conclusion head now_attempt event now_workflow seen_url <<EOF
$seen
EOF
[ -n "$seen_url" ] && url=$seen_url
# Only a success of that very run counts, for the commit it started from, still the one checked out here.
[ "$head" = "$sha" ] && [ "$now_attempt" = "$attempt" ] && [ "$event" = push ] && [ "$now_workflow" = "$workflow" ] || infra "the run is no longer the one started for $sha (commit $head, attempt $now_attempt of $attempt, $event, $now_workflow)"
[ "$(git rev-parse HEAD)" = "$sha" ] || infra "the branch moved on while it ran: it checked $sha, and $(git rev-parse HEAD) is checked out now"
if [ "$conclusion" = success ]; then echo "The check passed on GitHub: $url"; exit 0; fi
# A step that failed is the code's: its job and step, and the end of its log (each line without its job, step and time).
steps=$(gh run view "$id" -R "$repo" --json jobs --jq '.jobs[] | .name as $job | .steps[]? | select(.conclusion == "failure") | "\($job) / \(.name)"' 2>/dev/null)
if [ "$conclusion" = failure ] && [ -n "$steps" ]; then
  echo "Failed on GitHub (job / step):"
  printf '%s\n' "$steps" | sed 's/^/  /'
  echo
  echo "The end of its log:"
  gh run view "$id" -R "$repo" --log-failed 2>&1 | tail -n 60 | cut -f3- | sed 's/^[^ ]*[0-9]Z //'
  echo
  echo "The check failed on GitHub: $(printf '%s\n' "$steps" | head -n 1) · $url"
  exit 1
fi
jobs=$(gh run view "$id" -R "$repo" --json jobs --jq '[.jobs[] | "\(.name) \(.conclusion)"] | join(", ")' 2>/dev/null)
infra "the run ended ${conclusion:-with no conclusion}${jobs:+ ($jobs)}, with no step of its own failed"
