---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Each changed test module that loads code is held to the register before the push, by its syntax tree

## Context and Problem Statement

The modules under `scripts/tests` are read by a census that SPEC-190 R12 part 1 built (its section
12, lines 292-302; A12 at line 349; ADR-292; #377). Every site in the test directory that imports
or runs code by a name held in data is listed in the `DYNAMIC_IMPORTS` register of
`scripts/tests/test_ci_workflows.py` by its module, its qualified name and its text, with its count
and its reason, and any other site is refused. The census runs in one place, the `hygiene` job's
python stage. So a delivery that adds a module that loads a script through `importlib`, `runpy` or
`exec`, and does not add that module's sites to the register, learns of it only after its push.

Two records show the cost. SPEC-352's first push read the census test red in CI
(`docs/red-first/SPEC-352.md:78-80`), and SPEC-347 moved two sites into `DYNAMIC_IMPORTS` after
its manifest had named `NOT_WORKFLOW_READS` (`docs/red-first/SPEC-347.md:77-79`). SPEC-375 shows
the shape that works: its manifest lists the census module as changed because the register names
the new module's sites. #789 asks for two halves, both on `dev`: a rule a SPEC's author reads,
and a check that runs before the push.

## Decision Drivers

- The census stays the judge. It is default-deny, and each site it admits carries a reason and an
  exact count (SPEC-190 R12 part 1); nothing here loosens it.
- The check runs where the author runs it, before the push, on the commit the push carries.
- The check never refuses a module the census admits, so it can never block a correct push.
- It reads text and imports or runs no test module, with the standard library only.
- Every count it reports is a count of what it examined, and a run that could not read its inputs
  is VOID, never OK.

## The population, measured

Read at `dev` 32f6217 by `git show`, `git grep -n` and a syntax-tree read (no module imported):

| what | where | measured |
|---|---|---|
| the register | `scripts/tests/test_ci_workflows.py:6482-7064` | 43 `**allowed(...)` groups, no literal key; 132 site tuples naming 33 modules |
| how a group is built | `allowed()`, `test_ci_workflows.py:4779-4782` | `{(module, qualified name, text): (count, reason)}` |
| how a module is named | `module_sources()`, `test_ci_workflows.py:4510-4518` | every `*.py` under `scripts/tests` at any depth, by its dotted name there; `__init__` names its package |
| what the census sums | `census_problems()`, `test_ci_workflows.py:4546`; line 4765 | `NOT_WORKFLOW_READS` and `DYNAMIC_IMPORTS`, one count per key |
| why `importlib` and `runpy` count | `VETTED_MODULES`, `test_ci_workflows.py:4149` | neither is vetted, so every reference through either is a site |
| why `exec` counts | `BARE_DYNAMIC`, `test_ci_workflows.py:4069-4080` | `exec` is named; a name bound nowhere and in no list is red |
| the loader member names | `DYNAMIC_NAMES`, `test_ci_workflows.py:4014` | `exec_module`, `module_from_spec`, `spec_from_file_location`, `run_path`, `run_module`, `import_module` among them |
| the test that judges | `test_ci_workflows.py:7489`, 7570 | `WorkflowFilesAreReadAsBytes.test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read` |
| its planted controls | `test_ci_workflows.py:7366`, 7373, 7382, 7411; run at 7646 | an unlisted `spec_from_file_location`, `runpy.run_path`, `exec` of a string and `importlib.import_module` are each refused |
| where it runs | `.github/workflows/ci.yml:298`, 347-351; `scripts/check.sh:179-195` | the `hygiene` job's step `bash scripts/check.sh python scrub secrets`; `stage_python` runs `python3 -m unittest discover -s scripts/tests -p 'test_*.py'` (line 186) |
| what waits on it | `.github/workflows/ci.yml:875-878` | the aggregate `ci` needs `hygiene` |

Twenty modules under `scripts/tests` load code through `importlib`, `runpy` or `exec` by their
syntax trees, and the register names every one: `test_audit_web`, `test_backup_units`,
`test_dispatch_shards`, `test_formal_config_presence`, `test_memory_scope`, `test_mutation_python`,
`test_mutation_python_cli_kills`, `test_mutation_python_judge_kills`,
`test_mutation_python_lister_kills`, `test_mutation_python_shard_binding`,
`test_mutation_python_verdict`, `test_mutation_rows_group`, `test_mutation_rows_missing_tool`,
`test_mutation_verdict`, `test_public_scrub`, `test_rail_contract`, `test_slo_evaluator`,
`test_testflight_age`, `test_threat_model` and `test_web_engine_size`.

A text match finds four more, and every one holds the loader's words only in a string constant:
`test_mutation_rows.py:43-50` and `test_host_scrub.py:109-130`, which the register names for other
sites, and `test_deploy_scripts.py:1495` and 1512 and `test_stand_in_census.py:331`, which it does
not name. The census admits all four, because a string is no site.

The python mutation runner's population is every `scripts/<name>.py` (`scripts/mutation_python.py`
`population()`, lines 374-383), and its census refuses a script that `scripts/mutation-python.json`
does not map. The SPEC template's file manifest section (`docs/specs/_TEMPLATE.md:38-44`) asks for
every file a delivery adds or changes and names no register.

