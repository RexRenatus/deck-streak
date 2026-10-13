# SPEC-406: a loader-style test module the census register lacks is refused before the push

- **Wave:** the test-module census (SPEC-190 R12 part 1). **Issue:** #789 (a loader-style
  `scripts/tests` module the `DYNAMIC_IMPORTS` register lacks is caught only in CI).
  **Context(s):** `repo` (`scripts/`, `scripts/tests/`, `docs/`).
- **Decided by:** ADR-420 (this SPEC's own: the population, the check, the template rule, FORMAL,
  the push) and ADR-292 (the census and its register).
- **Schematic:** `docs/schematics/ci-jobs-and-caches.md`: its new last section, the dynamic-import
  register before the push and in the `hygiene` job.
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-406.md`. **Mutation
  band:** S40600-S40699, unused (section 7).

## 1. The problem, measured

Read at `dev` 32f6217.

- The census that SPEC-190 R12 part 1 built (its section 12, lines 292-302; A12 at line 349)
  refuses every site in `scripts/tests` that imports or runs code by a name held in data unless the
  `DYNAMIC_IMPORTS` register in `scripts/tests/test_ci_workflows.py` names it by module, qualified
  name and text, with its exact count. The register opens at line 6482
  (`git grep -n '^DYNAMIC_IMPORTS = {' 32f6217 -- scripts/tests/test_ci_workflows.py`) and closes at
  7064; it holds 43 groups (`git show 32f6217:scripts/tests/test_ci_workflows.py | sed -n
  '6482,7064p' | grep -c '^    \*\*allowed('`) and no literal key, and a syntax-tree read of it
  gives 132 site tuples naming 33 modules.
- The census runs only in CI: the `hygiene` job (`.github/workflows/ci.yml:298`) runs `bash
  scripts/check.sh python scrub secrets` (lines 347-351), whose `stage_python`
  (`scripts/check.sh:179-195`) runs `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`.
  Nothing before the push reads the register.
- Twenty modules under `scripts/tests` load code through `importlib`, `runpy` or `exec` by a
  syntax-tree read of each module's text, nothing imported, and the register names all twenty.
- A text search for the loader's words finds four more modules, each holding them only in a string
  constant: `test_mutation_rows.py:43-50`, `test_host_scrub.py:109-130`,
  `test_deploy_scripts.py:1495` and 1512, and `test_stand_in_census.py:331`. The register names
  neither of the last two, and the census admits all four, because a string is no site.
- The cost is recorded twice: SPEC-352's first push read the census test red in CI
  (`docs/red-first/SPEC-352.md:78-80`), and SPEC-347 moved two sites into `DYNAMIC_IMPORTS` after
  its manifest had named `NOT_WORKFLOW_READS` (`docs/red-first/SPEC-347.md:77-79`).
- The SPEC template's file manifest section (`docs/specs/_TEMPLATE.md:38-44`) asks for every file a
  delivery adds or changes and names no register (`git grep -n DYNAMIC_IMPORTS 32f6217 --
  docs/specs/_TEMPLATE.md` prints nothing).

## 2. Requirements

- R1. `scripts/dynamic-imports-check.py`, standard library only, reads the `.py` paths that `git
  diff --name-only --no-renames --diff-filter=AM <base>...HEAD -- scripts/tests` names, each by `git
  show HEAD:<path>`, and the register by `git show HEAD:scripts/tests/test_ci_workflows.py`. It
  reads no work-tree file, and it imports and runs no module it judges.
- R2. A module is loader-style when its syntax tree holds a load of a name that an import of
  `importlib` or `runpy` binds (any submodule, any alias, the `from` form included), or a load of
  the bare name `exec` that the module does not bind. An unused import, a string, a comment and a
  star import make no module loader-style.
- R3. The register is read from its syntax tree: exactly one module-level assignment to
  `DYNAMIC_IMPORTS`, a dict display whose every entry is `**allowed(reason, site, ...)` where each
  site is a tuple whose first element is a string, the module. The register's modules are those
  first elements. Any other shape, a second assignment, no assignment, or no module stops the run.
- R4. The check prints `dynamic-imports: register: N module(s) in DYNAMIC_IMPORTS at HEAD`, then
  `dynamic-imports: examined M changed scripts/tests module(s), L loader-style`, then for each
  loader-style module the register does not name `dynamic-imports: REFUSED: <path>: loads code by
  <importlib|runpy|exec> at line <n>; DYNAMIC_IMPORTS lists no site of <module>`, and a last line:
  `dynamic-imports: OK` (exit 0), `dynamic-imports: REFUSED` (exit 1), `dynamic-imports:
  NOT-APPLICABLE` when no module changed (exit 0, after the register line), or `dynamic-imports:
  VOID: <cause>` (exit 2) when the register, a changed module, the register file or the base
  cannot be read.
- R5. `--root` defaults to the repository that holds the script, and `--base` to `origin/dev`.
- R6. A module is named by its dotted path relative to `scripts/tests`, an `__init__` naming its
  package, as the census's `module_sources()` names it (`test_ci_workflows.py:4510-4518`).
- R7. The SPEC template's section "4. File manifest" gains, after "Every file the delivery adds or
  changes. A file outside this list needs an amendment first.", this paragraph, word for word:
  "A delivery that adds a module under `scripts/tests` that loads code through `importlib`, `runpy`
  or `exec`, or adds such a load to a module there, lists `scripts/tests/test_ci_workflows.py` in
  this table as changed: its `DYNAMIC_IMPORTS` register names each such site by its module,
  qualified name and text, with its count, and the census in the python stage refuses a site the
  register does not name. Before the push, `python3 scripts/dynamic-imports-check.py --base <the
  pull request's base>` refuses a new or changed module there that loads code and that the register
  does not name."
