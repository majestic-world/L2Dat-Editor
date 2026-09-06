# Issue tracker: Local Markdown

Issues and specs for this repo live as markdown files in `.scratch/`.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`.
- The spec is `.scratch/<feature-slug>/spec.md`.
- Implementation issues are one file per ticket at
  `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01`,
  never a single combined tickets file.
- Triage state is recorded as a `Status:` line near the top of each
  issue file. Use the role strings in `docs/agents/triage-labels.md`.
- Append comments and conversation history at the bottom of the file
  under a `## Comments` heading.

## When a skill says "publish to the issue tracker"

Create the spec or individual issue files at the paths above,
creating directories as needed.

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. Resolve a bare issue number
within the relevant feature directory; if ambiguous, ask which feature.

## Wayfinding operations

Used by `/wayfinder`. The map is a file with one child file per ticket.

- Map: `.scratch/<effort>/map.md`, containing Notes, Decisions-so-far,
  and Fog.
- Child ticket: `.scratch/<effort>/issues/NN-<slug>.md`, numbered
  from `01`, with the question in the body. A `Type:` line records
  `research`, `prototype`, `grilling`, or `task`.
- Wayfinding lifecycle: `Status: claimed` or `Status: resolved`.
  These lifecycle states are separate from the triage role vocabulary.
- Blocking: a `Blocked by: NN, NN` line near the top. A ticket is
  unblocked when every file it lists is resolved.
- Frontier: scan the effort's issues for open, unblocked, unclaimed
  tickets; first by number wins.
- Claim: set `Status: claimed` and save before any work.
- Resolve: append the answer under `## Answer`, set `Status: resolved`,
  then append a context pointer (gist + link) to Decisions-so-far
  in `map.md`.
