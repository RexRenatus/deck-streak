# SPEC-612: the dynamic-imports check agrees with the census on bindings, register writes and text, and reads one commit

- **Wave:** the test-module census (SPEC-190 R12 part 1). **Issue:** #804 (the follow-up of the
  dynamic-imports register delivery: three shapes the check and the census read differently, and a
  check that names `HEAD` in three reads). **Context(s):** `repo` (`scripts/`, `scripts/tests/`,
  `docs/`).
- **Decided by:** ADR-634 (this SPEC's own: the binding kinds, the register writes, the text, the
  one commit, FORMAL) and ADR-420 (the check, its population and its printed lines).
- **Schematic:** `docs/schematics/ci-jobs-and-caches.md`, its last section, unchanged: the check
  still runs before the push and the census still runs in the `hygiene` job.
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-612.md`. **Mutation
  band:** S61200-S61299, unused (section 7).

## 1. The problem, measured

Read at `dev` b065fce, by the line numbers of each file there.

- **Bindings.** `scripts/dynamic-imports-check.py` lines 44-66 read a module's own `exec` from 7
  binding kinds: imports, function, async function and class definitions, arguments, and names
  stored or deleted. The census's `bindings()` (`scripts/tests/test_ci_workflows.py` lines
  4342-4364) reads 16: the `ast` classes its body names, `ast.Load` aside (AsyncFunctionDef,
  ClassDef, ExceptHandler, FunctionDef, Global, Import, ImportFrom, MatchAs, MatchMapping,
  MatchStar, Name, Nonlocal, ParamSpec, TypeVar, TypeVarTuple, arg). So 9 kinds bind `exec` for the
  census and not for the check, and a module that binds `exec` by `except ... as exec` and calls it
  is refused by the check and admitted by the census, against ADR-420's driver "The check never
  refuses a module the census admits".
- **Register writes.** `register_modules` (lines 94-104) collects module-level `ast.Assign` and
  `ast.AnnAssign` nodes only, so a site added by `DYNAMIC_IMPORTS |= {...}` is never read and its
  module is refused. The real register writes the name once (line 7482) and reads it once more (line
  4882), by a syntax-tree read of every name node in the file.
- **Text.** `_git` (lines 137-147) runs git with `encoding="utf-8"` (line 142), so a blob that is
  not UTF-8 raises `UnicodeDecodeError` inside `subprocess.run`, which `main` (lines 195-207) does
  not catch: the run ends in a traceback with exit 1, never the VOID exit 2. The census reads every
  module as UTF-8 too (`module_sources`, lines 4627-4635).
- **One commit.** `_judge` names `HEAD` at lines 167 and 174, and again through `changed_modules`
  at line 158, so a ref that moves between those reads gives a verdict over two commits.

## 2. Requirements

- R1. Bindings. `loader_sites` reads `exec` as the module's own, and finds no `exec` load, when the
  module binds `exec` by any binding the census's `bindings()` reads: a definition (`def`, `async
  def`, `class`), a name stored or deleted, an argument, a capture (`except ... as`, a match capture
  or `as`, a match star, a match mapping rest), a type parameter (`TypeVar`, `ParamSpec`,
  `TypeVarTuple`), a `global` or `nonlocal` statement, and an import (its `asname`, else its name's
  first segment, never `*`). The reading stays module-wide, as SPEC-406's risks say: it can only
  admit a module the census also admits or judges in `hygiene`.
- R2. Register additions. The register's modules are the first elements of the sites of the one
  module-level plain assignment (SPEC-406 R3, unchanged) and of every module-level
  `DYNAMIC_IMPORTS |= {...}` after it whose value is a dict display of `**allowed(reason, site,
  ...)` entries, each entry read by SPEC-406 R3's rules and causes.
- R3. Unread writes stop the run. Any other write to `DYNAMIC_IMPORTS` anywhere in the register
  module stops the run as VOID with the cause `DYNAMIC_IMPORTS is written at line <n> in a shape the
  check does not read`: an augmented assignment by another operator, a `|=` whose value is not a
  dict display, a `|=` before the plain assignment or not at module level, any other binding of the
  name by R1's kinds (a `del` of the name and a store inside a function among them; a `global` or
  `nonlocal` statement only declares it), a subscript store or delete, and any use of its members
  `clear`, `pop`, `popitem`, `setdefault`, `update`, `__delitem__`, `__ior__` or `__setitem__`. A
  load of the name or of any other member (`items`, `keys`, `get`) is no write. A second plain
  assignment keeps SPEC-406's `assigned a second time` cause.
- R4. Text. The check decodes every git output itself, strictly, as UTF-8; a blob that is not
  UTF-8 stops the run as VOID (exit 2) with a cause that names its path and the words `is not UTF-8
  text`; git's own error text is decoded with replacement and never raises.
- R5. One commit. The check's first git process is `git rev-parse --verify HEAD^{commit}`; its
  output names the commit of every later read (`show <commit>:<path>` for the register and each
  module, `<base>...<commit>` for the listing), and no later process names `HEAD`. Every git process
  the check starts goes through the one function `_git`. A `HEAD` that does not resolve stops the
  run as VOID.
- R6. The printed lines are SPEC-406 R4's, byte for byte (the register line still ends `at HEAD`),
  and SPEC-406's eight tests pass unchanged.
- R7. The tests. The six tests below join `scripts/tests/test_dynamic_imports_check.py` inside its
  class; its imports, its helpers and its eight tests stay byte for byte. They add no load of code:
  every `exec`, `importlib` and `runpy` word in them sits inside a string constant (SPEC-406's
  real-register test reads this module's own source and requires exactly its two `importlib`
  loads); they name no dunder but those the census's `BENIGN_DUNDERS` names and no builtin outside
  its `BENIGN_BUILTINS`; they read files only by the forms the module already uses and walk no
  directory, so `DYNAMIC_IMPORTS` and `NOT_WORKFLOW_READS` need no new line.
- R8. Mutation. No row: the check is in the python mutation population and
  `scripts/mutation-python.json` maps it to `test_dynamic_imports_check` (SPEC-406 R9, unchanged),
  so the `mutation-python` legs judge the changed lines' mutants. No row in `scripts/mutation-rows.d`
  targets the check or its test module.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a table of 16 planted modules, one per binding kind, each binding `exec` by that kind only and calling it in that binding's scope, reads no load on every row; the table's kinds equal the `ast` class names the census's `bindings()` body names, read from its syntax tree, `Load` dropped: 16 | `scripts/tests/test_dynamic_imports_check.py` `test_every_binding_the_census_reads_binds_exec_for_the_check` |
| A2 | run as a program, a head that adds a module binding `exec` by `except ... as exec` exits 0, examines 1 module, 0 loader-style, and ends `OK` | `scripts/tests/test_dynamic_imports_check.py` `test_a_module_that_binds_exec_by_a_capture_is_not_refused` |
| A3 | a site added by a module-level `\|=` of `allowed` groups is read: one such line gives `register: 2 module(s)` and admits its loader module; two give `register: 3 module(s)` | `scripts/tests/test_dynamic_imports_check.py` `test_a_site_added_to_the_register_by_an_augmented_assignment_is_read` |
| A4 | each register write R3 names (another operator, a `\|=` of a name, before the plain assignment or inside a function, a subscript store or delete, a `del`, each changing member) exits 2 with a VOID line naming its line, and a `\|=` of another helper keeps SPEC-406's cause | `scripts/tests/test_dynamic_imports_check.py` `test_a_register_write_the_check_cannot_read_stops_the_run` |
| A5 | run as a program, a changed module and a register that are not UTF-8 each exit 2 with a VOID line naming the path and `is not UTF-8 text` | `scripts/tests/test_dynamic_imports_check.py` `test_a_blob_that_is_not_utf8_reads_void` |
| A6 | with `HEAD` moved after the first git read, the run judges the first commit: 4 git calls, the first `rev-parse --verify HEAD^{commit}`, no other naming `HEAD`, each other naming that commit and none the moved one | `scripts/tests/test_dynamic_imports_check.py` `test_the_check_resolves_head_once_and_reads_every_object_from_that_commit` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_every_binding_the_census_reads_binds_exec_for_the_check
A2: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_a_module_that_binds_exec_by_a_capture_is_not_refused
A3: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_a_site_added_to_the_register_by_an_augmented_assignment_is_read
A4: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_a_register_write_the_check_cannot_read_stops_the_run
A5: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_a_blob_that_is_not_utf8_reads_void
A6: python3 -m unittest discover -s scripts/tests -p test_dynamic_imports_check.py -k test_the_check_resolves_head_once_and_reads_every_object_from_that_commit
```

