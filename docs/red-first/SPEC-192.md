# Red-first record: SPEC-192

SPEC-192 and ADR-192 were committed alone (2a2193a). The guard came next (4fd0748), against the
unchanged tree: its five fixture tests pass, and the one test that walks `crates/*/src` fails by
assertion. The tests, the fifteen rows and the changelog fragment then turned it green (a6a850f).
The replay ran the whole guard file on 4fd0748's tree: six tests, one red by assertion.

```red-first
A1: red at 4fd0748: Lists differ: ['analytics::LeechThreshold (src/settings.[1167 chars]er"'] != [] : First list contains 15 additional elements : 24 impl(s) examined
A1: green at a6a850f
A2: not red: the leech threshold already carried the literal, and the test spells it; the row S19211 proves the test kills the rewritten literal
A3: not red: the mini app URL already carried the literal; the row S19212 proves the test kills the rewritten literal
A4: not red: the API URL already carried the literal; the row S19213 proves the test kills the rewritten literal
A5: not red: the four ingest settings already carried their literals; the rows S19202 to S19205 prove the test kills each rewritten literal
A6: not red: the courses file path already carried the literal; the row S19206 proves the test kills the rewritten literal
A7: not red: the hour, offset, worker and credentials settings already carried their literals; the rows S19207 to S19210 prove the test kills each rewritten literal
A8: not red: the taxonomy path already carried the literal; the row S19214 proves the test kills the rewritten literal
A9: not red: the folder setting already carried the literal; the row S19201 proves the test kills the rewritten literal
A10: not red: the request file already carried the literal; the row S19215 proves the test kills the rewritten literal
A11: red at f6a1b59: AssertionError: 1 != 2 ; AssertionError: 0 != 1 ; AssertionError: Lists differ: [] != ['demo::Depth (src/depth.rs) "a whole depth"'] ; AssertionError: 0 != 2 : four of the ten tests in the module fail by assertion
A11: green at 68c239b
A12: red at 9212a24: AssertionError: 1 != 2 ; AssertionError: 0 != 1 ; AssertionError: Lists differ: ['demo::Depth (src/depth.rs) "a whole depth"'] != [] ; AssertionError: 0 != 1 ; AssertionError: 0 != 1 : five of the seventeen tests in the module fail by assertion
A12: green at a8d57e9
```

## Addendum, 2026-09-29: the guard examines every impl and counts only a test's spelling

Criterion A11 joins the fence above. Its four tests were committed alone at f6a1b59, against the
guard as the first round left it: each fails by assertion, in this order
`test_a_generic_implementation_is_examined` (`AssertionError: 1 != 2`),
`test_a_shape_only_a_comment_spells_is_refused` (`AssertionError: 0 != 1`),
`test_a_shape_only_a_production_line_of_the_crate_spells_is_refused`
(`AssertionError: Lists differ: [] != ['demo::Depth (src/depth.rs) "a whole depth"']`) and
`test_a_shape_two_implementations_of_one_crate_share_needs_a_row_of_each_file`
(`AssertionError: 0 != 2`); the rest of the module passes. The guard change at 68c239b turns the
whole module green (`Ran 10 tests`, `OK`, `examined 24 Setting impl(s)`).

Disclosure: the bodies of the first round's guard tests changed after the red commit 4fd0748 (a
presence assertion was added beside each absence, at 5fdf8b2), so the A1 red line above quotes the
earlier bodies. The later bodies, run at 4fd0748, fail the same way:
`Lists differ: ['analytics::LeechThreshold (src/settings.[1167 chars]er"'] != [] / First list contains 15 additional elements`.

## Addendum, 2026-09-29: the guard reads Rust source as the compiler does

Criterion A12 joins the fence above. Its seven tests, the class `TheGuardReadsRustSource`, were
committed alone at 9212a24, against the guard as the second round left it: five fail by assertion
and two pass (the plain macro impl and the own-module spelling).

- `test_a_macro_implementation_through_dollar_crate_is_examined`: `AssertionError: 1 != 2`
- `test_a_production_line_after_the_own_files_test_module_is_refused`: `AssertionError: 0 != 1`
- `test_a_shape_a_test_spells_after_a_url_on_its_line_is_pinned`: `AssertionError: Lists differ: ['demo::Depth (src/depth.rs) "a whole depth"'] != []`
- `test_a_shape_only_a_block_comment_spells_is_refused`: `AssertionError: 0 != 1`
- `test_a_test_attribute_on_a_use_opens_no_test_module`: `AssertionError: 0 != 1`

