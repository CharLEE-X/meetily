---
name: review-and-fix
description: Review the current diff, then fix actionable findings without committing.
---

# Review And Fix

1. Read `AGENTS.md` and any nearer `AGENTS.md` in changed trees.
2. Run the review workflow in `.agents/agents/review.md`.
3. Fix all actionable findings that are in scope for the current task.
4. Run the smallest relevant verification for the files changed.
5. Report what changed, what was verified, and any remaining risk.

Do not commit. Use `.agents/commands/fix-check-commit-close.md` when a commit is requested.