- R8. `DYNAMIC_IMPORTS` names the new test module's three sites in its `load` function, in the
  group whose reason is "loads a production script by the path the call names; never a module of
  the test directory", after the `test_backup_units` sites: `importlib.util.module_from_spec(spec)`,
  `importlib.util.spec_from_file_location('dynamic_imports_check', SCRIPT)` and
  `spec.loader.exec_module(module)`, each with count 1. Nothing else in `test_ci_workflows.py`
  changes.
- R9. `scripts/mutation-python.json` maps `scripts/dynamic-imports-check.py` to the test directory
  and the module `test_dynamic_imports_check`, so the python mutation runner judges its mutants.
- R10. `docs/schematics/ci-jobs-and-caches.md` gains a last section, insert-only, drawing a new
  module from its commit through the check and the census.
- R11. The test module walks no directory. It reads the planted repositories it builds, its own
  source, the SPEC template, and `scripts/tests/test_ci_workflows.py` by parsing its text, never by
  importing it; every name it uses is one the census admits, `load` aside.

## 3. Acceptance criteria of SPEC-406

| id | criterion | decided by |
|---|---|---|
| A1 | run as a program over a planted repository whose head adds a loader module the register does not name, the check exits 1 and prints the REFUSED line naming the path, the loader, the line and `DYNAMIC_IMPORTS` | `scripts/tests/test_dynamic_imports_check.py` `test_a_planted_loader_module_missing_from_the_register_is_refused` |
| A2 | the same planted module, with its name in the register the head commits, passes: exit 0, `examined 1 changed scripts/tests module(s), 1 loader-style` and `OK` | `scripts/tests/test_dynamic_imports_check.py` `test_the_same_planted_module_registered_passes` |
| A3 | every spelling in an enumerated table is judged as the table says: each loader spelling found with its loader and line, and each non-loader (a string, a comment, an unused import, a module that binds `exec`, a process whose argument text says `exec`) not found; the test prints how many spellings it examined | `scripts/tests/test_dynamic_imports_check.py` `test_every_loader_spelling_is_found_and_loader_text_in_a_string_is_not` |
| A4 | the real register parses and names this module, `_support`, `test_backup_units`, `test_threat_model` and `test_audit_web`; the check finds this module's two `importlib` loads and admits it; the census's `VETTED_MODULES` names neither `importlib` nor `runpy` and its `BARE_DYNAMIC` names `exec`; the default root is the repository `_support.REPO` names | `scripts/tests/test_dynamic_imports_check.py` `test_the_real_register_lists_this_module_and_the_census_counts_what_the_check_reads` |
| A5 | a register or a module the check cannot read stops the run: no assignment, two assignments, a value that is not a dict display, a literal key, an entry `allowed` does not build, a module that is not a string, an empty register, a changed module that does not parse, a missing register file and a base that names no commit each exit 2 with a VOID line naming the cause | `scripts/tests/test_dynamic_imports_check.py` `test_a_register_or_a_module_the_check_cannot_read_stops_the_run` |
| A6 | the check judges what the head commits, by the head's names: a register entry left uncommitted admits nothing; a registered loader renamed is judged under its new name and refused; a deleted module is not read | `scripts/tests/test_dynamic_imports_check.py` `test_the_check_judges_what_head_commits_by_the_names_head_gives` |
| A7 | a head that changes no `scripts/tests` module reads NOT-APPLICABLE with exit 0 and still prints the register line; with no `--base` the check diffs against `origin/dev` | `scripts/tests/test_dynamic_imports_check.py` `test_a_push_that_changes_no_test_module_is_not_applicable_and_still_reads_the_register` |
| A8 | the SPEC template's file manifest section names `scripts/tests/test_ci_workflows.py`, `DYNAMIC_IMPORTS` and `scripts/dynamic-imports-check.py` | `scripts/tests/test_dynamic_imports_check.py` `test_the_spec_template_manifest_section_names_the_census_module_and_the_check` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_a_planted_loader_module_missing_from_the_register_is_refused
A2: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_the_same_planted_module_registered_passes
A3: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_every_loader_spelling_is_found_and_loader_text_in_a_string_is_not
A4: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_the_real_register_lists_this_module_and_the_census_counts_what_the_check_reads
A5: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_a_register_or_a_module_the_check_cannot_read_stops_the_run
A6: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_the_check_judges_what_head_commits_by_the_names_head_gives
A7: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_a_push_that_changes_no_test_module_is_not_applicable_and_still_reads_the_register
A8: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_the_spec_template_manifest_section_names_the_census_module_and_the_check
```

The census's own verdict on the three new register lines is read in the `hygiene` check-run, by its
existing test `test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read`,
and the schematic's new diagram is parsed in the `web` check-run by the docs diagram test SPEC-195
built; no criterion of this SPEC is decided there.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/dynamic-imports-check.py` | `repo` | added: the check (R1 to R6) |
| `scripts/tests/test_dynamic_imports_check.py` | `repo` | added: A1 to A8 (R11) |
| `scripts/tests/test_ci_workflows.py` | `repo` | changed: `DYNAMIC_IMPORTS` names the new module's three sites (R8) |
| `scripts/mutation-python.json` | `repo` | changed: the script's entry (R9) |
| `docs/specs/_TEMPLATE.md` | `repo` | changed: section 4's paragraph (R7) |
| `docs/schematics/ci-jobs-and-caches.md` | `repo` | changed: a new last section, insert-only (R10) |
| `docs/specs/SPEC-406-a-loader-style-test-module-the-census-register-lacks-is-refused-before-the-push.md` | `repo` | added: this SPEC |
| `docs/decisions/ADR-420-each-changed-test-module-that-loads-code-is-held-to-the-register-before-the-push-by-its-syntax-tree.md` | `repo` | added: its ADR |
| `docs/red-first/SPEC-406.md` | `repo` | added: the red-first record |
| `changelog.d/dynamic-imports-register-406.md` | `repo` | added: the fragment |

