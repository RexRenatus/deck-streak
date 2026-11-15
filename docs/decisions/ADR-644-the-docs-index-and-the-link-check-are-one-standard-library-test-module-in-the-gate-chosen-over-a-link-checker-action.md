# ADR-644: the docs index and the link check are one standard-library test module in the gate, chosen over a link-checker action

- **Status:** accepted with SPEC-622.
- **Decides:** SPEC-622.

## Context

The front door described a surface ADR-589 deprecated and named neither the iPhone and iPad app
nor the merge queue ADR-625 put in front of `dev`. The docs index was kept by hand and, read at dev
`8620664`, linked 15 of the 29 living docs it should list and 3 of the 7 record folders. Nothing in
the gate resolved a relative link, so a renamed file or heading broke a link silently. The gate
already runs every `scripts/tests/test_*.py` module through `python3 -m unittest discover`
(`scripts/check.sh` lines 185 to 186), and no module under `scripts/` imports a YAML library.

## Decision

- **D1. One rule says what a living doc is.** A living doc is a tracked `*.md` file outside the
  record folders (`docs/specs/`, `docs/decisions/`, `docs/schematics/`, `docs/red-first/`,
  `docs/rulings/`, `changelog.d/`) and `agent/`, and outside any `tests/` or `fixtures/` folder.
  The list comes from `git ls-files`, names only, so an untracked file never joins it.
- **D2. The index and the link check are one module.** `scripts/tests/test_front_door_docs.py`
  holds both, with the standard library alone, and runs in the Python stage with no workflow
  change. The index check reads `docs/README.md`'s links and refuses a living doc or record folder
  it does not link. The link check reads every living doc's inline links, images, reference
  definitions and HTML `href` and `src` values outside fences and code spans, resolves each relative
  destination against the tracked names, and resolves a fragment against the target's heading
  anchors by GitHub's documented rule. A destination with a scheme is neither fetched nor checked.
- **D3. The queue statements are tested against the queue file.** A small reader for
  `.mergify.yml`'s block form reads the label, the required checks, the batch size and the merge
  method; the branch model in `CONTRIBUTING.md` and the merge queue lines in `README.md` and
  `AGENTS.md` are refused unless they say the same.
- **D4. The surfaces and the deprecation are tested on the owned text.** The front door names the
  web app and the iPhone and iPad app with their folders, and any block of the owned text that names
  the Mini App, the bot or Telegram says deprecated or cites ADR-589.

## What it was chosen AGAINST

| alternative | why it lost |
|---|---|
| a link-checker action or binary in a workflow | it adds a third-party dependency and a workflow change, and the common ones fetch external links, so a remote site's outage turns the gate red |
| a hand-kept index with no test | that is what the tree had, and it drifted to 14 living docs and 4 record folders missing with nothing to say so |
| checking external links too | it puts the network in the gate: a run's verdict would depend on sites outside the repository, and the gate is meant to be the same offline as in CI |

## Consequences

- Good, because a pull request that renames a file or a heading, or adds a living doc, reads red
  until its links and the index say the same, with the path named in the failure.
- Good, because the merge queue text follows `.mergify.yml`: a changed label, check, batch size or
  merge method reds the pages until they are rewritten.
- Bad, because a pull request that adds a living doc owes one more line, in `docs/README.md`.
- Bad, because the anchor rule is a reading of GitHub's documented rule, not GitHub's own code; a
  heading shape outside it could read red where GitHub resolves it.

## What would make this wrong

- GitHub changes its heading-anchor rule, so a link that resolves on GitHub reads red here.
- The queue file grows a form the small reader does not know (an anchor, a flow list); the reader
  then refuses it by assertion, and a fuller reader is owed.
- The tree grows living docs outside Markdown that the index should list.
