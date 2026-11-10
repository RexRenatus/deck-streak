# SPEC-656: the web client holds strict equality and one spelling for an absent value

Path: `docs/specs/SPEC-656-the-web-client-holds-strict-equality-and-one-spelling-for-an-absent-value.md`.

- **Issue:** #1374. Follow-up to SPEC-436 and ADR-450, from #913.
- **Decided by:** ADR-678. It cites ADR-450 (a new rule starts at warn and is promoted by its own
  delivery under D6) and SPEC-030 (the house's text census over the web client's tests).
- **Status:** delivered by the pull request that adds this file, with
  `docs/red-first/SPEC-656.md`.

## 1. The problem, measured

No check refuses `==` or `!=` in the web client's TypeScript, and none holds one spelling for an
absent value: the tree holds no lint configuration, and a search for `eqeqeq` or
`strict equality` matches no file. The web app's `tsconfig.json` sets `strict`, which keeps
`null` and `undefined` apart as types, and refuses neither form.

A text reader at the measured base (each count below a lower bound, never an exact count over
the census's population; a file it did not read stays in that population, and a site there prints
at warn unlisted) found no loose comparison in the code of `web/app/src`'s `.ts` files and components.
The `==` and `!=` texts it saw sit in two test files, each inside a regular expression or a string.

At the client's boundaries an absent value is spelled `null`: the exported interfaces and type
aliases of `web/app/src/lib/api.ts` and the fifteen modules it imports a type from hold at least 34
`| null` members against 0 `| undefined`, 7 optional members and 1 optional parameter. A page's
state that holds an `Answer` not yet answered is spelled `undefined` in at least 14 `$state` sites
and one prop. The census lists those 23 sites.

## 2. Requirements

R1. A census, `scripts/tests/test_web_strict_equality.py`, reads every `.ts` file and every
    `.svelte` component under `web/app/src`, test files included, as text, and prints
    `examined <n> web source file(s)`; 0 is VOID.
R2. Code is read as the compiler reads it: a comment, a string, a regular expression literal and a
    template literal's text are never code; a template literal's `${ }` is; a component's
    `<script>` blocks and each `{ }` expression of its markup are code, and its `<style>` blocks
    and HTML comments are not.
R3. `loose-equality`: each `==` and `!=` in code that is not `===` or `!==` is a site, keyed by its
    file and its ordinal in the file.
R4. The boundary is the API module, `web/app/src/lib/api.ts`, and each module it imports a type
    from (an `import type` or a `type` name); the census prints
    `examined <n> boundary module(s)`, and 0 is VOID.
R5. `absent-spelling`: the client spells an absent value `null` at its boundary. A site is an
    optional member (`name?:`), an optional parameter or a `| undefined` in an exported interface
    or type alias of a boundary module, keyed by its file, its declaration, its form and its
    ordinal; a `$state` in a component or a `.svelte.ts` module whose type names `Answer<` and
    starts with no value or `undefined`, or holds `| undefined`, keyed by its variable; and a `?:`
    or `| undefined` in a `$props()` declaration that names `Answer<`, keyed by its form and
    ordinal. A test file is no boundary.
R6. `LEVEL` is `warn`. An unlisted site prints `WARN <rule> <key> line <n>` and passes at `warn`,
    and fails at `fail`; a listed entry no site matches fails as `STALE <rule> <key>` at both.
R7. `LISTED` holds today's 23 `absent-spelling` keys, each with its reason, and no
    `loose-equality` key.
