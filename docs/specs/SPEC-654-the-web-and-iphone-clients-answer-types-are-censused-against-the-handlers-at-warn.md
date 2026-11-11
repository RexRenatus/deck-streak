# SPEC-654: the web and iPhone clients' answer types are censused against the handlers at warn

Path: `docs/specs/SPEC-654-the-web-and-iphone-clients-answer-types-are-censused-against-the-handlers-at-warn.md`.

- **Issue:** #1372, in part: this SPEC adds the census; the generator that would make the client
  types from the handlers' own types is a later delivery, and this SPEC closes nothing.
- **Context(s):** the repository's guard tests (`scripts/tests`), their fixtures, the rows band and
  the documents. No product source, workflow, manifest, lock file or `scripts/check.sh` changes.
- **Decided by:** ADR-676 (the client types are a text census at warn before any generator). It
  cites ADR-450 D6 for the promotion rule and amends nothing.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-654.md`.

## 1. The problem, measured

The web client parses each answer of the request handlers by hand, and nothing compares the keys a
handler sends with the keys its client reads. Over the tree this SPEC was cut from, the files
directly under `crates/api/src` were read 24 by the measurement. Over those 24 the census of
section 2 examines 46 routes, the 19 `read(` calls of the web client and 80 Swift files, 65 types
in all: 12 pairs are equal; 4 are drifted (the score's `reviews` and the governor's `standby` and
`lapse` are sent and unread; the streak's track keys are put in a map, a form the census does not
read; the feed's items are a type the census cannot reach); 3 are unmeasured (their parsers read
outside the named forms); and 27 routes have no `read(` call and no Swift type. The routes of the
unread files were not measured: no listed entry holds them, so the census prints each as a `WARN`
line at warn and passes. No Swift type mirrors a route: the app has no
API layer yet. Most answers are `json!` literals, so no derive-based generator can reach them today.

## 2. Requirements

R1. One module, `scripts/tests/test_client_types_census.py`, reads as text the files directly
    under `crates/api/src`, `web/app/src/lib/api.ts` with the modules its parsers are imported
    from, and every `.swift` file under `ios/`. It runs no build and imports no product code.
R2. SERVER-ROUTE and SERVER-ANSWER find each route and the keys its handler sends, as the module's
    docstring names them; an object whose only key is `reason` is a refusal, never an answer.
R3. WEB-BINDING finds every `read(` call in `api.ts`, an awaited read in an async binding and a
    read with a template path among them, and WEB-READ the keys its parser reads.
R4. SWIFT-MIRROR finds each Swift type that mirrors a route. Zero mirrors print their count and
    pass; zero Swift files are refused.
R5. A route and a client type pair by method and path. A pair is equal, drifted (each key the
    server lacks named after `missing=`, each key the client leaves unread after `unread=`) or
    unmeasured (one side's keys empty); a client type with no route and a route with no client type
    are unmatched. Every non-equal pair is a finding, keyed `METHOD PATH side`.
R6. The module prints `examined N` for handler files, routes, web bindings, Swift files and types
    (routes plus client types), and refuses each at zero as void.
R7. At `LEVEL` warn an unlisted finding prints its `WARN` line and passes; at fail it fails. A
    listed finding prints its `LISTED` line with its reason. A listed entry no finding matches is
    `STALE` and fails at both levels.
R8. `LEVEL` is warn. `LISTED` holds the 34 findings section 1 gives for the 24 measured handler
    files, each with its reason; the routes of the file not measured are held by no entry and
    print as `WARN` lines at warn.
R9. Moving `LEVEL` to fail is its own delivery under ADR-676's promotion rule.
R10. Six planted trees under `scripts/tests/fixtures/client-types/` hold an equal pair, a drifted
    key, an unmatched client type, an empty population, a read inside an async binding and a read
    with a template path.

## 3. Acceptance criteria of SPEC-654

| id | criterion | decided by |
|---|---|---|
| A1 | R2 to R5: the equal tree holds two equal pairs, the web binding and the Swift type, and no finding | `PlantedTrees.test_an_equal_pair_reads_no_finding` |
| A2 | R5, R7: the drifted tree holds exactly one finding, `drifted unread=extra`, which warns at warn and fails at fail | `PlantedTrees.test_a_drifted_field_is_named_warns_at_warn_and_fails_at_fail` |
| A3 | R5: the unmatched tree holds the client type with no route and the route with no client type | `PlantedTrees.test_an_unmatched_client_type_and_its_route_are_found` |
| A4 | R7: a listed finding prints its reason at fail, and a listed entry no finding matches is stale at both levels | `PlantedTrees.test_a_listed_finding_reads_listed_and_a_stale_entry_fails_at_both_levels` |
| A5 | R6: the empty tree fails naming `examined 0 handler file(s)` | `PlantedTrees.test_an_empty_population_is_void` |
| A6 | R1, R8: over the repository `LEVEL` is warn, no line fails and some pair is equal | `TheTree.test_the_tree_holds_todays_listing_at_warn` |
| A7 | R3: the awaited-read tree's read inside an async binding pairs equal, and no finding | `PlantedTrees.test_a_read_inside_an_async_binding_is_paired` |
| A8 | R3: the template-path tree's read with a template path pairs equal with its `{id}` route, and no finding | `PlantedTrees.test_a_read_with_a_template_path_is_paired` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_an_equal_pair_reads_no_finding
A2: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_a_drifted_field_is_named_warns_at_warn_and_fails_at_fail
A3: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_an_unmatched_client_type_and_its_route_are_found
A4: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_a_listed_finding_reads_listed_and_a_stale_entry_fails_at_both_levels
A5: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_an_empty_population_is_void
A6: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_the_tree_holds_todays_listing_at_warn
A7: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_a_read_inside_an_async_binding_is_paired
A8: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_a_read_with_a_template_path_is_paired
```

Every expected key, line and count is a literal the test spells, never a value the census computes.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_client_types_census.py` | the census and its tests | added: R1 to R9, A1 to A8 |
| `scripts/tests/fixtures/client-types/` | the planted trees | added: R10 |
| `scripts/mutation-rows.d/S65400-S65499.json` | the rows | added: S65400 to S65411 (section 8) |
| `docs/specs/SPEC-654-the-web-and-iphone-clients-answer-types-are-censused-against-the-handlers-at-warn.md` | the spec | added |
| `docs/decisions/ADR-676-the-client-types-are-a-text-census-at-warn-before-any-generator.md` | the decision | added |
| `docs/red-first/SPEC-654.md` | the record | added |
| `changelog.d/client-types-census-654.md` | the fragment | added |

## 5. What this does NOT cover

- It adds no generator, no dependency and no lock change; the generator is its own delivery.
- It does not compare request bodies: the web client posts its three JSON bodies by hand, outside
  WEB-BINDING.
- It does not read a type another crate serializes for a handler, a key a handler puts in a map,
  or a key built outside the named forms; each such pair is listed with that reason.
- It does not compare value types, only key names.

## 6. Risks

- **A sibling change moves a pair.** A pull request that adds a route, a read binding or a key
  can turn a listed entry stale, and the census fails naming it; the listing is updated in that
  pull request.
- **A form the census does not read.** The pair reads unmeasured or unmatched and is listed with its
  reason, never silently equal.

## 7. Formal model

None. The census is a pure reader of committed text with no state, actor or write path; its
classes are closed by the seven planted-tree criteria of section 3 and by section 8's rows.

## 8. Mutation rows (S65400-S65499)

Rows S65400 to S65411 in `scripts/mutation-rows.d/S65400-S65499.json`, each on
`scripts/tests/test_client_types_census.py`. Each find is taken from the green commit's text and
occurs exactly once in the file.

| rows | what each mutant breaks | killer |
|---|---|---|
| S65400 to S65404 | the route form, the answer keys, the refusal shape, the web read, the Swift mirror | A1 |
| S65405, S65408 | the unread keys, the level's choice | A2 |
| S65406, S65407 | the unmatched client type, the unmatched route | A3 |
| S65409 | the stale entry | A4 |
| S65410 | the refusal of an empty population by its own name | A5 |
| S65411 | `LEVEL`'s warn | A6 |
