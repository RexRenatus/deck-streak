---
status: "accepted"
---

# ADR-660: the token compiler joins the web class by its exact path

## Status

Accepted, with SPEC-636.

## Context

The mutation plan sorts each changed path into a class, and its `web` class is production code
under `web/app/src` only (SPEC-039 R2; `scripts/mutation-verdict.py:117-118` at the base).
`web/app/scripts` holds two files outside that class. `build-tokens.ts` compiles the design tokens:
every build runs it through the Vite plugin in `web/app/vite.config.ts` (`:6`, `:16`), and
`src/lib/design/tokens.test.ts` judges its `compileTokens` under Vitest. A change to it gets no
StrykerJS leg, and the weekly battery, which mutates what `stryker.config.json`'s `mutate` names
(`mutation-weekly.yml:209-214`), never reaches it. `record-docs-mermaid-fences.js` is a maintainer
tool run by hand with an authenticated `gh`, rendering through GitHub's Markdown API; no Vitest
test runs it.

## Decision

- D1. `web/app/scripts/build-tokens.ts` joins the `web` class by its exact path, at every site that
  spells the class: the classifier (`classify`), StrykerJS's `mutate` in
  `web/app/stryker.config.json`, the configuration check's copy (`STRYKER_MUTATE`) and the
  exclusions walk. The class takes no glob over `web/app/scripts` and no `scripts/**`.
- D2. `web/app/scripts/record-docs-mermaid-fences.js` stays outside every mutation class. No Vitest
  test runs it, so StrykerJS would find every mutant of it uncovered, and its one caller is a
  maintainer's local run against GitHub's Markdown API.
- D3. SPEC-039 R2's `web` class is superseded by SPEC-636 R3 for this one path. SPEC-039 and ADR-057
  are not edited.

## Options rejected

| option | why not |
|---|---|
| a `web/app/scripts/**` glob in the class | it would take in the Mermaid recorder, whose mutants no test can kill, and any later script without a decision |
| a `scripts/**` glob in `mutate` | the same, and the configuration check would hold a glob the classifier does not spell |
| a stated exclusion for `build-tokens.ts` too | it is a production build input that a Vitest test already reaches, so leaving its changes unmeasured has no reason |
| moving `build-tokens.ts` under `web/app/src` | it moves a build input the Vite configuration imports, for a classification change; the class can name the file where it is |

## Consequences

Measured at the base, and stated plainly:

- (a) The web judge scores a changed file's mutants on the whole file, not on its changed lines.
  `judge_web` (`mutation-verdict.py:2256-2314`) loops over every file of the StrykerJS report and
  every mutant in it (`:2271-2307`) with no filter on changed lines, and the shard leg runs
  `pnpm exec stryker run --mutate "$MUTATE"` over its dealt files whole (`ci.yml:991`); ADR-057
  chose whole files for web. So the first later pull request that changes `build-tokens.ts` meets
  every mutant of that file, any older survivor included.
- (b) The weekly battery runs `pnpm exec stryker run` with the configuration's `mutate`
  (`mutation-weekly.yml:214`), so it now reaches the file, and reports its survivors before any
  pull request meets them.
- The class, `mutate`, `STRYKER_MUTATE` and the walk move together: `configs` compares the two
  lists whole, so a drift between them is refused.

## What would make this wrong

- StrykerJS's Vitest runner cannot reach the file's mutants through `tokens.test.ts`, so they read
  `NoCoverage`; the first battery run that mutates the file shows it.
- A second build input appears under `web/app/scripts` and no SPEC names it, so its changes go
  unmeasured as this file's did.
- The Mermaid recorder gains a Vitest test, and the reason for D2 no longer holds.
