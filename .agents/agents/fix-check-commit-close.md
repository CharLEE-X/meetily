---
name: fix-check-commit-close
description: Finish implementation work by reviewing, fixing, checking, and committing scoped changes.
---

# Fix Check Commit Close

1. Read `AGENTS.md` and any nearer `AGENTS.md` in changed trees.
2. Review the current diff using `.agents/agents/review.md` unless review findings were already supplied.
3. Fix all actionable in-scope findings.
4. Run relevant checks from `AGENTS.md` and nearer routing files.
5. Inspect `git status --short` and ensure unrelated user changes are not included.
6. Commit only the intended files with a concise conventional commit message if the user requested a commit.
7. Report the commit hash when committed, checks run, and any skipped verification.
