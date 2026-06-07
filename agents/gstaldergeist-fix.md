---
repo: git@github.com:frederictaillandier/gstaldergeist.git
branch_prefix: fix
---
Do ONE useful thing for this repository this run. Work through the priorities
below **in order** and stop at the first one that has something for you to do.
Only fall through to the next priority when the current one needs nothing.

## How your output becomes a PR (important)
The harness wraps this run: after you finish, if the working tree has
uncommitted changes it commits them to a fresh branch and opens a pull request
using `PR_DESCRIPTION.md`; if the tree is clean it does nothing further. You
also have `Bash`, so you can run `gh` and `git` yourself. That gives two modes:
- **Let the harness open the PR** — make your edits and leave them uncommitted
  in the working tree, and write `PR_DESCRIPTION.md` (see Output). Use this for
  a brand-new code change (priorities 2-fix and 3).
- **Act yourself via `gh`/`git`** — for commenting or pushing to an existing
  PR branch. When you do this, finish with a **clean working tree** and do
  **not** write `PR_DESCRIPTION.md`, so the harness doesn't open a spurious PR.

## Priority 1 — Respond to feedback on open PRs
Run `gh pr list --state open` and inspect each PR you authored, including its
review threads and comments (`gh pr view <n> --comments`, `gh pr diff <n>`).
If any has an unanswered comment or question directed at you:
- **A question / discussion:** reply with `gh pr comment <n> --body "..."`.
- **A requested code change:** check out the PR branch (`gh pr checkout <n>`),
  make the change, commit and push it to that same branch, then leave a comment
  summarising what you did. Ensure the working tree is clean afterwards.
Handle the oldest unaddressed PR first. This is your task for the run — do not
also start something new. (Skip threads already resolved or that don't need you.)

## Priority 2 — Make progress on an open issue
If no open PR needs a response, run `gh issue list --state open` and pick one
issue to move forward (prefer the oldest or any explicitly prioritised):
- **If you can implement it:** make the fix as a normal code change (leave edits
  uncommitted + write `PR_DESCRIPTION.md`) so the harness opens a PR. Reference
  the issue in the description (e.g. "Closes #N").
- **If it's unclear or needs a decision before you can act:** ask for the
  missing information with `gh issue comment <n> --body "..."`, leave the tree
  clean, and stop. That comment is your task for this run.

## Priority 3 — Audit the codebase (default)
If nothing above needs you, fall back to the normal audit: pick ONE well-scoped
improvement and implement it on the current branch.
- First, look at what's already in flight so you don't redo existing work
  (`gh pr list --state open`, `gh pr view <n>` / `gh pr diff <n>`). Choose a
  genuinely different improvement — don't duplicate an open PR.
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
This applies whenever you want the harness to open a PR (priorities 2-fix and 3).
Write the pull-request description to `PR_DESCRIPTION.md` at the repository root.
The FIRST line must be a concise one-line title. Then leave a blank line and
write the body in clean, readable Markdown — use short sections and bullets
rather than a single paragraph, for example:

  Concise one-line title

  ## What
  A sentence or two on what the change does.

  ## Why
  The problem it solves or the motivation.

  ## Notes
  - files touched / scope
  - build & test status

Make this one coherent change, plus PR_DESCRIPTION.md. (If instead you handled
priority 1, or asked for information in priority 2, do not write
PR_DESCRIPTION.md and leave the working tree clean.)
