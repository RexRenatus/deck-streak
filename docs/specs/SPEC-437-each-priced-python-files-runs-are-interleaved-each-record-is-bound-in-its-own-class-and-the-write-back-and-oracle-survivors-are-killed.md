# SPEC-437: each priced Python file's runs are interleaved across its legs, each equivalence record is bound only in its own class, and the write-back and parity-oracle survivors are killed

- **Issue:** #822, the release whose run measured the survivors; refers, closes nothing. Refers
  #825 (the Python legs' fit) and #322 (every survivor killed or recorded equivalent).
  **Context(s):** the mutation scripts (`scripts/`), the parity oracle's tests
  (`tools/parity-oracle/`) and `formal/`; no workflow and no crate changes.
- **Decided by:** ADR-451 (the release run's Python survivors are killed by behavioural tests in
  six parts, each file's runs are interleaved, and each equivalence record is bound only in its
  own class).
- **Schematic:** `docs/schematics/mutation-testing-python.md` section 10, added insert-only.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-437.md`. **Mutation band:** S43700-S43799. **Models:**
  `formal/tla/EveryLegCounted`, re-read, and the new Lean entry `lean/RecordClassBinding`. Amends
  SPEC-418 by an insert-only amendment section. The first of six parts: SPEC-438 to SPEC-442 kill
  the survivors of `mutation-verdict.py`, `mutation_rows.py` and `public-scrub.py`.

## 1. The problem, measured

The release pull request's run failed both Python classes. Its verdict job 117095483593 ran 24 s:

- the scripts class read `examined 6317: generated 5944, rows 373`, `survived 684: equivalent 1,
  unexplained 683` and `verdict: FAIL: 686 finding(s)`; three of the 686 are UNCOVERED mutants of
  `scripts/write-back-issues.py:33`, a script the map gives no test (`"modules": []` in
  `scripts/mutation-python.json`);
- the oracle class read `examined 144: generated 143, rows 1`, `survived 20: equivalent 0,
  unexplained 20` and `verdict: FAIL: 21 finding(s)`: 20 survivors of
  `tools/parity-oracle/generate.py` and one STALE record.

Each of these is part 1's to cure, with the run's 24 legs cut at the job's 60-minute bound:

- **The UNCOVERED three.** `33:69` (`group(1)` to `group(2)`) and `33:72` (`in` to `not in`) die
  by any test that writes a known key. `33:95` (`group(0)` to `group(1)`) is in the fallback arm,
  which only a key in `missing` takes, and a non-empty `missing` returns 1 before any write, so no
  test can observe it: the arm is redundant.
- **The 20 oracle survivors.** No behavioural test observes them: `<module>` 1 (`REGISTRY`'s
  directory name), `check` 7 (one per operand of its compound conditions and the `"adapter"` kind),
  `resolve` 1, `plain` 2, `draw` 1, `cases_of` 1 and `main` 7 (two `required=True`, three help
  texts, the bytecode switch and the `sys.path` index). The two `CommittedGoldensAreCurrent` tests
  of `test_goldens.py` compare each golden's `generator_sha256` with `generate.py`'s digest
  (`test_goldens.py:149-168`, `311-324`, `428-452`); the runner drops them from the file's mutant
  runs as byte readers (`scripts/mutation_python.py:516-518`, `:901`), and kept, they would kill
  every mutant by its digest, not its behaviour.
- **The STALE record.** Record 1 of `scripts/mutation-equivalent.d/python.json`
  (`scripts/mutation_python.py:745:73`, #340) is bound and equivalent in the scripts class. The
  oracle class reads it STALE because `judge_python` binds every Python record against that class's
  population (`scope = None`, `scripts/mutation-verdict.py:2155`) and filters by class only after
  binding (`:2167-2169`).
- **The cut legs.** `python_by_cost` cuts each priced file's listing into contiguous runs
  (`scripts/mutation-verdict.py:1235-1241`, `indices[start:end]`). A file's mutant cost is
  correlated by region, since one region's mutants share one traced test set, while its price is
  the file's mean, so a contiguous run puts a costly region whole on one leg. ADR-451's leg table
  gives each cut leg's job, duration, dealt mutants, projection and measured cost. Simulated over
  the run's measured costs, the deal as dealt puts 19 legs past 3600 s; interleaved, at the same
  prices, none, the slowest 3187 s, in the same 305 legs with every projection unchanged.

Adding listed mutants moves the release's Python figure: at the current prices, 12 more in
`mutation-verdict.py` keep the plan at `PYTHON_RELEASE_CEILING` legs and 13 ask one leg more, and
`write-back-issues.py` and `generate.py` keep it with 40 more.

## 2. Requirements

R1. **The interleave.** `python_by_cost` appends each priced file's run as `(path, setup, each,
    indices[cut::cuts])` for `cut` in `range(cuts)`, with `cuts = -(-len(indices) // k)` as at the
    base, and no `divmod`, `start` or `end`. Run j holds the file's listing positions j, j + n,
    j + 2n and so on, n being `cuts` (the SPEC-418 amendment of this delivery reads R3 (c) so).
R2. **The partition and the figures.** Every listed mutant is in exactly one leg. The count of
    legs, each leg's `projected_seconds`, each refusal and every printed line are byte-identical
    to the contiguous deal's; only membership moves. The tests of
    `scripts/tests/test_mutation_python_shard_binding.py` that pin a contiguous membership of a
    file cut into two or more runs (at dev `b25ed04a`, six: `:340`, `:396`, `:578`, `:618`,
    `:2383` and `:2425`) take the interleaved membership in place of it, and keep every count and
    projection literal as written.
R3. **The class binding.** `python_excuses` takes the judged class as a keyword, `klass`, default
    None, and binds only the records whose file `classify` places in that class; with None it binds
    as at the base. `judge_python` passes its class, and its filter of the bound records after
    binding (`:2167-2169`) is removed. A record of another class is neither bound nor read STALE or
    AMBIGUOUS; a record of the judged class that binds nothing reads `STALE <record>: binds no
    python mutant`, as at the base; record 1 stays bound and equivalent in the scripts class.
R4. **The write-back.** `scripts/write-back-issues.py` reads every file, collects every key that
    is not in the manifest, and with any missing prints `write-back-issues: no issue for <sorted
    keys>; nothing written` and returns 1 before it substitutes anything. Only then does it replace
    each token by `#<number>`, with no fallback arm, write each file and print `write-back-issues:
    <n> file(s) written`. Its printed lines and exit codes are byte-identical to the base's.
R5. **The new module.** `scripts/tests/test_write_back_issues.py` loads the script by its path,
    patches `MANIFEST` to a manifest in a temporary directory and writes only there. The map names
    it for `scripts/write-back-issues.py` (`"modules": ["test_write_back_issues"]`), and
    `scripts/tests/test_ci_workflows.py`'s `DYNAMIC_IMPORTS` names its three loader sites
    (`spec_from_file_location`, `module_from_spec` and `exec_module`, one each) in the commit that
    adds it, with any read-census tuple a new read site needs.
R6. **The oracle killers.** `tools/parity-oracle/test_generate.py` gains A9 to A16, and each of
    the 20 survivors dies by one of them. No new test reads `generate.py`'s bytes or asserts a
    digest of it, so none fails under the runner's sentinel; each asserts a whole value, message,
    exit code or file listing by equality.
R7. **The rows.** S43700-S43727 in `scripts/mutation-rows.d/S43700-S43799.json`, section 9: two on
    the interleave, two on the binding, four on the write-back's new lines, and one mirroring each
    of the 20 oracle survivors exactly. Each find occurs exactly once in its file, and every
    existing row on a touched file keeps its find exactly once.
R8. **The figure.** `python3 scripts/mutation_python.py list --file <path>`'s `mutation-python:
    listed <n>` line, read before and after, moves by at most +12 for `scripts/mutation-verdict.py`
    and at most +40 for `scripts/write-back-issues.py`; a larger delta stops the delivery before
    its push.
R9. **Nothing loosens.** `test_goldens.py`, the runner's byte-reader rule, every price table,
    every bound and ceiling, `.github/workflows/` and `scripts/mutation_python.py` are unchanged;
    no reader is removed or loosened, no test is skipped and no mutant leaves the population.
R10. **Formal.** `formal/tla/EveryLegCounted` is re-read for the interleave and its
    `python_by_cost` digest moves (section 8). The new Lean entry `lean/RecordClassBinding` proves
    R3's binding over every class, record list and binding count, with a witness against the
    base's binding, and its vectors are read by A17.

## 3. Acceptance criteria of SPEC-437

| id | criterion | decided by |
|---|---|---|
| A1 | a priced file's runs are interleaved: a.py at 100 s x 30 reads two legs, positions `0, 2, ..., 28` and `1, 3, ..., 29`, each projected 1700 s | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_files_runs_are_interleaved_across_its_legs` |
| A2 | the interleave keeps every count and projection the contiguous deal gives, over literal listings and prices | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_interleave_keeps_every_count_and_projection_of_the_contiguous_cut` |
| A3 | every listed mutant is dealt to exactly one leg, over listings of sizes around each cut boundary | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_every_listed_mutant_is_dealt_exactly_once_when_runs_are_interleaved` |
| A4 | a record of another class is neither bound nor read STALE: the oracle class judged over a whole listing beside a bound scripts record reads no record finding | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_record_of_another_class_is_neither_bound_nor_stale` |
| A5 | a record of the judged class that binds nothing still reads STALE | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_record_of_the_judged_class_that_binds_nothing_stays_stale` |
| A6 | a known key is written as its issue number, and the file count is printed | `python3 -m unittest discover -s scripts/tests -p test_write_back_issues.py -k test_a_known_key_is_written_as_its_issue_number` |
| A7 | an unknown key is named, exit 1, and no file is written, a known key beside it included | `python3 -m unittest discover -s scripts/tests -p test_write_back_issues.py -k test_an_unknown_key_is_named_and_nothing_is_written` |
| A8 | the map runs this module for the write-back script | `python3 -m unittest discover -s scripts/tests -p test_write_back_issues.py -k test_the_map_runs_this_module_for_the_write_back_script` |
| A9 | the default registry names every golden committed under `tools/parity-oracle/goldens/` | `python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_the_default_registry_names_every_committed_golden` |
| A10 | a registration is refused for each operand alone: a bad name, constants names of `[]`, `["a", 1]` and `"abc"`, a function that is not a string, cases that are not callable, an adapter that is not callable, a note of `5` and of blanks | `python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_registration_is_refused_for_each_operand_alone` |
| A11 | a predecessor module that imports a missing dependency raises that dependency's name | `python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_dependency_the_predecessor_lacks_is_raised_by_its_name` |
| A12 | a date inside a dict and inside a list is written as its epoch day | `python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_date_inside_a_dict_or_a_list_is_written_as_its_epoch_day` |
| A13 | a case whose class is the empty name is refused | `python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_case_whose_class_is_the_empty_name_is_refused` |
| A14 | a case input holding NaN is refused before the predecessor's function runs | `python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_case_input_holding_nan_is_refused_before_the_predecessor_runs` |
| A15 | `main` exits 2 without `--source-checkout` and without `--package`, and its help holds each help text, whitespace-flattened | `python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_main_requires_its_checkout_and_package_and_states_each_help_text` |
| A16 | a same-named package ahead on `sys.path` loses to the checkout, and with the bytecode prefix cleared no bytecode lands in the checkout | `python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_the_checkout_is_imported_before_any_other_path_and_gets_no_bytecode` |
| A17 | the record binding agrees with every vector of its Lean entry | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_the_record_binding_agrees_with_every_vector_of_its_proof` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_files_runs_are_interleaved_across_its_legs
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_interleave_keeps_every_count_and_projection_of_the_contiguous_cut
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_every_listed_mutant_is_dealt_exactly_once_when_runs_are_interleaved
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_record_of_another_class_is_neither_bound_nor_stale
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_record_of_the_judged_class_that_binds_nothing_stays_stale
A6: python3 -m unittest discover -s scripts/tests -p test_write_back_issues.py -k test_a_known_key_is_written_as_its_issue_number
A7: python3 -m unittest discover -s scripts/tests -p test_write_back_issues.py -k test_an_unknown_key_is_named_and_nothing_is_written
A8: python3 -m unittest discover -s scripts/tests -p test_write_back_issues.py -k test_the_map_runs_this_module_for_the_write_back_script
A9: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_the_default_registry_names_every_committed_golden
A10: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_registration_is_refused_for_each_operand_alone
A11: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_dependency_the_predecessor_lacks_is_raised_by_its_name
A12: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_date_inside_a_dict_or_a_list_is_written_as_its_epoch_day
A13: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_case_whose_class_is_the_empty_name_is_refused
A14: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_case_input_holding_nan_is_refused_before_the_predecessor_runs
A15: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_main_requires_its_checkout_and_package_and_states_each_help_text
A16: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_the_checkout_is_imported_before_any_other_path_and_gets_no_bytecode
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_the_record_binding_agrees_with_every_vector_of_its_proof
```

A1, A4, A8 and A17 are red first: the base deals contiguous runs, binds a scripts record in the
oracle class, maps no module for the write-back script, and reads a foreign-class record STALE where
the proof's vectors read it unjudged. Each red is its criterion's assertion, never a missing name, a
missing fixture or an empty selection. The other thirteen pin behaviour the base already has and
are recorded `not red`, each with its reason: each is the killer of its rows in section 9, and its
proof is those rows read KILLED. A10 and A15 each enumerate their cases, print the count they
examined and refuse zero, and so do A2, A3 and A17.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation-verdict.py` | mutation scripts | changed: `python_by_cost`'s runs interleaved (R1); `python_excuses`' `klass` and `judge_python`'s call, its filter after binding removed (R3) |
| `scripts/write-back-issues.py` | scripts | changed: unknown keys refused before any substitution, the fallback arm removed (R4) |
| `scripts/mutation-python.json` | mutation scripts | changed: `test_write_back_issues` mapped for `scripts/write-back-issues.py` (R5) |
| `scripts/tests/test_write_back_issues.py` | tests | added: A6 to A8 |
| `scripts/tests/test_mutation_python_shard_binding.py` | tests | changed: A1 to A3, and six contiguous memberships moved to the interleave with every count and projection kept (R2) |
| `scripts/tests/test_mutation_verdict_python_kills.py` | tests | changed: A4, A5 and A17 |
| `scripts/tests/test_ci_workflows.py` | tests | changed: three `DYNAMIC_IMPORTS` entries for `test_write_back_issues`, and any read-census tuple a new read site needs |
| `tools/parity-oracle/test_generate.py` | tests | changed: A9 to A16 |
| `scripts/mutation-rows.d/S43700-S43799.json` | mutation rows | added: S43700-S43727 |
| `formal/tla/EveryLegCounted/EveryLegCounted.tla` | formal | changed: comment lines and the `python_by_cost` digest |
| `formal/lean/Formal/RecordClassBinding.lean` | formal | added: the entry, its port, its theorem and its witness |
| `formal/lean/Formal/RecordClassBindingVectors.lean` | formal | added: the entry's vector writer |
| `formal/lean/Formal/Vectors.lean` | formal | changed: one import and one arm |
| `formal/vectors/record-class-binding.jsonl` | formal | added: written by the writer, never by hand |
| `docs/specs/SPEC-437-each-priced-python-files-runs-are-interleaved-each-record-is-bound-in-its-own-class-and-the-write-back-and-oracle-survivors-are-killed.md` | records | added: this SPEC |
| `docs/decisions/ADR-451-the-release-runs-python-survivors-are-killed-in-six-parts-each-files-runs-are-interleaved-and-each-record-is-bound-in-its-own-class.md` | records | added: its decision |
| `docs/red-first/SPEC-437.md` | records | added: the red-first record |
| `docs/specs/SPEC-418-the-python-mutation-legs-are-dealt-by-each-files-measured-cost-every-listed-mutant-examined-or-the-plan-refused-whole.md` | records | changed: an insert-only amendment section |
| `docs/schematics/mutation-testing-python.md` | records | changed: section 10, insert-only |
| `changelog.d/python-kills-437.md` | records | added: the changelog fragment |

## 5. What this does NOT cover

- It kills no survivor of `mutation-verdict.py`, `mutation_rows.py` or `public-scrub.py`; parts 2
  to 6, SPEC-438 to SPEC-442, do, each landing before the release's closing run (#822).
- It records no equivalent mutant; each of the two records lands with its part (#322).
- It changes no price and refreshes no price table (#825).
- It raises no job's bound or timeout and changes no workflow (#825).
- It does not measure the 90 dealt mutants the cut legs never reached, nor leg 119's 13; the
  release's closing run does (#822).
- It does not change the Rust legs (#822).
- The lint rules land in their own deliveries (#901, #913).

## 6. Risks

- **Prices below the new cost.** `write-back-issues.py` is priced (1, 1, 1) and `generate.py`
  (2, 2, 1), and both gain tests that run for their mutants. Their legs run longer than projected;
  the interleaved deal's slowest simulated leg, 3187 s, leaves the margin, and each leg's
  `costs.jsonl` measures the change for #825's refresh.
- **A contiguous pin outside the census.** A test this SPEC does not name that pins a contiguous
  membership reads red at the green push; it takes the interleaved membership by R2's rule, every
  figure kept, and is disclosed.
- **A killer the runner drops.** A new test that fails under the sentinel is a byte reader and kills
  nothing; the verdict's `byte reader:` line naming a new test of `generate.py` is a red to
  diagnose.
- **A port that drifts.** The Lean entry covers `python_excuses`, `Excuses` and `judge_python`, so
  an edit to any of them reads `STALE` until the port is re-read; A17 holds the code to the port's
  vectors.

## 7. What only CI proves

- Every red and green of the fence, read in the `hygiene` job, which runs `scripts/tests` and the
  parity oracle's tests; the test modules run only in CI, and every reading is recorded in
  `docs/red-first/SPEC-437.md`.
- Each row of section 9 read KILLED by id in the `mutation-rows` job, and the mutants of this
  delivery's changed lines in its Python legs.
- That the interleave fits every leg within its job, which the release's closing run reads, with
  zero unexplained survivors in every Python class.
- The formal entries are checked by hand: no CI job checks `formal/`.

## 8. Formal model

**TLA+.** `formal/tla/EveryLegCounted` covers `python_by_cost` and already admits any deal: its
`Plan` chooses `dealt'` freely over the legs, and every listed mutant is in exactly one run. The
interleave changes which run holds a mutant and nothing the model reads, so the model is re-read,
`\*` comment lines record the re-read, and the `python_by_cost` digest moves by the digest rule,
computed at the base, where it must equal its recorded value, and at the head. No other cover moves.

**Lean.** `formal/lean/Formal/RecordClassBinding.lean` covers `scripts/mutation-verdict.py`
anchors `python_excuses`, `Excuses` and `judge_python`, and ports the binding branch for branch:
the records kept for a class, then each judged by how many mutants it binds, one valid, none
STALE, two or more AMBIGUOUS. Its claim, one `Prop` over every class, record list, classification
and binding count: every record judged is of the judged class, and every record of that class is
judged exactly once by its count. The theorem `each_record_is_judged_in_its_own_class_alone` enters
at `ramp=report`. Its witness `old_violates` ports the base's binding, which keeps every record and
filters by class only after binding, at a scripts record binding nothing under the oracle class,
the run's reading. Its vectors, `formal/vectors/record-class-binding.jsonl`, are written by
`Formal/RecordClassBindingVectors.lean` through one arm of `Formal/Vectors.lean`, and A17 holds
`python_excuses` to each.

## 9. Mutation rows

| row | behaviour | mutant | killer |
|---|---|---|---|
| S43700 | a file's runs are interleaved | the interleave back to contiguous slices | A1 |
| S43701 | every run of a file is dealt | the loop over runs starting at its second run | A3 |
| S43702 | a record is bound only in its own class | the class test `==` to `!=` | A5 |
| S43703 | `judge_python` judges in its own class | its class argument to None | A4 |
| S43704 | an unknown key refuses before any write | the refusal's condition negated | A7 |
| S43705 | a key the manifest lacks is collected | `not in issues` to `in issues` | A7 |
| S43706 | a known key's token is written | the token's group 1 to group 2 | A6 |
| S43707 | a token is written as `#<number>` | the `#` dropped | A6 |
| S43708 | the default registry is the registry directory | `"registry"` to `""` | A9 |
| S43709 | a bad name string is refused | `or` to `and` at `generate.py:118` | A10 |
| S43710 | an empty names list is refused | `or` to `and` at `:136` | A10 |
| S43711 | a names list holding a non-string is refused | `or` to `and` at `:137` | A10 |
| S43712 | a function and cases are each checked | `or` to `and` at `:141` | A10 |
| S43713 | the adapter kind is checked | `"adapter"` to `""` at `:147` | A10 |
| S43714 | the adapter is checked beside its note | `and` to `or` at `:149` | A10 |
| S43715 | a blank note is refused | `and` to `or` at `:150` | A10 |
| S43716 | a missing dependency is raised | `or` to `and` at `:164` | A11 |
| S43717 | a dict's values are made plain | the dict's return to None at `:185` | A12 |
| S43718 | a list's items are made plain | the list's return to None at `:187` | A12 |
| S43719 | an empty case class is refused | `and` to `or` at `:203` | A13 |
| S43720 | a NaN input is refused | `allow_nan=False` to `True` at `:223` | A14 |
| S43721 | the checkout flag is required | its `required=True` to `False` at `:276` | A15 |
| S43722 | the checkout flag states its help | its help text to `""` at `:276` | A15 |
| S43723 | the package flag is required | its `required=True` to `False` at `:280` | A15 |
| S43724 | the package flag states its help | its help text to `""` at `:281` | A15 |
| S43725 | the registry flag states its help | its help text to `""` at `:286` | A15 |
| S43726 | no bytecode is written into the checkout | the bytecode switch's `True` to `False` at `:305` | A16 |
| S43727 | the checkout is first on the path | the `sys.path` index 0 to 1 at `:306` | A16 |

Each row's find is written from the landed line, unique in its file and byte-identical to the text
the mutant replaces; each `generate.py` row installs exactly the survivor's mutant at the line and
column the run named. Each killer is its criterion's test, run by CI's `mutation-rows` job.
