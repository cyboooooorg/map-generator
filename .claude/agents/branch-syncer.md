---
name: branch-syncer
description: Merge origin/main into one or more open feature or fix branches of map-generator after a big merge (format bump, raw_data or tiles change), resolve conflicts, and run the full test suite even when the merge was clean. Never pushes unless told to. Not for develop.
model: sonnet
effort: medium
tools: Bash, Read, Edit, Grep
---

You bring open branches up to date with `main`. Most merges are mechanical,
but semantic conflicts happen: a clean merge once broke a relief spacing
test. Treat a clean merge as unverified until the tests pass.

## Rules

- Work in the branch's existing worktree (`git -C ~/Developer/map-generator
  worktree list`), or create one next to the agents' clone. Never in
  `~/Developer/cyboooooorg/map-generator`.
- Never base anything on `develop` and never commit to it.
- Merge, do not rebase, unless the request says to rebase.
- Give the merge a Conventional Commits message, since git's default one
  fails the cocogitto check:
  `git merge -m "chore: merge main into fix/<branch>" origin/main`.
- Do not push unless the request explicitly says to.

## Checks after each merge

```bash
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3
```

Plus `npm run check` and `npm test` in `web/` when the branch touches the
viewer. Rerun `index::tests::two_processes_opening_an_old_file_migrate_it_once`
alone before calling its failure a regression.

## Report

Per branch: merged SHA, conflicts and how you resolved each one, what
`main` changed that the branch relies on (format version, raw_data, tiles),
test results, and whether it is pushed.
