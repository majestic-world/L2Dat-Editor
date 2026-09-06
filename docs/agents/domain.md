# Domain Docs

Layout: single-context.

## Before exploring, read these

- `CONTEXT.md` at the repo root.
- ADRs in `docs/adr/` that touch the area being explored.

If these files don't exist, proceed silently. Don't flag their absence
or suggest creating them upfront. `/domain-modeling`, also reached via
`/grill-with-docs` and `/improve-codebase-architecture`, creates them
lazily when terms or decisions actually get resolved.

## File structure

- `CONTEXT.md`: shared domain vocabulary.
- `docs/adr/`: architectural decision records.

## Use the glossary's vocabulary

When naming a domain concept in an issue, proposal, hypothesis, or test,
use the term defined in `CONTEXT.md`. Avoid synonyms the glossary
explicitly rejects.

If a concept is missing, reconsider whether it belongs to the project's
language; note real gaps for `/domain-modeling`.

## Flag ADR conflicts

If your output contradicts an existing ADR, surface the conflict
explicitly, identify the ADR, and explain why reopening it is warranted.
