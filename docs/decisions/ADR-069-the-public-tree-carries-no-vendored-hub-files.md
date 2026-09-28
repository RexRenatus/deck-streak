---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The public tree carries no vendored hub files, and the box run judges every pack

## Context and Problem Statement

ADR-056 kept the packs box-only in two halves: public CI ran the vendored probes' rows from
`.packs/`, and the maintainer's box ran the packs built into the runner. The owner's box-only
ruling, applied in full, moves every pack verdict to the box. The vendored files name the
maintainer's private tooling, which public text may not do (ADR-059), and with the ruling applied
they have nothing left to do in public CI. Three pieces of DeckStreak's own code still read them: the
vault adapter compiles in the vault-duties pack's rails, layout and rows; the public scrub composes
its rules from two packs' deny lists; and the deploy templates' tests parse units with a pack's
lint. Public CI holds no private checkout (ADR-017). 53 files carry `phx.*` schema ids, which the
box-run packs judge DeckStreak's outputs by. And 21 acceptance lines of seven delivered SPECs run a
test this removal takes away, while an accepted SPEC changes only insert-only. How is every pack judged, with nothing of the
maintainer's tooling in the public tree?

## Decision Drivers

- The owner's box-only ruling, applied fully: the box run is the one place a pack is judged.
- ADR-059: public text describes DeckStreak only, and names none of the maintainer's tooling.
- The vault's rails and the scrub are security checks: their behaviour must not change.
- Public CI has no private checkout, and never will (ADR-017).
- A reviewer must still be able to read how a pack verdict is reached.

## Considered Options (the alternatives it was chosen against)

- A public driver that names no private tool and reads a private wiring file, with DeckStreak owning the data its code reads and the box checking that data for drift — chosen: nothing private is published, every refusal keeps a public test, and each security check keeps its exact behaviour.
- Move the whole driver into the maintainer's private tooling — rejected because its refusals (VOID without its wiring, stale expectations, unreadable issues) would lose their public tests, and nobody could read how a verdict is reached.
- Keep the vendored copies in `.packs/` — rejected because they name the maintainer's private tooling (ADR-059), and the ruling leaves them no job in public CI.
- Read the packs' data at run time from a checkout — rejected because public CI holds no checkout (ADR-017), and it builds the vault adapter and runs the scrub.
- Rename the `phx.*` schema ids to a DeckStreak prefix — rejected because every parity golden would be regenerated and the box-run packs taught new ids, for a prefix that names a schema, not a tool.
- For a criterion whose test this removal takes away, retire it insert-only: strike its id, and set its command and its red-first lines apart in `retired` fences — chosen: every accepted byte is kept (ruling (i)), and the sdd and tdd packs stop judging a test that no longer exists.
- Leave those acceptance lines as they are — rejected because the tdd pack refuses the 21 lines that select no test, so the box run could never read green.
- Delete those acceptance lines and their red-first lines — rejected because an accepted SPEC and its record change only insert-only (ruling (i)).
- Keep the removed subjects' tests, run against synthetic stand-ins — rejected because they would test nothing DeckStreak ships.

## Decision Outcome

Chosen option.

- **The public tree** holds no vendored pack file: `.packs/`, the row runner, the vendoring script,
  the four methodology probes and the settings scan are removed (SPEC-056 R1). `methodology.json`
  keeps DeckStreak's own configuration, which the box run's probes read with `--root`.
- **Public CI** runs DeckStreak's own stages: fmt, clippy, test, test-engine, doctest, the two
  audits, web, python, scrub and secrets. It has no `packs` stage or job.
- **The box run** is `scripts/box-packs.sh`, public, naming no private tool. `PACKS_WIRING` names a
  private file, `deckstreak.box-wiring.v1`, which holds the pin, the packs and their states, the box
  section's expectations, the methodology probes' and the proxy-client scan's paths, the skills
  directory and each owned file's source; `PACKS_CHECKOUT` is the private checkout at the pin, and
  `PACKS_RUNNER` the runner built from it. It refuses VOID by name when one is unset.
  - Every pack runs through the runner with the verb its catalog admits, and each card is read by
    the suffix of its schema. The `packs` section keeps the removed row runner's judgment
    (enforced, pending, deferred, excluded and deferred rows, stale); the `box` section keeps its
    own (expected reds, pending, issues read with `gh`).
  - The sdd, ddd and tdd probes and the proxy-client and apiKeyHelper scans run from the checkout
    against the judged tree, each with its own verdict line. The apiKeyHelper scan keeps the
    refusals the public gate gave it, and reads `pending` while the tree holds no settings file.
  - `--post-status` posts one `box/packs` commit status on the judged commit, with no row detail
    and no private name. It is not a required check: a new required context would strand the pull
    requests opened before it, and making it one is a later decision.
- **DeckStreak owns the data its code reads**, reduced to the fields its parsers read:
  `crates/vault/data/` (the rails, the default layout, the gate's classes) and `scripts/scrub-rules/`
  (the scrub's public shapes). The deploy templates' tests read units with `scripts/tests/_units.py`.
  The box run compares each owned file with its source at the pin and fails naming the first field
  that differs, so the rails DeckStreak enforces cannot drift from the pack that judges them.
- **The `phx.*` ids** stay as opaque schema identifiers: the box-run packs require them, and the
  parity goldens' digests include them. A later rename is the maintainer's to make.
- **A criterion whose test is removed** is retired insert-only (SPEC-056 R14): its id is struck in
  its SPEC's table, and inserted fence lines set its command and its red-first lines apart in
  `retired` fences. Its SPEC's dated amendment section says why its subject is gone and what
  judges it now. The sdd and tdd packs read only `acceptance` and `red-first` fences and ignore
  every other, so a `retired` fence holds by that rule: it is a convention the packs honour, not a
  form they name.

This supersedes ADR-056's public half (CI's pack stage over the vendored rows), ADR-004's vendoring,
ADR-039's vendoring script and ADR-030's driver interface (its variables, and the wiring and pin it
read from the judged commit). The rest of each stands.

### Consequences

- Good, because the public tree names nothing of the maintainer's tooling, and one run judges every pack.
- Good, because the rails and the scrub keep their behaviour exactly, and the drift check keeps them equal to their sources.
- Bad, because public CI no longer shows SPEC-shape or pack verdicts: the maintainer's box run is the one place they are judged, which is the ruling applied in full.
- Bad, because a pull request's pack verdict waits for that run; `box/packs` makes it visible.
- Bad, because the retired form rests on the packs ignoring every fence but their own: a pack that learned to read another fence could judge a retired line again.

### Confirmation

SPEC-056's acceptance tests (A1 to A18); the box run through the new interface reading `BOX PACKS OK`
with the sdd, ddd and tdd probes' lines, both scans' and the drift check's; and the scrub's counts,
unchanged.

## What would make this wrong

- A published pack runner that public CI can install at a pinned version: then pack verdicts could
  return to public CI without a private checkout.
- The box run stops being run before merges, so `box/packs` goes missing on merged pull requests.
- The sdd or tdd pack starts reading fences other than its own: the retired lines would be judged
  again, and the packs would need a retired-criterion form of their own.

## More Information

ADR-004; ADR-030; ADR-039; ADR-056; ADR-059; SPEC-056; #60.