## Decisions, and the alternatives each was chosen against

### D1. The population is each `scripts/tests` module the push adds or changes, read at `HEAD`, beside the register as `HEAD` holds it

The check reads the `.py` paths that `git diff --name-only --no-renames --diff-filter=AM
<base>...HEAD -- scripts/tests` names, each by `git show HEAD:<path>`, and the register by `git show
HEAD:scripts/tests/test_ci_workflows.py`. A module is named as `module_sources()` names it. A
renamed module is judged under its new name, and a deleted one is not read. The check lives in
`scripts/`, beside the repository's other guard scripts, so the python mutation runner judges it.

Chosen against:

- Every module under `scripts/tests` on every run: rejected because the census already judges the
  whole directory in CI, and a module the push did not change was admitted there at its base.
- The work tree's files: rejected because a register entry left uncommitted would admit a module
  the push does not carry, so the check would pass a push that the census then refuses.
- A module counted when its text holds `importlib`, `runpy` or `exec` anywhere: rejected because
  `test_deploy_scripts.py` and `test_stand_in_census.py` hold those words only in strings, the
  register names neither, and the census admits both, so a text match refuses two correct modules.

### D2. `scripts/dynamic-imports-check.py` refuses, before the push, a changed loader-style module the register does not name; the census in `hygiene` stays the judge

A module is loader-style when its syntax tree holds a load of a name that an import of `importlib`
or `runpy` binds (any submodule, any alias, the `from` form included), or a load of the bare name
`exec` that the module does not bind. An unused import, a string, a comment and a star import make
no module loader-style: a star import binds no name the census can follow either, and the census
refuses the use of a name bound nowhere. Every site this rule counts is one the census counts, because
neither module is in `VETTED_MODULES` and `exec` is in `BARE_DYNAMIC`, so the check refuses a module
only where the census would refuse it too.

The register is read from its syntax tree: exactly one module-level assignment to
`DYNAMIC_IMPORTS`, a dict display whose every entry is `**allowed(reason, (module, ...), ...)` with
a string module. Any other shape, a second assignment, no assignment, or no module at all stops the
run as VOID with exit 2, naming the line; so does a changed module that does not parse, and a base
that names no commit. The check reads `DYNAMIC_IMPORTS` alone, never `NOT_WORKFLOW_READS`.

It prints `dynamic-imports: register: N module(s) in DYNAMIC_IMPORTS at HEAD`, then
`dynamic-imports: examined M changed scripts/tests module(s), L loader-style`, then one line per
refusal, `dynamic-imports: REFUSED: <path>: loads code by <importlib|runpy|exec> at line <n>;
DYNAMIC_IMPORTS lists no site of <module>`, and ends on `OK` (exit 0), `REFUSED` (exit 1),
`NOT-APPLICABLE` when no module changed (exit 0, the register still read), or `VOID` (exit 2).
`--root` defaults to the script's repository and `--base` to `origin/dev`.

