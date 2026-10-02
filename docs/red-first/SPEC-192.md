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
A15: not red: the base guard already passed the assertions added inside A11's and A12's tests; the rows S19216 to S19277, less the six deleted ids, prove each kills its rewrite
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

## Addendum, 2026-09-29, round 3: the guard reads a module's file as rustc does, or refuses

R8's class is "the file the guard reads for an out-of-line test module". A third review found that a
stale `tests.rs` spelling could still pin the guard where rustc reads a different file, through
`cfg_attr`, other spellings of a `path` attribute, a `src/bin` file and a file loaded through
`#[path]`. The class rule is the union rule: the guard lists every file rustc could read for the
declaration and reads the one that exists; two, none, or any `path` attribute in any spelling read
none, so the shape stays refused. The population test generates the class: six declaring-file kinds,
seven attributes, a stale spelling at each other place rustc could look, and the shape spelled or not,
408 members. Criteria A13 and A15 cover it; this round adds no criterion.

The population test and the renamed path-attribute test were committed alone at 202a9529, against the
guard as the head left it. Both are red by assertion there.

```text
A13: red at 202a9529: Ran 24 tests, FAILED (failures=2): AssertionError: 0 != 1 : #[cfg(test)] #[path = "words/shape.rs"] mod tests; (test_a_module_whose_file_an_attribute_chooses_is_not_read, line 452)
A13: red at 202a9529: AssertionError: [tests.rs, tests/mod.rs] not found in ([tests.rs], []) : lib.rs '' stale=tests/mod.rs spelled=True (test_every_module_file_choice_is_read_from_rustcs_file_or_refused, line 526, the first member rejected)
A13: green at 52c898f7: Ran 24 tests, OK, examined 24 Setting impl(s), R8 population: 408 members
```

The five rows that read a `#[path]` value (S19239, S19240, S19242, S19243, S19244) lost their finds
with that reading and are deleted. Six rows pin the arms of the union rule (S19257 to S19262), each
proved KILLED by full id, and each red by assertion under its mutant. None of the six finds exists at
the previous head, so their survival there is not measurable; the killer's red under each mutant, on
the new code, is the evidence. The band holds forty-two script rows.

## Addendum, 2026-09-30, round 4: the guard reads a test module when rustc compiles it only under test

R8's class is every module the implementation's own file declares, inline or out-of-line, in any
visibility, as `name` or `r#name`: the guard reads a test module's source exactly when rustc
compiles the declaration under `--cfg test` and not without it, and reads exactly the source rustc
reads; otherwise it refuses, including when rustc reads no module file at all. A fourth review found
a `#[cfg(test)] mod tests;` whose attribute run also carries a `cfg` that removes it
(`#[cfg(any())]`, `#[cfg(not(test))]`, `#[cfg_attr(test, cfg(any()))]`, a feature that is off):
rustc reads no module file, and the guard read a stale `tests.rs` and pinned. The inline reading
admitted the same run. The rule reads the whole run, outer and leading inner, in three-valued logic
in which `test` is the one known option, and a module is a test module only when the run is true
with `test` and false without. Criteria A13 and A15 cover it; this round adds no criterion.

The generated test, with the operator table it is built from, was committed alone at 09074a62,
against the guard as the head left it. It builds its members from the guard's own operators and
asks rustc, at test time, which module sources each member compiles with `test` on and off under
every setting of two other options; when rustc is not on PATH it fails rather than skip. It is red
there by assertion. The rule commit 57842f68 edits the guard, which lives in the same file as the
generated test, and leaves the test's own hunks as committed at 09074a62.

```text
A13: red at 09074a62: Ran 24 tests, FAILED (failures=1): AssertionError: Lists differ: [...] != [] : 1683 of 4768 members, the first lib.rs #[cfg(x)] #[cfg(test)] mod tests; at tests.rs: reads ['tests.rs'], only under test [] (test_every_module_file_choice_is_read_from_rustcs_file_or_refused, line 682)
A13: green at 57842f68: Ran 24 tests, OK, examined 24 Setting impl(s), examined 3320 R8 member(s) judged against rustc
```

The replay of the 73 rewrites and the union rule's six against the committed tests turns 55 red;
exactly the seven equivalents (X11, T8, R3, R4, C3, C4, L1) survive, and 17 no longer apply because
their finds left with the old reading. Four rows are re-anchored on the new reading (S19227, S19238,
S19246, S19247) and S19256's killer moves to the generated test. S19241 is deleted, because the new
reading makes its rewrite equivalent. Fifteen rows pin the arms of the new reading (S19263 to
S19277), each proved KILLED by full id. The band holds fifty-six script rows.

## Addendum, 2026-09-30, round 5: the guard reads Rust with rustc's lexer

