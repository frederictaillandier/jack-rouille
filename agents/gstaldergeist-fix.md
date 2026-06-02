---
repo: git@github.com:frederictaillandier/gstaldergeist.git
branch_prefix: fix
---
Pick ONE well-scoped improvement to this repository and implement it on the current branch.

## Choosing what to work on
- First, look at what is already in flight so you don't redo existing work: run
  `gh pr list --state open` and inspect each one's intent (`gh pr view <n>` /
  `gh pr diff <n>`). Do NOT pick something an open PR already addresses — choose
  a genuinely different improvement.
- A good change is a single, coherent concern. It can be any of:
  - a correctness/logic bug, a missing validation, or a user-facing text error;
  - a refactor that improves separation of concerns, readability, or adherence
    to good practices (e.g. extracting a function, removing duplication,
    clarifying a module boundary);
  - added or strengthened tests for under-covered behaviour.
- Keep it cohesive and reviewable — one concern per PR, not a grab-bag of
  unrelated edits. Don't feel obliged to make the diff tiny: if the right
  improvement is a small refactor or a new test module, that's fine. Just avoid
  reformatting or rewriting unrelated code.
- If the project builds and tests easily, make sure your change keeps it green.

## Output
When done, write the pull-request description to `PR_DESCRIPTION.md` at the
repository root. The FIRST line must be a concise one-line title. Then leave a
blank line and write the body in clean, readable Markdown — use short sections
and bullets rather than a single paragraph, for example:

  Concise one-line title

  ## What
  A sentence or two on what the change does.

  ## Why
  The problem it solves or the motivation.

  ## Notes
  - files touched / scope
  - build & test status

Make this one coherent change, plus PR_DESCRIPTION.md.