R8. Moving `LEVEL` to `fail` is its own delivery, under ADR-450 D6; no lint package is added.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | R1, R4, R6, R7: over the repository, nothing fails, `LEVEL` is `warn`, the API module is examined and every listed key carries a reason | `test_web_strict_equality.TheWebClientHoldsStrictEqualityAndOneAbsentSpelling.test_the_repository_holds_exactly_todays_listing` |
| A2 | R2, R3: in the planted tree each line marked `planted: loose-equality` is found, with its exact key, and a strict comparison, a comment, a string, a regular expression, template text, an HTML comment and markup text are not | `test_web_strict_equality.TheWebClientHoldsStrictEqualityAndOneAbsentSpelling.test_a_planted_loose_comparison_is_found_and_its_lookalikes_are_not` |
| A3 | R4, R5: in the planted tree each line marked `planted: absent-spelling` is found, with its exact key; `null`, a module imported for a value, an unexported interface, a non-Answer state and props that name no Answer are not | `test_web_strict_equality.TheWebClientHoldsStrictEqualityAndOneAbsentSpelling.test_a_planted_boundary_with_the_other_spelling_is_found` |
| A4 | R6: an unlisted site prints its exact `WARN` line at `warn` and fails at `fail`; a stale entry fails at both | `test_web_strict_equality.TheWebClientHoldsStrictEqualityAndOneAbsentSpelling.test_the_level_decides_an_unlisted_site_and_a_stale_entry_always_fails` |
| A5 | R1, R4: an empty population and a population with no API module are each VOID | `test_web_strict_equality.TheWebClientHoldsStrictEqualityAndOneAbsentSpelling.test_an_empty_population_is_void` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_web_strict_equality.py -k test_the_repository_holds_exactly_todays_listing
A2: python3 -m unittest discover -s scripts/tests -p test_web_strict_equality.py -k test_a_planted_loose_comparison_is_found_and_its_lookalikes_are_not
A3: python3 -m unittest discover -s scripts/tests -p test_web_strict_equality.py -k test_a_planted_boundary_with_the_other_spelling_is_found
A4: python3 -m unittest discover -s scripts/tests -p test_web_strict_equality.py -k test_the_level_decides_an_unlisted_site_and_a_stale_entry_always_fails
A5: python3 -m unittest discover -s scripts/tests -p test_web_strict_equality.py -k test_an_empty_population_is_void
```

Every planted line is text a fixture spells literally, and every expected key is a literal the
test writes, never a value the census computes.

## 4. File manifest

| path | change |
|---|---|
| `scripts/tests/test_web_strict_equality.py` | added: R1 to R7, A1 to A5 |
| `scripts/tests/fixtures/web-strict-equality/plant/web/app/src/lib/api.ts.fixture` | added: the planted API module |
| `scripts/tests/fixtures/web-strict-equality/plant/web/app/src/lib/views.ts.fixture` | added: a module the API imports a type from |
| `scripts/tests/fixtures/web-strict-equality/plant/web/app/src/lib/paths.ts.fixture` | added: a module the API imports only a value from |
| `scripts/tests/fixtures/web-strict-equality/plant/web/app/src/lib/Header.svelte.fixture` | added: props that name an Answer |
| `scripts/tests/fixtures/web-strict-equality/plant/web/app/src/lib/Plain.svelte.fixture` | added: props that name none |
| `scripts/tests/fixtures/web-strict-equality/plant/web/app/src/routes/+page.svelte.fixture` | added: state, script and markup |
| `scripts/tests/fixtures/web-strict-equality/empty/web/app/src/keep.fixture` | added: the empty population |
| `scripts/mutation-rows.d/S65600-S65699.json` | added: the rows of section 9 |
| `docs/specs/SPEC-656-the-web-client-holds-strict-equality-and-one-spelling-for-an-absent-value.md` | added |
| `docs/decisions/ADR-678-a-text-census-holds-strict-equality-and-null-at-the-web-clients-boundary-at-warn.md` | added |
| `docs/red-first/SPEC-656.md` | added |
| `changelog.d/web-strict-equality-656.md` | added |

## 5. What this does NOT cover

- It adds no lint package, configuration or CI step; the lint route is its own delivery, #1404.
- It cures no listed site; each cure is a product change, #1404.
- It does not move `LEVEL` to `fail`; that is its own delivery under ADR-450 D6, #1374.
- It reads no TypeScript outside `web/app/src`, #1404.

## 6. Risks

- **A site this SPEC's count missed.** The build re-derives both lists at its base; a count that
  differs stops the build for a new measurement, never a silent relisting.
- **A sibling change adds or moves a site.** An open pull request that changes
  `web/app/src/lib/engine/protocol.ts`, `web/app/src/lib/study/ReviewScreen.svelte` or
  `web/app/src/routes/progress/+page.svelte` and lands first can make an entry STALE; detected by
  A1 at the merge.
- **The reader misreads a form the compiler accepts.** A false site prints at warn; a false STALE
  fails A1 by name, closed.
- **A tree-scanning guard reads the new files.** The fixtures carry the `.fixture` suffix, so no
  Vitest, svelte-check or probe glob reads them.

## 7. What only CI proves

- A1 reads the repository, so its red and green are read in CI's `hygiene` job by name.

## 8. Formal model

No state, actor or write path is added; the census is a pure reader of committed text, closed by
the planted populations of A2 to A5 and by section 9's rows. No model checker runs.

## 9. Mutation rows (S65601-S65626)

Rows S65601 to S65626 in `scripts/mutation-rows.d/S65600-S65699.json`, each on
`scripts/tests/test_web_strict_equality.py`, each find taken from the green commit's text and
occurring exactly once. S65601 to S65611 and S65626 are killed by A2, S65612 to S65620 by
A3, S65621 and S65622 by A4, S65623 by A1, and S65624 and S65625 by A5.
