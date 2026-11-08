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
