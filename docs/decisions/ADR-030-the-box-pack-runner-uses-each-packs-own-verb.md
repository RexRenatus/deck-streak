---
status: accepted
date: "2026-09-27"
decision-makers: "the DeckStreak architect (SPEC-030), the SPEC-030 builder"
---

# The box-pack runner runs each pack with its own verb, judges the committed tree without the vendored rules, and names every expected red

## Context and Problem Statement

ADR-004 runs the packs built into the packs' binary, and the subscription-proxy client scan, on the maintainer's
box with `scripts/box-packs.sh`. Run at `dev` c1f53c1 with a binary built from the vendored
packs commit, that script judged nothing (SPEC-030 section 1):

- every `<binary> pack probe` exited 4 with `skills/catalog.json not found`, because the script never
  passed `--skills-root`;
- four packs (web-security, cyber-pipeline, ux-laws, ui-styles) declare `<binary>.pack.run.v1`, and the
  script's hard-coded `pack probe` was refused (`wrong_verb`) for each;
- web-security read two vendored rule files under `.packs/scripts/` as DeckStreak's code and
  reported two false reds;
- the proxy scan exited 2 (VOID) with no settings document, and the script reported that as red.

Corrected by hand, the same tree had 37 red rows in seven packs. Some are real gaps with owners
(a missing Content-Security-Policy, #21; unvalidated init data, #17), others wait for a built site
(#59), and nothing recorded which reds were expected. How should the runner choose each pack's verb,
choose the tree it judges, and tell an expected red from a new one (SPEC-030 R10 to R14)?

## Decision Drivers

- the binary admits a pack to a verb only when the pack's catalog row declares that verb's card schema,
  so the catalog, not the runner, decides the verb, and a re-pin can move a pack between verbs.
- A rule's own source is not DeckStreak's code, and a pull request publishes its commits, not the
  maintainer's working copy.
- A red row nobody expects must fail the run by name; a red row somebody expects must name the open
  issue that builds its subject; an expectation that no longer holds must be removed, as SPEC-030
  R6 and R7 do for pending packs and deferred rows.
- No exit status alone is a verdict here: the binary exits 4 for a red card and for a refusal alike, and
  the proxy scan's `check all` exits VOID when a RED row is also present.
- Examining nothing is VOID, never a pass.

## Considered Options (the alternatives it was chosen against)

- Read each verb from the catalog, judge the exported commit, and name every expected red: chosen,
  because every false red of the c1f53c1 run disappears for a stated reason, and every red that
  remains is either named with an open issue or fails the run by name. The verb comes from
  `<binary> pack list`; the tree is `git archive` of `--rev` less `.packs/` and every path
  `.packs/VENDORED.json` lists; the expectations are a `box` section of `.packs/wiring.json`.
- Hard-code each pack's verb in the script: rejected, because it is the defect measured at c1f53c1:
  four packs had moved to `<binary>.pack.run.v1` and the script still called `pack probe`, and nothing
  but a refused run said so. The catalog is the binary's own admission rule.
- Judge the live working tree, `.packs/` included: rejected, because the vendored probes are rule
  code whose patterns read as findings (web-security's two false reds), and because an uncommitted
  file would be judged although no pull request carries it.
- Treat every red row as a failure with no named expectation: rejected, because the tree holds reds
  whose subjects are open issues of later waves, so the run would stay red until the last of them
  and could not tell a new red from a known one; ignoring reds instead would hide both.
- Decide each pack by its exit status: rejected, because exit 4 means both "red" and "refused", and
  the proxy scan's exit ranks VOID over RED.

## Decision Outcome

Chosen option.

- `.packs/wiring.json` gains a `box` object. `box.packs` names every pack the box runs (each pack
  the wiring marks `<binary>` must be there), and each entry may hold `expected_red` (row id to the
  open issue that builds that row's subject) or `pending` (the open issue after which the pack
  examines a row), never both, and a `note`. `box["proxy-client-scan"]` may hold `pending` and a
  `note`. `scripts/tests/test_pack_wiring.py` holds every such issue to the issue manifest.
- The verb comes from the pack's `requires_<binary>_schema`, as `<binary> pack list` reports it:
  `<binary>.pack.probe.v1` is `<binary> pack probe --skills-root <checkout>/skills`; `<binary>.pack.run.v1` is
  `<binary> --ledger <scratch> pack run --project <id>` against a ledger made in the scratch directory
  with `<binary> init` and `<binary> project register`; `<binary>.seo-pipeline.v1` is
  `<binary> verify seo-pipeline` over `web/site/dist` once the judged tree holds it. A schema no verb
  admits fails that pack by name.
- The judged tree is `git archive` of `--rev` (default `HEAD`) in a scratch directory outside both
  repositories, removed when the run ends. `.packs/` and every `VENDORED.json` path are removed
  before any pack runs; the wiring and the pin are read from that same commit.
- Each card is read by its schema. A blocking red row the wiring does not name fails the run by
  name; a named row that is not red is stale and fails the run; an advisory row never fails it. A
  pack that examines nothing reads `pending` with its issue, is VOID without one, and is stale once
  it examines a row. The proxy scan is read from its row lines: any RED fails, and with none it is
  pending while it examines no settings document (#29).
- The run prints one line per pack (its examined count, its unexpected, expected and stale rows) and
  a summary, exits 0 when no pack failed, 1 when one did, and 2 when it cannot judge (a missing
  variable, a pin that does not match, a malformed wiring).

### Consequences

- Good, because a new red is named on the pull request, and a fixed one names the expectation to
  delete, so the wiring cannot lag the tree in either direction.
- Good, because a pack that moves between verbs in a re-pin needs no edit to the runner.
- Good, because the tests drive the real script with a fake binary, so the runner's logic is proved in
  public CI although the packs themselves run only on the box.
- Bad, because expectations name rows by id: a row renamed in a re-pin reads as one stale
  expectation and one unexpected red until the wiring is edited.
- Bad, because a pack that examines nothing by design (ui-styles, whose rows are catalog-only)
  reads `pending` with the issue that settles it (#60), never green.

### Confirmation

`scripts/tests/test_box_packs.py` (SPEC-030 A9 to A13) drives `scripts/box-packs.sh` with
`scripts/tests/fixtures/box-packs/fake-runner` and a fake catalog. The maintainer's run with the real
binary is posted on each pull request (ADR-004).

## What would make this wrong

- the binary changes a card's schema or its row fields: the runner then reads no card and fails the pack
  by name, which is the signal to re-read the card.
- The open-source pack runner exists (#60): it replaces this script, and this ADR is superseded.

## More Information

ADR-004 (the vendored packs and the box run); SPEC-030; `docs/schematics/box-pack-runner.md`.

Amended by ADR-056 (2026-09-28): no pack runner will be published, so the premise that one
replaces this script is retired; this script is the permanent box runner.

Amendment (2026-09-28): names of the maintainer's private tooling were replaced with 'the box-run
packs' and neutral names for their repository, binary and checkout under the public-text rule
(ADR-059).