The red-first tests are `test_a_planted_loader_module_missing_from_the_register_is_refused` and
`test_the_same_planted_module_registered_passes` in `scripts/tests/test_dynamic_imports_check.py`,
over planted repositories. The runner that observes them is `python3 -m unittest discover -s
scripts/tests -p test_dynamic_imports_check.py -k <name>`, before the push and again in the
`hygiene` job's python stage. The check-runs read by name are `hygiene` (the census and the new
module), `mutation-plan`, `mutation-python (<shard>)` (the script's mutants), `mutation-rows` (the
rows anchored in `test_ci_workflows.py`), `mutation-verdict`, `web` (whose docs diagram test,
SPEC-195, parses the schematic's new diagram) and the aggregate `ci`.

Chosen against:

- A check only in CI, the census alone as at `dev`: rejected because its red arrives after the
  push, as SPEC-352's first push read it, and costs a push and a CI run per missed entry.
- A census that discovers loaders by itself and drops the register: rejected because it turns a
  default-deny census, whose every site carries a reason and an exact count, into default-allow,
  so a new loader would pass unread; that is a weakening.
- A SPEC rule alone: rejected because prose is not enforced, and SPEC-347's manifest named the wrong
  table with the rule in plain view.
- A grep of each module's text: rejected because it refuses two modules the census admits (D1).
- A copy of the census's keys, each site's qualified name, text and count: rejected because it is a
  second implementation of the census's scoping, free to drift from it, and a drift refuses a
  correct push; a module-level rule cannot disagree with the census about a module it omits.
- A git pre-push hook: rejected because each clone must opt in to it, `--no-verify` skips it, and
  the repository holds no hooks directory to version it in.
- A new stage in `scripts/check.sh`: rejected because the census already runs in that script's
  python stage in CI, so a stage there reads nothing earlier.
- A parity test inside `test_ci_workflows.py`: rejected because that module's reds are read in CI,
  which is the delay this ADR removes.

### D3. The rule lives in the SPEC template's file manifest section, in these words

After the line "Every file the delivery adds or changes. A file outside this list needs an
amendment first." (`docs/specs/_TEMPLATE.md:44`), the template gains one paragraph:

> A delivery that adds a module under `scripts/tests` that loads code through `importlib`, `runpy`
> or `exec`, or adds such a load to a module there, lists `scripts/tests/test_ci_workflows.py` in
> this table as changed: its `DYNAMIC_IMPORTS` register names each such site by its module,
> qualified name and text, with its count, and the census in the python stage refuses a site the
> register does not name. Before the push, `python3 scripts/dynamic-imports-check.py --base <the
> pull request's base>` refuses a new or changed module there that loads code and that the register
> does not name.

Chosen against:

- A manifest check outside the repository alone: rejected because a contributor working from `dev`
  never reads it, and the rule must hold for every author.
- The template and the repository's contributor instructions both: rejected because two copies of
  one rule drift, and the template is the file every SPEC is written from.
- `docs/TESTING.md`: rejected because a SPEC's author writes the manifest from the template, not
  from the testing guide.

### D4. FORMAL is not applicable, by surface

The check is one process that reads immutable objects of one commit and prints a verdict; it
writes nothing another actor reads. The census is a separate run in CI over the checked-out tree,
and the two share no mutable state, so no interleaving exists for a TLA+ model to hold. The rule
is a membership test over a parsed syntax tree; its truth rests on the parser, which a Lean proof
cannot reach, and the planted table of spellings holds it. No entry under `formal/` cites the
register, the census, `scripts/tests`, `importlib` or `runpy`, and no `@phx covers` names a file
this delivery changes (the only covers of a `scripts/` file are five of `scripts/mutation-verdict.py`).

Chosen against:

- A TLA+ model of the check and the census as two actors: rejected because neither writes what the
  other reads, so every interleaving gives each the verdict it gives alone.
- A Lean proof of the subset property: rejected because the property depends on how the census
  scopes a name, which lives in Python code no proof covers; the A4 test pins the vocabulary it
  rests on instead.

### D5. No wait on open work, and one push

Four open pull requests share a path: #770 changes `scripts/tests/test_ci_workflows.py` and
`scripts/mutation-python.json` and adds `scripts/tests/test_swift_mutants.py`; #752 changes
`scripts/tests/test_ci_workflows.py`; #769 changes `scripts/tests/test_privacy_policy.py`; #765
changes `scripts/tests/test_card_web_view_layers.py`. None touches the template or the schematic.
The register gains three lines inside one group, and the map one entry in its sorted place, so a
conflict is a rebase of those lines, re-measured at the cut.

Every criterion is decided by the new module, which runs before the push, so each red is read
where it is written. The census reads the three new register lines in `hygiene` after the one push.

Chosen against:

- Two pushes, the red commit alone first: rejected because no criterion's red lives in
  `test_ci_workflows.py`, so nothing needs CI to be seen red.
- Waiting for #770 and #752 to land first: rejected because their edits and these touch different
  lines, and a re-measure at the cut finds any move before the first commit.

## Decision Outcome

`scripts/dynamic-imports-check.py` reads each changed `scripts/tests` module at `HEAD` and refuses
a loader-style one the `DYNAMIC_IMPORTS` register does not name, before the push; the SPEC template
tells every author to list the census module when a delivery adds such a load; the census in
`hygiene` stays the judge of every site and count. The delivery is one push.

### Consequences

- Good: a missing register entry for a new loader module is caught before the push, by a command
  any author can run.
- Good: the census changes by three register lines and nothing else; its default-deny reading is
  untouched.
- Good: the check can refuse only what the census would refuse, so it never blocks a correct push.
- Bad: a registered module that gains a new site still passes the check; the census reds it in
  `hygiene`, and the template's rule names that case.
- Neutral: dynamic names beyond `importlib`, `runpy` and `exec` stay the census's alone.

### Confirmation

SPEC-406's A1 to A8, the `hygiene` check-run, and the `mutation-python (<shard>)` legs that judge
the script's mutants.

## What would make this wrong

- The census vets `importlib` or `runpy`, or drops `exec` from `BARE_DYNAMIC`: the check would then
  refuse a module the census admits. A4 reads both lists from the census module and goes red first.
- The register takes a shape other than `**allowed(...)` groups: the check reads VOID on every run,
  and A4, which reads the real register in `hygiene`, goes red on that delivery.
- Authors stop running the check before the push: the census still holds, at the old cost.

## More Information

- SPEC-406 (this ADR's SPEC); SPEC-190 R12 part 1 and A12; ADR-292; SPEC-375's manifest.
- The schematic `docs/schematics/ci-jobs-and-caches.md`: its last section, which SPEC-406 adds.