R8's class is drawn at rustc's lexer: for every source rustc accepts, the guard reads a module
declaration as the implementation's test module exactly when rustc compiles that declaration under
`--cfg test` and not without it, whatever else is configured, and reads it from the source rustc
reads for it; otherwise it refuses. Two shapes lie outside that sentence, disclosed and not claimed:
a file that a second declaration compiles without `test` (#458), and an item that a `cfg` removes
inside a compiled test module (#449). Its axes are the Reference's token grammar: whitespace and comments at every place
between an attribute's `#`, `!` and `[`, doc comments of each kind among the attributes, every
literal prefix and hash count, macro token trees of each delimiter, raw identifiers, a byte order
mark, a shebang, CRLF, and the `cfg` literals `true` and `false`. A fifth review found an inner
attribute whose tokens a space, a newline or a comment separated (`# ![cfg(any())]`), which the
guard did not read, and a raw C string, which it misread so that a later spelling counted. The rule
reads every file with a tokenizer for the Reference's grammar and reads attributes and declarations
as tokens; it refuses a source it cannot tokenize, and it reads the own file's test modules only
when a crate root reaches that file through declarations kept under `--cfg test`. Criteria A13 and
A15 cover it; this round adds no criterion.

The tests were committed alone at 103e5af0, against the guard as the head left it: the generated
test's new members, drawn from that grammar, and a C string before a spelling in A15's test of a
`//` inside a literal. The generated test asks rustc, at test time, which sources each member
compiles under all eight settings of `test` and two other options, and it fails closed: each rustc
run must exit 1 and count exactly the errors it printed, and every member rustc compiles must fire
its root's probe in every run, so a rustc that compiles nothing is no answer. It fails, and never
skips, when rustc is not on PATH. Both tests are red there by assertion. The rule commit
71173419 edits the guard, which lives in the same file, and leaves the tests' own hunks as
committed at 103e5af0.

```text
A13: red at 103e5af0: Ran 24 tests, FAILED (failures=2): AssertionError: Lists differ: [...] != [] : 397 of 7703 members, the first lib.rs ool crlf=0 '#[cfg(test)]\nmod tests' '# ![cfg(any())]\n': a source rustc does not compile only under test is read (test_every_module_file_choice_is_read_from_rustcs_file_or_refused)
A15: red at 103e5af0: AssertionError: Lists differ: ['demo::Depth (src/depth.rs) "a whole depth"'] != [] : cr#"a" // /* "# (test_a_shape_a_test_spells_after_a_url_on_its_line_is_pinned)
A13: green at 71173419: Ran 24 tests, OK, examined 24 Setting impl(s), examined 6251 R8 member(s) judged against rustc
```

Nineteen rows are re-anchored on the token reader and eight are deleted: four lost their arms, three
became another row's, and S19267 is equivalent on source rustc accepts. Twenty-two rows pin the new
arms (S19278 to S19299), each proved KILLED by full id. The band holds seventy script rows.
S19241's deletion above rested on a claim the fifth review measured false: a string after a `;`
among a macro's arguments can follow the end of an item. The token reader reads a string as one
token, and A13's tests plant that string among a macro's arguments in each delimiter.

## Addendum, 2026-09-30, round 6: five members for the arms a fifth review found unpinned

This round adds no criterion. Five members join the generated test of A13, and each is red by
assertion under one rewrite of the reader: a byte order mark before a shebang line, a declaration
whose attribute leaves its file undecided, a raw `r#path`, a `/***` comment, and a `cfg_attr` that
carries two attributes. The members are committed at 7d31b7a3 with the guard as the head left it,
so the module is green there; the replay below rewrites the reader once per member in a scratch copy
of that commit and runs the generated test.

```text
A13: green at 7d31b7a3: Ran 24 tests, OK, examined 24 Setting impl(s), examined 6309 R8 member(s) judged against rustc
A13: replay of the byte order mark rewrite: FAILED (failures=1): AssertionError: Lists differ: [...] != [] (test_every_module_file_choice_is_read_from_rustcs_file_or_refused)
A13: replay of the undecided declaration rewrite: FAILED (failures=1): AssertionError: Lists differ: [...] != [] (test_every_module_file_choice_is_read_from_rustcs_file_or_refused)
A13: replay of the raw path rewrite: FAILED (failures=1): AssertionError: Lists differ: [...] != [] (test_every_module_file_choice_is_read_from_rustcs_file_or_refused)
A13: replay of the triple star comment rewrite: FAILED (failures=1): AssertionError: Lists differ: [...] != [] (test_every_module_file_choice_is_read_from_rustcs_file_or_refused)
A13: replay of the two attribute cfg_attr rewrite: FAILED (failures=1): AssertionError: Lists differ: [...] != [] (test_every_module_file_choice_is_read_from_rustcs_file_or_refused)
```

## Addendum, 2026-10-01: A16 to A18, the nested module, the macro module and the rival declaration

Three criteria join SPEC-192 (section 13). Their tests were committed first at 87ed53af, with the
guard as the base left it: 38 tests ran and 10 failed, each by assertion and none by an import or a
syntax error (A16 three tests, A17 three, A18 four). The arms were committed at 3ce8e297 and
repaired at 8a44e700, 54a29c9f added a planted positive beside two tests that asserted only
absences, and 33b8fe8b added two assertions on `test_files`. The four commits after the red one
change the guard's own file, test_setting_shapes.py, and no assertion of a red test was weakened in
them. The module is green at 54a29c9f (Ran 38 tests, OK,
examined 25 Setting impl(s), examined 194 crate file(s) read for a macro_rules! body, examined 6309
R8 member(s) judged against rustc); the base module ran 24 tests.

```red-first
A16: red at 87ed53af: AssertionError: Lists differ: ['demo::Depth (src/lib.rs) "a whole depth"'] != [] (test_a_declaration_inside_inline_modules_is_read_from_the_inline_path)
A16: green at 54a29c9f
A17: red at 87ed53af: AssertionError: 0 != 1 :         #[cfg(test)]\n        mod tests;\n (test_a_macro_body_with_a_cfg_test_module_is_refused_by_its_file)
A17: green at 54a29c9f
A18: red at 87ed53af: AssertionError: 0 != 1 : ('lib.rs', '#[cfg(test)]\nmod tests;\n#[cfg(not(test))]\nmod tests;\n') (test_a_module_declared_beside_one_with_not_test_is_refused)
A18: green at 54a29c9f
```

Rows S19305 to S19309 pin the arms, and each is KILLED by its pinning test under one invocation.

A17's red line writes its message's two newlines as `\n`: the message is the planted macro body,
which unittest prints as it is.

## Addendum, 2026-10-01, round 2: a file's own inner attributes, two more macro forms and a rival the guard cannot name

This round adds no criterion: SPEC-192 section 13 widens A16 to A18. Nine new tests were committed
first at f21071a2, with the guard as 33b8fe8b left it: 47 tests ran and 4 failed, each by assertion
and none by an import or a syntax error (A16 one test, A17 one, A18 two). The other five new tests
were green there, and are disclosed as such: each plants a negative that the guard already refused
or a case it already read (a declaration whose file the guard cannot choose, an attribute of another
item or of a macro's matcher, a `cfg_attr` path that does not decide its file alone, a rival in a
`src/bin` root or under a tool attribute, and a plain `#[path]` in another file), and each kills a
mutant of the new arms. The arms were committed at 74807ff7, where the module is green (Ran 47
tests, OK). A second red, 3767d92c, planted a rival whose attributes the guard cannot read back to
the previous item's end (`unsafe mod`): 48 tests ran and 1 failed by assertion, and d336c648 made it
green. No assertion of a red test was weakened after its red commit. The module is green at d336c648
(Ran 48 tests, OK, examined 25 Setting impl(s), examined 194 crate file(s) read for a macro_rules!
body, examined 6309 R8 member(s) judged against rustc).

```text
A16: red at f21071a2: AssertionError: Lists differ: ['demo::Depth (src/lib.rs) "a whole depth"'] != [] (test_a_file_whose_own_inner_attribute_keeps_it_under_test_is_read)
A16: green at 74807ff7
A17: red at f21071a2: AssertionError: 0 != 1 :         mod x {\n            #![cfg(test)]\n        }\n (test_an_inner_attribute_or_an_attribute_before_a_repetition_is_read)
A17: green at 74807ff7
A18: red at f21071a2: AssertionError: Lists differ: ['demo::Depth (src/lib.rs) "a whole depth"'] != [] (test_a_file_only_test_reaches_by_its_own_attribute_or_a_cfg_attr_path_is_read)
A18: red at f21071a2: AssertionError: 0 != 1 : ('lib.rs', '#[path = "tests\\x2ers"]\nmod prod;\n#[cfg(test)]\nmod tests;\n') (test_a_rival_whose_path_the_guard_cannot_read_is_refused)
A18: green at 74807ff7
A18: red at 3767d92c: AssertionError: 0 != 1 : ('lib.rs', '#[cfg(not(test))]\n#[path = "tests.rs"]\nunsafe mod prod;\n') (test_a_rival_whose_attributes_the_guard_cannot_read_whole_is_refused)
A18: green at d336c648
```

Rows S19310 to S19314 pin the round's arms, and each is KILLED by its pinning test under one
invocation, as S19305 to S19309, S19227 and S19277 still are.