The guard change at a8d57e9 turns the class green (`Ran 7 tests`, `OK`) and the whole module
(`Ran 17 tests`, `OK`, `examined 24 Setting impl(s)`). A11's nine tests are unchanged; its
criterion text now maps one clause to each of them.

## The population: 24 `impl Setting for`, and how each is pinned

The guard's 24 (`examined 24 Setting impl(s)`), from `git grep -n 'impl .*Setting for' -- crates |
grep /src/` at the base c814bbc. Plants are the middle letter to `Q` and `XX<shape>XX`, each run
with `cargo test -p <crate> --tests --no-fail-fast`.

| crate | impl | pinned by |
|---|---|---|
| agent | `RosterPath` | row S05766 (SPEC-057), test `persona::a_roster_outside_the_repository_fills_the_four_slots` |
| agent | `AiRoute` | row S04302 (SPEC-043), test `constants::the_ai_route_setting_states_its_shape`; both plants fail exactly that test |
| analytics | `LeechThreshold` | row S19211, test `rollup_metrics::a_malformed_leech_threshold_is_refused_naming_its_whole_shape` |
| api | `SocketSetting` | row S05772, test `listen::a_listen_setting_that_is_not_an_address_is_refused_naming_its_shape` |
| bot | `MiniAppUrl` | row S19212, test `commands::the_mini_app_url_is_https` |
| bot | `ApiUrl` | row S19213, test `transport::the_api_url_is_https_or_loopback_http` |
| daemon | `StateDirectory` | row S05751, test `roles::a_relative_state_directory_is_refused_naming_the_shape_it_must_have` |
| daemon | `NotifyAddress` | row S05755, test `lifecycle::tests::each_private_setting_states_its_shape` |
| daemon | `Micros` | row S05756, the same test |
| daemon | `Text` | row S05757, the same test |
| daemon | `RequestFile` | both plants survived (rc 0, no failure); row S19215, test `sync_request::a_relative_request_path_is_refused_naming_its_whole_shape` |
| identity | `Freshness` | pinned by `init_data::the_freshness_bound_is_read_from_its_setting`; no row |
| ingest | `SyncEndpoint` | row S19202, test `settings::each_malformed_setting_is_refused_naming_its_whole_shape` |
| ingest | `StateDirectory` | row S19203, the same test |
| ingest | `IncludeDecks` | row S19204, the same test |
| ingest | `LawDeckRoot` | row S19205, the same test |
| kernel | `CoursesPath` | row S19206, test `courses_config::the_courses_file_refuses_a_duplicate_or_overlapping_course` |
| kernel | `Hour` | row S19207, test `settings::a_malformed_setting_is_refused_by_name_without_its_value` |
| kernel | `UtcOffset` | row S19208, the same test |
| kernel | `OffloadWorkers` | row S19209, the same test |
| kernel | `CredentialsDirectory` | row S19210, the same test |
| readings | `TaxonomyPath` | row S19214, test `topics::a_taxonomy_file_that_names_a_deck_badly_is_refused_whole_and_never_quoted` |
| vault | `VaultRoot` | pinned by `confinement::the_vault_settings_refuse_by_name_and_never_by_value`; no row |
| vault | `FolderName` | row S19201, the same test |

Fifteen rows in this delivery's band (S19201 to S19215); seven implementations pinned before it by
rows of other bands (five of them by #354) and two by tests alone.

## Addendum, 2026-09-29: the guard's tests kill every lexer-arm mutant

Criteria A13, A14 and A15 join the fence (SPEC-192 section 9). The eight rewrites of issue #406 (six of the
lexer, two of the selection of spellings) each survived the base guard (`Ran 17 tests`, `OK`, `examined 24 Setting impl(s)`). 7034826c
strengthens assertions inside the existing tests of A11 and A12, so each rewrite now turns the module
red by assertion, and the unmodified guard prints the same `examined 24 Setting impl(s)`; that
commit edits a test file, so A15 is recorded `not red`.

The new tests of A13 (five) and A14 (one) were committed alone at faaf43ec against the guard as
7034826c left it: five of the six fail by assertion; the sixth,
`test_a_file_that_is_no_declared_test_module_is_not_read_as_one`, passes there, because it pins a
refusal the guard already made.

```red-first
A13: red at faaf43ec: AssertionError: Lists differ: ['demo::Depth (src/depth.rs) "a whole depth"'] != [] : four of the five tests of TheGuardReadsOutOfLineTestModules, each naming the implementation's file
A13: green at 77ed28a0
A14: red at faaf43ec: AssertionError: 2 != 1 : test_a_block_comment_holding_an_impl_is_not_examined
A14: green at 77ed28a0
A15: not red: the base guard already passed the assertions added inside A11's and A12's tests; the rows S19216 to S19256 prove each kills its rewrite
```

The red run (`Ran 23 tests`, `FAILED (failures=5)`; one is the `lib.rs` case, `Lists differ:
['demo::Depth (src/lib.rs) "a whole depth"'] != []`). The guard change at 77ed28a0 edits the test file itself
(the guard and its tests are one module: the guard code and its docstring, no assertion); it turns the whole module green (`Ran 23 tests`,
`OK`, `examined 24 Setting impl(s)`). Thirteen rows (S19216 to S19228) are proved KILLED by full id.

## Addendum, 2026-09-29, round 1 fix: three more arms are pinned

Three rewrites of the guard were planted in a scratch copy, one at a time, and each one survived the
tests as the head had them (`Ran 23 tests`, `OK`). e195540e adds a fixture line and an assertion (X10), edits an existing
string (X2) and adds a tuple element (H3) in three existing tests (no test is renamed or removed), so each rewrite now turns the module red by
assertion; the unmodified guard prints `Ran 23 tests`, `OK`, `examined 24 Setting impl(s)`. The
commit edits a test file. It also changed the A13 and A15 fence lines and SPEC-192's A15
acceptance line (6f04f76d); these arms are recorded here and not as new criteria: they belong to
A13 and A15. Rows S19226 to S19228 are proved KILLED by full id.

```text
X2 (a `//` comment kept in the skeleton): survives before; after, line 349, AssertionError: 0 != 1 : test_a_production_line_after_the_own_files_test_module_is_refused
H3 (the `#[cfg(test)]` requirement deleted from the out-of-line reading): survives before; after, line 413, AssertionError: 0 != 1 : #[allow(dead_code)] : test_a_file_that_is_no_declared_test_module_is_not_read_as_one
X10 (a raw string closed without its hashes): survives before; after, line 332, AssertionError: Lists differ: ['demo::Depth (src/depth.rs) "a whole depth"'] != [] : test_a_shape_a_test_spells_after_a_url_on_its_line_is_pinned
```

A15's decider now selects `TheGuardJudgesAPlantedTree` and `TheGuardReadsRustSource` (16 tests, `OK`).
Run under that selection alone, each of the eight rewrites turns it red.

## Addendum, 2026-09-29, round 2: the guard's tests kill every arm of its reader

A second mutation review planted 72 rewrites of the guard's reader in a scratch copy. At the head
(6f04f76d) 27 of them changed a verdict on a valid Rust tree and survived: nine of the lexer, seven
of the out-of-line reading and eleven of the selection of spellings. Seven more are equivalent on
valid Rust and are named in SPEC-192 section 8. Criteria A13 and A15 cover them; this round adds no
criterion.

The new assertions and fixture lines were committed alone at 58448ad2, against the guard as the head
left it. Only one is red there, by assertion: the guard follows a `mod tests;` declared inside an
inline `mod inner { }`, and reads a file the compiler would not.

```text
A13: red at 58448ad2: Ran 23 tests, FAILED (failures=1): AssertionError: 0 != 1 : mod inner {  (test_a_file_that_is_no_declared_test_module_is_not_read_as_one)
A13: green at 45515771: Ran 23 tests, OK, examined 24 Setting impl(s)
```

Every other new assertion passes at 58448ad2 (`not red`): each pins behaviour the head already had,
and the head's tests did not check it, so the proof is the row, not a red run. The guard change at
45515771 is the depth check in `out_of_line`: a declaration at brace depth above zero is skipped.

The replay ran the 73 rewrites (the 72, with the depth check deleted as the seventy-third) against
the committed tests: 66 turn the module red, and exactly the seven equivalents survive (X11, T8, R3,
R4, C3, C4, L1). S12 turns red by assertion, not by an error: the killer now catches the guard's
TypeError and fails with `a shape with no literal must be refused, not crash the guard`.

The 28 rows S19229 to S19256 are proved KILLED by full id; the band holds forty-one script rows,
S19216 to S19256.
