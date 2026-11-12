---
status: "accepted"
---

# ADR-678: a text census holds strict equality and null at the web client's boundary, at warn

Decides SPEC-656. Cites ADR-450 (a new rule starts at warn and is promoted only by its own
delivery, D6) and SPEC-030 (a text census over the web client's tests). Amends nothing.

## Context

The web client is TypeScript and Svelte under `web/app/src`, type-checked by svelte-check under
`strict`, which keeps `null` and `undefined` apart as types and refuses neither `==` nor a second
absent spelling. The tree holds no lint configuration. A text reader found no loose comparison in
code (a lower bound), and the boundary spells an absent value `null` in at least 34 declared members
against 8 optional members and parameters, while a page's unanswered `Answer` is `undefined` in 15 places
(SPEC-656 section 1).

## Decision Drivers

- No rule already in force is lowered, and no dependency is added without its own review.
- A new rule starts at warn with today's sites listed; promotion is its own delivery.
- Every check prints what it examined and fails on an empty population, which is VOID.
- Code is read as the compiler reads it: a comment, a string or a regular expression is not code.

## Decisions

**D1. A Python text census under `scripts/tests/` (SPEC-656 R1, R2).** It reads each file's text
through a reader that blanks comments, strings, regular expressions and template text and keeps
`${ }`, and a component's `<script>` blocks and markup expressions; its planted tests run with the
house's Python stage and need no package.

**D2. Two rules, keyed without lines (R3, R5).** A loose comparison is keyed by its file and
ordinal, a boundary site by its file, declaration, form and ordinal, or by its state variable.

**D3. The spelling is `null`, by count (R5).** The boundary's declared answer types hold at least 34
`| null` and 0 `| undefined`; the 23 sites that spell `undefined` today are listed with reasons.

**D4. The boundary is derived (R4).** It is the API module and each module it imports a type
from, read from its import lines, so a new answer module joins without an edit here.

**D5. Level and listing (R6 to R8).** `LEVEL` is `warn` and `LISTED` holds today's 23 keys; a fix
removes its entry. Moving to `fail` follows ADR-450 D6's promotion rule, by its own delivery;
demoting a fail rule is a signed ruling.

## What it was chosen against

- ESLint with `eqeqeq`, typescript-eslint and eslint-plugin-svelte now: rejected because each is a new package under its own dependency review, which this delivery does not add; it is the next delivery's route (#1404).
- A Vitest census reading sources through the pinned TypeScript compiler: rejected because the build route here runs no package manager, and the Python stage already holds the house's web text censuses.
- `exactOptionalPropertyTypes` in `tsconfig.json`: rejected because it is not part of `strict` and would refuse optional members across the app at once, with no warn start.
- Keying sites by line number: rejected because any edit above a site would break its entry.

## Consequences

- Good: every web source file is examined on each run, and a new loose comparison prints with its key.
- Bad: at warn a new site passes, visible in the test's output only.
- Bad: a text reader can misread a form the compiler accepts; a false site prints, a false STALE fails by name.

## What would make this wrong

- The reader reports a site the review finds correct in more than 1 of 5 cases: D5 proposes a demotion.
- The client adopts `undefined` as its absent spelling by count: D3 is revisited by its own delivery.