## 5. What this does NOT cover

- It does not judge a registered module that gains a new site, nor any tuple's text or count; the
  census in `hygiene` judges those, as SPEC-190 R12 part 1 built it (#377).
- It does not read dynamic names beyond `importlib`, `runpy` and `exec` (`__import__`, `eval`,
  `compile`, `getattr` by a name held in data, `sys.modules`, a loader member reached through an
  object another module made); the census counts each (#377).
- It does not follow a star import from `importlib` or `runpy`, which binds no name; the census
  refuses the use of a name bound nowhere (#377).
- It does not read `NOT_WORKFLOW_READS`, or any read the census counts in its population (#377).
- It does not change the census, its planted controls or any row anchored in
  `test_ci_workflows.py` (#377).
- It adds no git hook, no CI job and no stage to `scripts/check.sh`; CI keeps the census as its
  judge, and the check is run before the push (#789).
- It does not change `docs/TESTING.md` or the repository's contributor instructions; the template
  is the rule's one home (#789).

## 6. Risks

- **A registered module gains a new loader site.** The check passes it, by design. Detected by the
  census in `hygiene`, naming the site; the template's paragraph tells the author to list the
  census module whenever a load is added.
- **The census vets `importlib` or `runpy`, or drops `exec` from `BARE_DYNAMIC`.** The check would
  then refuse a module the census admits. Detected by A4, which reads both lists from the census
  module in `hygiene` on the delivery that changes them.
- **The register takes another shape** (a literal key, or a helper other than `allowed`). The check
  reads VOID on every run. Detected by A4, which reads the real register in `hygiene`.
- **A name an `importlib` or `runpy` import binds is reused for another value in another scope.**
  The check counts the name module-wide, so an unregistered module could read REFUSED where the
  census reads no site. Detected by the REFUSED line, which names the line; no module at `dev` has
  this shape.
- **The base ref is absent where the check runs** (`origin/dev` not fetched). The check reads VOID
  with git's message and exit 2, never OK; the author passes `--base`.

## 7. Mutation rows

No row is added: `scripts/dynamic-imports-check.py` is in the python mutation runner's population
(`scripts/mutation_python.py` `population()`, lines 374-383), and R9's map entry has the
`mutation-python (<shard>)` legs judge its generated mutants by `test_dynamic_imports_check`. The
template, schematic, SPEC and ADR text carries no behaviour.

Eleven rows anchor in `scripts/tests/test_ci_workflows.py`: S19057, S19058, S19059 and S19060
(`scripts/mutation-rows.d/S19000-S19099.json`), S19105, S34410, S34411, S34412, S34717, S34848 and
S34849. Their finds are text, and the three register lines are inserted inside `DYNAMIC_IMPORTS`
with no find among them, so no anchor moves; each find still occurs exactly once, and the
`mutation-rows` check-run re-kills them.
