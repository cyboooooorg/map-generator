# Agent instructions

## Branches and pull requests

`main` holds what has been tested. `develop` is a test branch: `main` plus
the pull requests being tried, rebuilt from `main` at every cycle.

1. Every feature or fix gets its own branch from `main` (`feat/…`,
   `fix/…`, `docs/…`) and its own pull request into `main`.
2. Before trying a pull request, rebuild `develop` on the current `main`,
   then merge the pull request's branch into it, keeping the branch:

   ```bash
   git fetch origin
   git checkout -B develop origin/main
   git merge --no-ff origin/<branch>   # once per pull request still under test
   git push --force-with-lease origin develop
   ```

3. Test on `develop` (the user or an agent): build and run it, look at
   the change in the app.
4. A change to make goes on the pull request's own branch, never on
   `develop`. Then start again from step 2.
5. Once it works on `develop`, merge the pull request into `main` (with
   the user's approval; GitHub then deletes its branch, see below) and
   reset `develop` to `main`:

   ```bash
   git fetch origin
   git push --force-with-lease origin origin/main:develop
   ```

Rules that follow from it:

- `develop` is rewritten at every cycle: never base a branch on it, never
  commit to it directly.
- Branches are merged into `develop` locally, with `git merge` and a push
  (step 2), never through GitHub: no pull request targets `develop`, and
  a pull request's base is never switched to it. The repository deletes
  a pull request's branch as soon as the pull request is merged
  ("Automatically delete head branches"), so a pull request merged into
  `develop` would delete a branch still under test.
- Several pull requests can be on `develop` at once: step 2 merges each
  of them again on top of the new `main`.
- Two sessions working at once tell each other before rebuilding
  `develop`, since a rebuild drops what the other merged into it.
- A pull request that conflicts with `main` is rebased (or `main` merged
  into it) on its own branch before step 2.
- Merge commits follow Conventional Commits too: git's default message
  (`Merge remote-tracking branch …`) fails the cocogitto check. Give each
  one a message such as `chore: merge main into fix/<branch>` or
  `chore: merge fix/<branch> (#123) into develop` (`git merge -m`), and
  merge pull requests into `main` with a conventional subject
  (`gh pr merge --merge --subject "feat(web): … (#123)"`).

## Backlog and issues

The backlog lives in the repository's GitHub issues (label `backlog`, plus
a theme label), not in `docs/backlog.md`, which keeps the decisions and
the history.

- A new backlog item, or a review remark left for later, becomes an issue
  (`gh issue create --label backlog`).
- A pull request that resolves an issue says so in its description with a
  closing keyword, one per issue (`Closes #123`, `Fixes #124`), so that
  GitHub closes the issue when the pull request is merged into `main`. A
  pull request that only advances an issue writes `Refs #123` instead.
  Commit footers may repeat them (`Closes: #123`).
- Comment on issues and pull requests whenever it helps the next reader:
  what was found, what was decided, why an issue stays open or is split,
  what a review left for later (with the follow-up issue's number).

## Subagents

`.claude/agents/` holds the subagents for the recurring tasks of this
repository, each with its model and effort fixed. Prefer them over generic
agents:

| Agent | Use it to | Model, effort |
|---|---|---|
| `codebase-mapper` | Find interfaces with `file:line` before a plan | sonnet, medium |
| `repo-state-auditor` | Check branches, worktrees and PRs (read-only) | haiku, low |
| `science-fixer` | Fix one science issue, test first, on a `fix/` branch | opus, high |
| `branch-syncer` | Merge `main` into open branches and run the tests | sonnet, medium |
| `branch-reviewer` | Review a whole branch before its PR (read-only) | opus, high |
| `develop-rebuilder` | Rebuild `develop` from `main` with the listed PRs | sonnet, low |
| `develop-deployer` | Deploy `develop` to planets.localhost and check it | sonnet, medium |
| `viewer-visual-check` | Screenshot and measure the viewer in a GPU browser | sonnet, medium |
| `generation-profiler` | Time and profile generation before and after | sonnet, medium |
| `world-coherence-checker` | Compare worlds of two binaries | sonnet, medium |
| `pr-shepherd` | Push, open the PR, watch CI, merge when approved | sonnet, low |
| `issue-writer` | File a backlog issue from a review remark | sonnet, medium |

The main session keeps what needs the conversation's context:

- design and spec discussions with the user;
- deciding to merge or push: `pr-shepherd` merges only when the request
  quotes the user's explicit approval for that pull request;
- warning the other sessions before `develop` is rebuilt, and agreeing on
  `FORMAT_VERSION` numbers and branch overlap;
- debugging driven by the user's screenshots: agents measure, the main
  session decides what to fix;
- judging whether a change that alters the generated worlds is acceptable.

## Context and token economy

Every turn resends the whole conversation, so a long context costs on every
turn that follows. A session of 992 turns reached 831k tokens of context
and reread 433M tokens from the cache.

### Images

- Never `Read` a full-resolution screenshot or render in the main session:
  an image stays in the context until the session ends.
  `viewer-visual-check` looks at the pixels and reports what it sees.
- When the main session must see an image itself, downscale a copy in the
  scratchpad first and read the copy:

  ```bash
  sips -Z 800 shot.png --out "$SCRATCH/shot-800.png"
  ```

### Large documents

`README.md` (about 180 KB), `docs/backlog.md`, `docs/system-spec.md` and
`docs/analysis/*.md` are too large to read whole. Find the section first
(`grep -n '^#' README.md` or `grep -n '<term>'`), then read only that range
with an offset and a limit. The same goes for subagents.

### Implementation plans and reviews

The agents of `.claude/agents/` replace the generic workflows. When a plan
is executed with subagents (superpowers `subagent-driven-development` or
otherwise):

- implementers run on sonnet; opus only for a task the plan marks as hard
  (physics, formats, concurrency) or for `science-fixer`;
- no review subagent after each task: the implementer runs the tests, and
  `branch-reviewer` reviews the whole branch once, before the PR;
- after a fix following that review, re-run the tests, not a new full
  review, unless the fix changes the design;
- recon goes to `codebase-mapper`, never to `Explore` or `general-purpose`.

### Session length

One session per topic: once a PR is open or a topic is settled, tell the
user it is a good point for `/clear`. Past about 200k tokens of context,
suggest a `/compact` that keeps the decisions, the branch and the open
questions.
