# SPEC-492: the scripts class's 813 release survivors are each killed, recorded or cured by a behaviour-neutral edit

**Decided by:** ADR-510. **Red-first record:** `docs/red-first/SPEC-492.md`. **Mutation band:** `S49200-S49299`
(section 4). **Models:** none new; two covered spans are re-read and re-stamped (section 7).

## 1. The problem, measured

The release pull request's mutation run 39182685247 read the scripts class in job 118006842234 (`mutation-verdict`) as
`verdict: FAIL: 813 finding(s)`: 813 survivors, each a `SURVIVED` line of that job's log, 690 in `scripts/mutation-verdict.py`,
107 in `scripts/mutation_rows.py` and 16 in `scripts/public-scrub.py`. Read against the scripts pipeline's own rules, each is one
of three:

- 792 are killed by a test: 154 by tests `scripts/tests/test_mutation_workflows.py` already holds, a module the population map
  gives no file, and 638 by the tests this specification adds.
- 1 is equivalent on a file whose mapped suite runs no record census: `scripts/public-scrub.py:118`, where a default flags text of
  `""` and one of `"XX"` both hold no `i`.
- 20 are equivalent on a file whose mapped suite runs the record census, 14 in `scripts/mutation-verdict.py` and 6 in
  `scripts/mutation_rows.py`. A record there cannot hold: its anchor holds the mutant's own token, so the mutant edits the record's
  anchor out and the census refutes it. Each site is removed by an edit that changes no output, exit or file.

A pull request into dev is judged on its diff (`scope_of` in `scripts/mutation-verdict.py`), and its plan lists each file's added
code lines alone, so this pull request's own mutation legs examine the survivors on the lines it changes; the release run reads
the rest.

## 2. Requirements

R1. `scripts/mutation-python.json` names `test_mutation_workflows` and `test_mutation_verdict_survivor_kills` for
`scripts/mutation-verdict.py`, and `test_mutation_rows_survivor_kills` for `scripts/mutation_rows.py`, each once, beside the
modules it names now.

R2. Each of the 792 survivors classed killed has a test, in a module the map gives its file, that fails on its mutant by an
assertion on the value the mutant changes, with the expected value spelled literally.

R3. `scripts/mutation-equivalent.d/python.json` holds one more record, for `scripts/public-scrub.py`'s flags default: its anchor
occurs once in that file and binds that one mutant, with its reason, evidence, issue and the test that reaches it.

R4. The 20 equivalent sites are removed by edits that change no output, exit or file: the five `find` results compared `< 0`,
where the find cannot return 0, compare `== -1`; the two `zip(..., strict=True)` over lists built in lockstep iterate one list;
the record check's placeholder row gives way to a resolver that takes the crate and the killer alone; the empty subject is no
longer searched; the four maxsplit reads of `split` and `rsplit` use `partition` and `rpartition`; and the two temporary
directories take no name prefix.

R5. `scripts/mutation_rows.py` resolves a killer from a crate cell and a killer cell alone (`locate_test`, `resolve_test`);
`locate_killer` and `resolve_killer` hand a row's two cells to them unchanged, and a Rust record's `reached_by` is resolved
through `resolve_test` in its record's crate.

R6. Every mutation row whose find an edit of R4 or R5 rewrites is re-anchored on the new text with its mutant, description and
killer kept, and every formal cover whose span an edit moves is re-read and carries its new digest.

R7. No bound, price, timeout, workflow, test filter or record on a census-held file changes.

## 3. Acceptance criteria

