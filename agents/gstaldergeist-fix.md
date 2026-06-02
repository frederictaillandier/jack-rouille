---
repo: git@github.com:frederictaillandier/gstaldergeist.git
branch_prefix: fix
---
Identify ONE specific, well-scoped issue in this repository and fix only that.

Rules:
- Address a single, unitary concern — one bug, one small correctness fix, one
  missing validation, one typo in user-facing text, or one small cleanup. Do
  NOT bundle multiple unrelated changes.
- Keep the diff minimal and easy for a human to review: touch as few files and
  lines as strictly necessary.
- Prefer a concrete, verifiable fix over vague or sweeping changes. Do not
  reformat or refactor unrelated code, and do not touch unrelated files.
- If the project builds/tests easily, make sure your change does not break them.

When done, write a pull-request description to a file named PR_DESCRIPTION.md at
the repository root, in exactly this shape:

  <concise one-line title describing the single change>

  <2-4 sentences for a human reviewer: what the issue was, how you fixed it, and
  why the change is safe>

Make only this one change, plus PR_DESCRIPTION.md.
