# Karpathy Behavioral Guidelines

Merge these with `AGENTS.md` and the nearest local code pattern.

## Think Before Coding

- State assumptions when they matter.
- If multiple interpretations are plausible, surface the tradeoff.
- Ask only when a reasonable assumption would be risky.

## Simplicity First

- Implement the minimum code that solves the request.
- Avoid speculative configuration, abstraction, or error paths.
- If an edit grows large, re-check whether the same outcome can be reached more directly.

## Surgical Changes

- Touch only files needed for the task.
- Match existing style even when another style is also valid.
- Remove dead imports or code introduced by your own changes.
- Mention unrelated cleanup opportunities instead of doing them.

## Goal-Driven Execution

- Define how the change will be verified before calling it done.
- Use targeted checks first, then broader checks when the blast radius requires them.
- Do not claim completion until the relevant verification has run or a concrete blocker is reported.