| criterion | holds | decided by |
|---|---|---|
| A1 | the map names each kill module once for its own file (R1) | `test_mutation_verdict_survivor_kills.TheMapRunsTheKillers.test_the_map_names_each_kill_module_once_for_its_file` |
| A2 | the flags default is excused by one whole record (R3) | `test_public_scrub.TheScrubOwnsItsShapes.test_the_flags_default_is_excused_by_one_whole_record` |
| A3 | the verdict script keeps none of the equivalent shapes R4 names (R4) | `test_mutation_verdict_survivor_kills.TheEquivalentShapesAreGone.test_the_verdict_keeps_none_of_its_equivalent_shapes` |
| A4 | the rows runner keeps none of the equivalent shapes R4 names (R4) | `test_mutation_rows_survivor_kills.TheEquivalentShapesAreGone.test_the_rows_runner_keeps_none_of_its_equivalent_shapes` |
| A5 | the narrow resolvers answer as the row resolvers do over the same cells (R5) | `test_mutation_rows_survivor_kills.TheResolveTest.test_resolve_test_answers_as_resolve_killer_over_the_same_cells` |
| A6 | the verdict script's kill tests pass (R2) | `test_mutation_verdict_survivor_kills` |
| A7 | the rows runner's kill tests pass (R2) | `test_mutation_rows_survivor_kills` |
| A8 | the configuration and exclusion checks' tests pass (R2) | `test_mutation_workflows` |
| A9 | the scrub's tests pass (R2, R3) | `test_public_scrub` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_survivor_kills.py -k test_the_map_names_each_kill_module_once_for_its_file
A2: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_the_flags_default_is_excused_by_one_whole_record
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_survivor_kills.py -k test_the_verdict_keeps_none_of_its_equivalent_shapes
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_survivor_kills.py -k test_the_rows_runner_keeps_none_of_its_equivalent_shapes
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_survivor_kills.py -k test_resolve_test_answers_as_resolve_killer_over_the_same_cells
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_survivor_kills.py
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_survivor_kills.py
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py
A9: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-492-the-scripts-classs-release-survivors-are-killed-recorded-or-cured-by-edit.md` | docs | this specification. |
| `docs/decisions/ADR-510-equivalent-survivors-on-a-census-held-file-are-cured-by-behaviour-neutral-edits.md` | docs | its decision. |
| `docs/red-first/SPEC-492.md` | docs | the red-first record. |
| `scripts/mutation-python.json` | scripts | R1's three module names. |
| `scripts/mutation-verdict.py` | scripts | R4's 14 sites and R5's call. |
| `scripts/mutation_rows.py` | scripts | R4's 6 sites, `locate_test` and `resolve_test`. |
| `scripts/mutation-equivalent.d/python.json` | scripts | R3's record. |
| `scripts/tests/test_mutation_verdict_survivor_kills.py` | tests | new, the verdict script's kills, A1, A3 and two rows' killers. |
| `scripts/tests/test_mutation_rows_survivor_kills.py` | tests | new, the rows runner's kills, A4, A5 and two rows' killers. |
| `scripts/tests/test_mutation_workflows.py` | tests | the configuration and exclusion checks' new kills. |
| `scripts/tests/test_public_scrub.py` | tests | the scrub's new kills and A2. |
| `scripts/tests/test_ci_workflows.py` | tests | `DYNAMIC_IMPORTS` and `NOT_WORKFLOW_READS` name each import site and each file-open site that the new and edited test modules add. |
| `scripts/mutation-rows.d/S49200-S49299.json` | rows | new, the band's four rows. |
| `scripts/mutation-rows.d/S03900-S03999.json` | rows | S03951 re-anchored on `locate_test`'s crate cell. |
| `scripts/mutation-rows.d/S05700-S05799.json` | rows | S05705 re-anchored on `resolve_test`'s call. |
| `formal/tla/EveryLegCounted/EveryLegCounted.tla` | formal | the `whole_reports` re-examination and its cover's digest. |
| `formal/lean/Formal/TokenProgress.lean` | formal | the `rust_tokens` cover's digest. |
| `formal/vectors/token-progress.jsonl` | formal | the vectors, regenerated by the formal checker for the moved digest. |
| `changelog.d/scripts-survivors-822.md` | docs | the changelog fragment. |

## 5. What this does NOT do

- It triages no Rust survivor and no VOID Rust shard of the same run (#993).
- It changes no price, bound, timeout, workflow or test filter, so the price tables stay as their recorded runs read them while
  each mapped suite grows (#993).
- It leaves `python_reached`'s placeholder row as it stands: a test pins each of its fields, and none of its mutants survived
  (#993).
- Its own mutation legs examine only the lines it changes; the survivors on the lines it leaves unchanged are read by the release
  run (#993).

## 6. Risks

- An edit of R4 or R5 that is not behaviour-neutral: the tests the map already gives each file, and the kill modules, run over it
  in CI, and the release run reads every survivor again.
- A killer that reads its expected value from the code under test or from the mutated constant kills nothing: each assertion
  spells its expected value literally, and the mutation legs read each kill by name.
- The mapped suites grow while the price tables keep their recorded figures, so a release leg can run past its bound: the release
  run's legs line and each leg's VOID name it, and a re-price is a recorded run's output, never an edit here.
- A re-anchored row on the wrong text: the anchor census reads each find once, and the rows legs read each row by id.

## 7. Decided outside the repository's tests

- Each survivor's kill is read from the mutation legs, by mutant name: this pull request's legs for the lines it changes, and the
  release run's for the rest.
- `tla/EveryLegCounted` and `lean/TokenProgress` are re-read against the moved spans of `whole_reports` and `rust_tokens`; neither
  model nor port changes, and the formal checker reads each new digest and regenerates the vectors.
