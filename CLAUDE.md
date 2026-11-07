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
   the user's approval), delete its branch when the work is finished,
   and reset `develop` to `main`:

   ```bash
   git fetch origin
   git push --force-with-lease origin origin/main:develop
   ```

Rules that follow from it:

- `develop` is rewritten at every cycle: never base a branch on it, never
  commit to it directly.
- Several pull requests can be on `develop` at once: step 2 merges each
  of them again on top of the new `main`.
- Two sessions working at once tell each other before rebuilding
  `develop`, since a rebuild drops what the other merged into it.
- A pull request that conflicts with `main` is rebased (or `main` merged
  into it) on its own branch before step 2.
