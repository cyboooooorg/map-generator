# Agent instructions

## Architecture

The project is being rewritten from a clean slate (design:
[architecture rewrite](docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
roadmap #427). The last legacy commit is the tag `legacy-final`: read the
legacy code there (`git show legacy-final:<path>`,
`git grep <term> legacy-final`), never from memory.

Crates, and the direction of their dependencies (checked by
`tools/guardrails`):

```text
cli ──> adapters ──> catalogue, system, world, timeline, science
catalogue ──> system (the inputs type), timeline
system ──> timeline, science
world  ──> timeline, science
timeline ──> science (units only)
```

Where a change goes:

| Change | Crate |
| --- | --- |
| A physical law, a unit, a calibrated constant | `science` |
| How a body or system changes over time, an event | `system` |
| What the engines exchange | `timeline` |
| How a state is laid out on the surface | `world` |
| What is stored, cached, evicted | `catalogue` |
| A file format, an export, a text catalogue | `adapters` |
| Wiring, command-line options | `cli` |

Rules (the anti-goals of the design):

- The map generator (`world`) never feeds anything back into the system
  engine, and never reads the kind of the central object: it reads
  physical forcings from the timeline.
- One code path per value: the computation that derives a value produces
  its cause node; nothing recomputes a value to explain it.
- The domain emits codes, parameters and cause trees, never human text;
  translation lives in `adapters`.
- Only the inputs are stored truth; timelines, worlds and exports are
  caches, invalidated by the engine version, never migrated.
- Physics is ported from `legacy-final` with its source and its tests,
  never rewritten from memory.
- No random draw outside the seed hierarchy, no file, network or process
  in the domain crates: clippy denies them (`clippy.toml`).

## No invented data

Values have been made up in the past where the science was missing. For
Claude and every agent:

1. **Missing or thin science stops the work.** When a law, a constant or
   a range is needed and `docs/science/` does not cover it, or covers it
   too shallowly for the case, do the study first: search the
   literature, then create or extend the reference page (with its
   sources) before writing the code. A larger question gets a study in
   the [Wiki](https://github.com/cyboooooorg/map-generator/wiki), and the
   page cites it.
2. **Calibrate only when no data exists.** A calibrated value has a doc
   comment starting with `Calibration:` (what it was fitted against, how,
   a link to the page), a calibration section in its page saying what was
   searched and not found, and a line under its own heading in the pull
   request description.
3. **Every number in `science` is sourced, calibrated or a definition.**
   Its doc comment starts with `Source:` (the citation), `Calibration:`
   or `Definition:` (an exact definition or a unit conversion). A tier-1
   test fails otherwise.
4. `branch-reviewer` flags any new number that is neither sourced nor
   calibrated, in the engines too.

## Tests

Three tiers with budgets (the user's to change), split by
`.config/nextest.toml`:

| Tier | What | Where | Budget |
| --- | --- | --- | --- |
| 1. Unit | Laws, pure functions against published values, guardrails | Every test outside `crates/regression` | 2 min |
| 2. Regression | Shared reference systems, contracts, determinism | `crates/regression` | 8 min |
| 3. Heavy | 15 km worlds, oracle, censuses, profiles | `#[ignore = "heavy: …"]` | None |

- `mise run ci` runs what the pipeline runs (format, clippy, tiers 1 and
  2 with their budgets, markdownlint): run it before every pull request.
  `mise run web:check` covers the front-end; never run it in a worktree
  (no `npm ci` there on this machine).
- No test generates a system or a world of its own outside tier 3:
  tier-2 tests assert on the shared reference systems.
- Expected values come from sources, not from the code. A test that pins
  the code's current output is allowed only in tier 2, as a regression
  net.
- Over a budget, arbitrate: move a test to tier 3, merge it with another
  or delete it. `test-reviewer` proposes; the user decides when a budget
  is at stake.
- A pull request that changes what the engines produce pastes the output
  of `mise run test:heavy` (oracle report, censuses) in its description.

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

3. Test on `develop`. During the engine phase (no server), build
   `develop` and run the CLI on the oracle's seeds; deliver the oracle
   report and downscaled 2:1 projections of a few bodies, which the user
   looks at before approving the merge.
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
a theme label). `docs/backlog.md` keeps the legacy decisions and history.

- A new backlog item, or a review remark left for later, becomes an issue
  (`gh issue create --label backlog`).
- Place every new issue in the roadmap when filing it, so none is
  forgotten or goes stale outside the plan. The roadmap is #427; each
  track (T0 to T10) is a tracking issue whose sub-issues are its steps.
  - An issue that belongs to a track becomes a sub-issue of that track's
    tracking issue, and goes into its ordered list (`## Steps, in order`):

    ```bash
    id=$(gh api repos/{owner}/{repo}/issues/<n> --jq .id)
    gh api repos/{owner}/{repo}/issues/<track>/sub_issues -F sub_issue_id="$id"
    ```

  - An issue that waits on a track without belonging to it goes under
    `## Issues waiting on a track` in #427, with what it waits for. A
    large idea not scheduled yet goes under `## Later, not scheduled`.
  - A small, independent issue stays out of the roadmap: its theme label
    is enough, and it can be picked any time.
  - A new track gets its tracking issue, a row in #427's `## Tracks`
    table, a place in `## Order`, and becomes a sub-issue of #427.
  - An issue closed as not planned or as a duplicate leaves its track's
    list; one moved to another track moves in both places.
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
| --- | --- | --- |
| `codebase-mapper` | Find interfaces with `file:line` before a plan | sonnet, medium |
| `repo-state-auditor` | Check branches, worktrees and PRs (read-only) | haiku, low |
| `science-fixer` | Port or fix one law in `science`, test first, on a `fix/` branch | opus, high |
| `branch-syncer` | Merge `main` into open branches and run `mise run ci` | sonnet, medium |
| `branch-reviewer` | Review a whole branch before its PR (read-only) | opus, high |
| `test-reviewer` | Check a branch's tests: tiers, duplicates, sources, budgets | sonnet, medium |
| `develop-rebuilder` | Rebuild `develop` from `main` with the listed PRs | sonnet, low |
| `generation-profiler` | Time and profile generation before and after | sonnet, medium |
| `world-coherence-checker` | The oracle: legacy against new, by distributions | sonnet, medium |
| `pr-shepherd` | Push, open the PR, watch CI, merge when approved | sonnet, low |
| `issue-writer` | File a backlog issue from a review remark | sonnet, medium |
| `develop-deployer` | Paused during the engine phase (no server) | sonnet, medium |
| `viewer-visual-check` | Paused during the engine phase (no viewer) | sonnet, medium |

The main session keeps what needs the conversation's context:

- design and spec discussions with the user;
- deciding to merge or push: `pr-shepherd` merges only when the request
  quotes the user's explicit approval for that pull request;
- warning the other sessions before `develop` is rebuilt, and agreeing on
  changes to the `timeline` contract and branch overlap;
- judging whether a change that alters the generated worlds is acceptable.

## Context and token economy

Every turn resends the whole conversation, so a long context costs on every
turn that follows. A session of 992 turns reached 831k tokens of context
and reread 433M tokens from the cache.

### Images

- Never `Read` a full-resolution render in the main session: an image
  stays in the context until the session ends.
- When the main session must see an image itself, downscale a copy in the
  scratchpad first and read the copy:

  ```bash
  sips -Z 800 shot.png --out "$SCRATCH/shot-800.png"
  ```

### Large documents

`docs/backlog.md`, `docs/system-spec.md`, the design documents and many
`docs/science/*.md` pages are too large to read whole. Find the section
first (`grep -n '^#' <file>` or `grep -n '<term>'`), then read only that
range with an offset and a limit. The same goes for subagents. Studies
live in the [Wiki](https://github.com/cyboooooorg/map-generator/wiki).

### Implementation plans and reviews

The agents of `.claude/agents/` replace the generic workflows. When a plan
is executed with subagents (superpowers `subagent-driven-development` or
otherwise):

- implementers run on sonnet; opus only for a task the plan marks as hard
  (physics, formats, concurrency) or for `science-fixer`;
- no review subagent after each task: the implementer runs `mise run ci`,
  and `branch-reviewer` and `test-reviewer` review the whole branch once,
  before the PR;
- after a fix following that review, re-run `mise run ci`, not a new full
  review, unless the fix changes the design;
- recon goes to `codebase-mapper`, never to `Explore` or `general-purpose`.

### Session length

One session per topic: once a PR is open or a topic is settled, tell the
user it is a good point for `/clear`. Past about 200k tokens of context,
suggest a `/compact` that keeps the decisions, the branch and the open
questions.