SPEC-406's eight tests stay in the same module and keep deciding its criteria; the census's own
verdict on the new tests is read in the `hygiene` check-run, and no criterion of this SPEC is
decided there.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/dynamic-imports-check.py` | `repo` | changed: R1 to R6 |
| `scripts/tests/test_dynamic_imports_check.py` | `repo` | changed: the six tests (R7), inserted inside the class |
| `docs/specs/SPEC-612-the-dynamic-imports-check-agrees-with-the-census-on-bindings-register-writes-and-text-and-reads-one-commit.md` | `repo` | added: this SPEC |
| `docs/decisions/ADR-634-the-dynamic-imports-check-takes-its-binding-kinds-from-the-census-reads-augmented-register-sites-decodes-strictly-and-resolves-head-once.md` | `repo` | added: its ADR |
| `docs/red-first/SPEC-612.md` | `repo` | added: the red-first record |
| `changelog.d/dynamic-imports-agreement-612.md` | `repo` | added: the fragment |

## 5. What this does NOT cover

- It does not change the census, its register or any row anchored in the census module; the
  census in `hygiene` stays the judge of every site and count (#377).
- It does not re-cite SPEC-406's five bullets that cite #377, the issue's optional third point
  (#804).
- It reads no register write beyond R2 and R3: a write through another name bound to the register
  is not read, and none exists at `dev` b065fce (#804).
- It adds no git hook, no CI job and no stage to the repository's check script; the check is still
  run before the push (#804).
- It changes no price line and no mutation row (#804).

## 6. Risks

- **The census gains a binding kind.** The check would then refuse a module the census admits.
  Detected by A1, which reads the census's kinds from its syntax tree in `hygiene` on the delivery
  that adds one.
- **The register grows through another name bound to it** (an alias whose members are called).
  The check does not read it and may refuse a module the alias registers; the census counts the
  site in `hygiene`, and no such alias exists at `dev` b065fce.
- **`HEAD` names a tag.** `^{commit}` peels it to the commit it tags, so every read still names one
  commit.

## 7. Mutation rows

No row is added: the check is in the python mutation runner's population, and the
`mutation-python` legs judge its changed lines' mutants by `test_dynamic_imports_check` through
the map entry SPEC-406 R9 added. The SPEC, ADR, record and fragment carry no behaviour.
