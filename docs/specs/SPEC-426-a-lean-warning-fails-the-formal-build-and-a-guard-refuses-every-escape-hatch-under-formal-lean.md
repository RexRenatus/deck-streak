# SPEC-426: a Lean warning fails the formal build, and a guard refuses every escape hatch under formal/lean

- **Issue:** none; the pull request closes nothing. **Context(s):** `formal` (`formal/lean/`) and
  `repo` (`scripts/tests/` and `docs/`).
- **Decided by:** ADR-440 (this SPEC's own: the Formal library's warnings are errors, the pin, the
  local reading of the build, the guard and its matcher, and what each was chosen against).
- **Status:** built. It holds `docs/red-first/SPEC-426.md`.

## 1. The problem, measured

Every figure below was read at the development branch's commit `ac7ebd75`; each one the tests
re-derive, they print.

- **A warning does not fail the build.** `formal/lean/lakefile.toml:1-8` declares one library,
  `Formal`, built by the glob `Formal.*` as the package's one default target, and sets no option:
  `git show ac7ebd75:formal/lean/lakefile.toml | grep -c -e leanOptions -e LeanArgs` reads 0. With
  no option set, Lean prints a warning and `lake build` still exits 0, so a module with a warning
  passes every build of the package. Section 4's base reading records that exit and the warning.
- **The package depends on nothing.** `formal/lean/lake-manifest.json` reads `"packages": []`, and
  `formal/lean/lean-toolchain` pins the Lean every build uses.
- **Nothing refuses an escape hatch.** No test, stage or workflow reads the Lean sources. Under
  `formal/lean`, `git ls-tree -r --name-only ac7ebd75 -- formal/lean` lists 28 `.lean` files, 6164
  lines in all, and a word search of them (`git grep -w -c <word> ac7ebd75 -- formal/lean`, fenced
  as the house requires) reads 0 for each of `sorry`, `admit`, `axiom`, `native_decide`,
  `implemented_by`, `extern`, `unsafe`, `nolint`, `set_option` and `maxHeartbeats`. The word
  `decide`, which the kernel checks, occurs 99 times. A substring search for `admit` reads 56 hits
  in four files, every one prose or the theorem name `rposition_some_admits`.
- **Where the build runs.** `git grep -l 'lake build' ac7ebd75` lists no file, and the only matches
  of `lean` under `.github/workflows/` are the word `clean`. `config/formal.json` names the formal
  checker's Lean budget, `budgets.lean_seconds`. So the package is built where the formal checker
  runs, outside this repository, and by hand; CI builds no Lean.
- **Where a guard can run.** The python stage, `scripts/check.sh:185-186`, discovers every
  `scripts/tests/test_*.py` module, in the `hygiene` job (`.github/workflows/ci.yml:298` and
  `:351`). The python stage took 727 s in hygiene job 116828746762 at `ac7ebd75`.

## 2. Requirements

- **R1.** `formal/lean/lakefile.toml` keeps lines 1 to 8 byte for byte and gains, directly after
  them, inside the `[[lean_lib]]` table named `Formal`, exactly these three lines:

  ```toml
  # ADR-440. Every warning Lean prints while it builds the library is an error, so a warning
  # fails the build and every formal check that runs it.
  leanOptions = { warningAsError = true }
  ```

- **R2.** `scripts/tests/test_formal_config.py` gains the test class `LakefileWarnings`, outside
  `FormalConfig`. It reads `formal/lean/lakefile.toml` as the tree stores it, a link at `formal`,
  at `formal/lean` or at the file refused by assertion naming that component, parses it with
  `tomllib`, and holds the parsed document equal to `LAKEFILE`, the declared document written in
  the test as data: `name` `"formal"`, `defaultTargets` `["Formal"]`, and one `lean_lib` with
  `name` `"Formal"`, `globs` `["Formal.*"]` and `leanOptions` `{"warningAsError": True}`. Its
  reader, `warnings_are_errors(doc)`, returns a list of problems, each its arm and a message naming
  the place, and answers none for the committed document. Its arms: `no-formal-library` (no
  `lean_lib` named `Formal`), `formal-library-twice` (two of them), `key-absent` (the library's
  own `leanOptions` lacks `warningAsError`), `key-not-true` (the value is anything but the boolean
  true: false, a string, an integer) and `key-in-args` (a `moreLeanArgs` or `weakLeanArgs`, of
  the package or of the library, holds an argument naming `warningAsError`). The reader is not
  named `read`, and no test of the class calls `load`. The module docstring names SPEC-426 beside
  SPEC-295.
- **R3.** Under R1's key, a Lean warning fails `lake build` of the package. The local reading of
  section 4 decides it, with the Lean `formal/lean/lean-toolchain` names, in an export of each
  commit's `formal/lean` outside the work tree: the planted module of section 4 builds with exit 0
  and a printed warning at the base, and with a non-zero exit and the same diagnostic as an error
  at the head; the head with no plant exits 0 with no warning and no error; a base build cached
  with the plant, given the head's lakefile, fails; and the plant headed by
  `set_option warningAsError false` is read at the head and recorded as read.
- **R4.** `scripts/tests/test_lean_escape_hatches.py` reads every path `git ls-files` lists under
  `formal/lean/` ending in `.lean`, read from the work tree, with a link at any component refused
  by assertion. It prints `examined <n> Lean files under formal/lean` through `_support.examined`,
  so 0 examined fails, then `read <m> lines in <s> s`, and it holds `formal/lean/Formal.lean` among
  the examined. A file git does not track is never judged.
- **R5.** The guard refuses each of these in Lean code, by its arm name, its path and its line:
  `sorry`; the `admit` tactic (`admit`); `axiom`; `native_decide`; `implemented_by`; the
  `@[extern]` attribute (`extern`); `unsafe`; `set_option linter.* false` (`linter-off`);
  `maxHeartbeats 0` (`heartbeats-zero`); and the `@[nolint]` attribute (`nolint`). It does not
  refuse `decide`.
- **R6.** The matcher reads code as ADR-440 D4 states: a line comment from `--`; a block comment
  from `/-` to its matching `-/`, nesting counted, doc and module comments alike; a string from `"`
  to the next unescaped `"`; an interpolated string, a `"` directly after `!`, whose `{` to
  matching `}` spans are code; a raw string `r#…"…"#…` closed by as many `#`; a character literal,
  a `'` after no identifier character holding one character or one escape; every removed
  character a space and every newline kept. An unterminated block comment, string, raw string or
  interpolation is refused by name (`unterminated-comment`, `unterminated-string`,
  `unterminated-raw-string`, `unterminated-interpolation`). Code is a stream of names, guillemet
  names and numerals, punctuation dropped. A word arm is a token equal to its word; `linter-off` is
  `set_option`, a name beginning `linter.` and `false`, consecutive; `heartbeats-zero` is
  `set_option`, a name that is `maxHeartbeats` or ends in `.maxHeartbeats`, and a numeral of value 0
  in decimal, `0x`, `0b` or `0o`, consecutive.
- **R7.** The module keeps the allow-list `ALLOWED`, a tuple of 0 entries, each entry a path, an
  arm, a count and a reason. A file's hits of one arm pass only when an entry names that path, that
  arm and exactly that count; an entry that admits no hit, or names another count, is refused as
  stale. The guard prints the list's size.
- **R8.** The module holds `ORACLE_PLANTS`, section 9's table byte for byte, and `LEAN_WARNS`, one
  boolean per plant: whether the Lean `formal/lean/lean-toolchain` names printed
  `declaration uses 'sorry'` when it elaborated that plant alone, read in section 4's local
  reading. The guard's `sorry` arm refuses exactly the plants whose recorded verdict is true; the
  two tables name the same plants; and each verdict occurs at least once.
- **R9.** The guard refuses a module under `formal/lean` whose code names `warningAsError`
  (`key-off`), since a module that sets it switches R1's key off for itself. A mention in a
  comment or a string is not refused.
- **R10.** The guard runs in the python stage by discovery. `scripts/check.sh` and every workflow
  are unchanged. The seconds the guard prints are a print, never an assertion.
- **R11.** The formal reading of section 4: the formal checker's Lean check and its ratchet-only
  check at the committed head, each beside the same check at the base; each Lean entry's
  clean-landing count read at both; every `@phx covers` digest equal at both.

## 3. Acceptance criteria of SPEC-426

| id | criterion | decided by |
|---|---|---|
| A1 | the committed lakefile, read as the tree stores it and parsed, equals `LAKEFILE`, whose Formal library's `leanOptions` holds `warningAsError` as the boolean true, and the reader answers no problem for it | `test_formal_config.py` `LakefileWarnings.test_the_formal_library_makes_every_warning_an_error` |
| A2 | each planted lakefile fault, each derived from `LAKEFILE` by one edit (the key absent; false; the string `"true"`; the integer 1; set only in the package's own `leanOptions`; the library renamed; no library; a second Formal library; an argument naming the key in the package's `moreLeanArgs`, the library's `moreLeanArgs` and the library's `weakLeanArgs`), is refused by its arm naming its place and is unequal to `LAKEFILE`; the planted arms are exactly the arms the reader's source names, found by its syntax tree | `LakefileWarnings.test_the_lakefile_reader_refuses_each_planted_fault_by_its_arm` |
| A3 | in a scratch tree, a link at each component of the lakefile's path (`formal`, `formal/lean`, the file) is refused by assertion naming that component, and the same tree with no link is read | `LakefileWarnings.test_the_lakefile_is_read_as_the_tree_stores_it` |
| A4 | the live tree: the examined count printed, `formal/lean/Formal.lean` among the examined, and no hit outside `ALLOWED`; a copy of the live tree with one hatch planted at a known line of one file is refused naming exactly that file, line and arm; a link at a tracked `.lean` path in a scratch tree is refused by assertion naming it | `test_lean_escape_hatches.py` `EscapeHatches.test_no_lean_file_under_formal_lean_holds_an_escape_hatch` |
| A5 | each of the ten arms, planted in code, is refused by its own arm name at its line; the same text in a line comment, a block comment, a doc comment and a string is admitted; and these are admitted: `decide`, `set_option linter.unusedVariables true`, `set_option maxHeartbeats 400000`, `unsafeCast` and the name `rposition_some_admits`; `0x0`, `0b0` and `0o0` read as zero and `synthInstance.maxHeartbeats` as a heartbeat name | `EscapeHatches.test_each_hatch_in_code_is_refused_by_its_name_and_none_in_prose` |
| A6 | an unterminated block comment, string, raw string and interpolation are each refused by their own name, and the matcher's code keeps every line of a terminated file | `EscapeHatches.test_an_unterminated_comment_or_string_is_refused_by_name` |
| A7 | for every plant of `ORACLE_PLANTS`, the `sorry` arm refuses it exactly when `LEAN_WARNS` records true; the two tables name the same plants, and each verdict occurs at least once | `EscapeHatches.test_the_matcher_agrees_with_leans_own_verdict_on_each_planted_file` |
| A8 | the live tree names `warningAsError` in no module's code; a planted module that sets it false is refused as `key-off`, and the same name in a comment and in a string is admitted | `EscapeHatches.test_no_lean_module_switches_warnings_as_errors_off` |
| A9 | `ALLOWED` holds 0 entries; a planted entry that names a planted hit's path, arm and count admits it; one naming another count, and one that admits no hit, are each refused as stale | `EscapeHatches.test_the_allow_list_is_empty_and_an_unused_entry_is_refused` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k the_formal_library_makes_every_warning_an_error
A2: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k the_lakefile_reader_refuses_each_planted_fault_by_its_arm
A3: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k the_lakefile_is_read_as_the_tree_stores_it
A4: python3 -m unittest discover -s scripts/tests -p test_lean_escape_hatches.py -k no_lean_file_under_formal_lean_holds_an_escape_hatch
A5: python3 -m unittest discover -s scripts/tests -p test_lean_escape_hatches.py -k each_hatch_in_code_is_refused_by_its_name_and_none_in_prose
A6: python3 -m unittest discover -s scripts/tests -p test_lean_escape_hatches.py -k an_unterminated_comment_or_string_is_refused_by_name
A7: python3 -m unittest discover -s scripts/tests -p test_lean_escape_hatches.py -k the_matcher_agrees_with_leans_own_verdict_on_each_planted_file
A8: python3 -m unittest discover -s scripts/tests -p test_lean_escape_hatches.py -k no_lean_module_switches_warnings_as_errors_off
A9: python3 -m unittest discover -s scripts/tests -p test_lean_escape_hatches.py -k the_allow_list_is_empty_and_an_unused_entry_is_refused
```

R1's key and R2 are decided by A1 to A3; R4 to R7 by A4 to A6 and A9; R8 by A7; R9 by A8. R3 and
R11 are decided by the local reading of section 4, which no repository test runs. R1's exact lines
and R10 are rules of the diff, read in review.

## 4. The local reading of the build, decided outside the test suite

- **Who and when.** The builder: the oracle and the base readings before the first commit, the
  head readings once the implementation is committed, and the formal checker's checks once the
  rows are committed, because the checker reads the committed tree. Then the review, and the
  check at landing.
- **The tool.** The Lean that `formal/lean/lean-toolchain` names; its `lean --version` is read
  against that file first, and any other Lean is VOID, never a reading.
- **The planted module** is `Formal/PlantedWarning.lean`, which the library's glob builds because
  the file exists:

  ```lean
  def plantedUnused : Nat :=
    let unused := 1
    0
  ```

- **What runs**, each `lake build` in an export of the named commit's `formal/lean`, outside the
  work tree, which never holds the plant:
  1. the base with the plant: the exit code and the unused-variable warning;
  2. the head with the plant: a non-zero exit and the same diagnostic as an error;
  3. the head with no plant: exit 0, with no warning and no error;
  4. the base's export, built with the plant, given the head's lakefile and built again with no
     clean: a non-zero exit;
  5. the head with the plant headed by `set_option warningAsError false`: the exit code and output
     as read;
  6. the oracle: each plant of section 9, written to its own file and elaborated by `lean` alone,
     with whether its output holds `declaration uses 'sorry'`;
  7. the formal checker's Lean check (`formal check --kind lean`) and its ratchet-only check at the
     committed head, each beside the same check at the base, and each Lean entry's clean-landing
     count (`formal ramp`) at both.
- **The record.** Sections of `docs/red-first/SPEC-426.md` quoting each exit code and each
  diagnostic line relative to the package, the oracle's verdict per plant, and the checks' verdict
  lines with each theorem row's `clean` and `ramp`. A reading the step does not name is quoted, and
  a `VOID` line is a stop, never a reading.

## 5. File manifest

| path | part | change |
|---|---|---|
| `formal/lean/lakefile.toml` | formal | changed: R1's three lines after line 8 |
| `scripts/tests/test_formal_config.py` | scripts | changed: the class `LakefileWarnings` (A1 to A3) and the docstring |
| `scripts/tests/test_lean_escape_hatches.py` | scripts | added: the guard (A4 to A9) |
| `docs/specs/SPEC-426-a-lean-warning-fails-the-formal-build-and-a-guard-refuses-every-escape-hatch-under-formal-lean.md` | docs | added |
| `docs/decisions/ADR-440-the-formal-library-makes-every-warning-an-error-and-a-lexing-guard-refuses-the-escape-hatches-outside-an-empty-allow-list.md` | docs | added |
| `docs/schematics/lean-warnings-are-errors-and-the-escape-hatch-guard.md` | docs | added |
| `docs/red-first/SPEC-426.md` | docs | added |
| `scripts/mutation-rows.d/S42600-S42699.json` | scripts | added: section 8's rows |
| `changelog.d/lean-lint-426.md` | changelog | added |

## 6. What this does NOT cover

- It changes neither the Formal library's glob nor the package's one library, the shape #472 set
  (#472).
- It changes no field of `config/formal.json`: the axiom allow-list, `k`, the budgets and the
  toolchain identity stay as #468 set them (#468).
- It adds no lint for the TLA+ models under `formal/tla` (#864).
- It adds no Batteries `#lint` run, which would be a new lake dependency (#865).
- It adds no CI job that builds Lean or runs TLC: the build's rule is read locally, as section 4 says (#806).

## 7. Risks

- **The key's spelling.** If R1's key, spelled so, did not take effect, only section 4's head
  reading would see it: step 2 would exit 0, and that is a stop before any push of the
  implementation.
- **A tree-wide guard.** The guard reads every Lean file, so a sibling pull request that adds a
  hatch under `formal/lean` turns red in its own python stage. That is the rule working.
- **The presence harness.** SPEC-295's harness runs exactly `FormalConfig`'s tests under planted
  settings files. A later move of `LakefileWarnings` into `FormalConfig` would make the harness
  answer for the lakefile; the class stays outside it.
- **Test-directory censuses.** A census that reads every module of `scripts/tests` may name the new
  module's reads or its process call; CI's first run reads every census, and a red there is fixed
  only by the entry it names, recorded.
- **The oracle's release.** `LEAN_WARNS` is the pinned Lean's reading. A release that lexes
  comments, strings or literals otherwise needs the oracle read again with that release.
- **Anchors.** 35 rows anchor on `scripts/tests/test_formal_config.py`; the new class repeats no
  anchored line, and the builder's anchor census proves each find occurs exactly once.

## 8. The mutation rows

Band `S42600` to `S42699`, `scripts/mutation-rows.d/S42600-S42699.json`, in `SCRIPT_MUTATIONS`. One
row per arm, word, boundary and lexical rule, each killed by the test its criterion names. The
record commit fills each row's find and replacement into this table, each find written from the
landed line and occurring exactly once in its file.

| id | target | the mutant | killer |
|---|---|---|---|
| `S42600-THE-FORMAL-LIBRARY-MAKES-EVERY-WARNING-AN-ERROR` | `formal/lean/lakefile.toml` | the key's value set false | A1 |
| `S42601-THE-DECLARED-DOCUMENT-HOLDS-THE-KEY-ON` | `scripts/tests/test_formal_config.py` | `LAKEFILE`'s key set false | A1 |
| `S42602-AN-ABSENT-KEY-IS-REFUSED` | `scripts/tests/test_formal_config.py` | `key-absent` answers nothing | A2 |
| `S42603-ONLY-THE-BOOLEAN-TRUE-TURNS-THE-KEY-ON` | `scripts/tests/test_formal_config.py` | any truthy value admitted | A2 |
| `S42604-THE-KEY-IS-READ-ON-THE-FORMAL-LIBRARY-ALONE` | `scripts/tests/test_formal_config.py` | a library of any name taken | A2 |
| `S42605-A-SECOND-FORMAL-LIBRARY-IS-REFUSED` | `scripts/tests/test_formal_config.py` | two Formal libraries admitted | A2 |
| `S42606-AN-ARGUMENT-NAMING-THE-KEY-IS-REFUSED` | `scripts/tests/test_formal_config.py` | `key-in-args` answers nothing | A2 |
| `S42607-THE-ARGUMENT-ARM-READS-THE-PACKAGE-TOO` | `scripts/tests/test_formal_config.py` | the package's arrays not read | A2 |
| `S42608-A-LINK-ON-THE-LAKEFILE-PATH-IS-REFUSED` | `scripts/tests/test_formal_config.py` | the link walk skipped | A3 |
| `S42609-SORRY-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the word replaced by a non-word | A5 |
| `S42610-THE-ADMIT-TACTIC-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the word replaced by a non-word | A5 |
| `S42611-AXIOM-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the word replaced by a non-word | A5 |
| `S42612-NATIVE-DECIDE-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the word replaced by a non-word | A5 |
| `S42613-IMPLEMENTED-BY-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the word replaced by a non-word | A5 |
| `S42614-THE-EXTERN-ATTRIBUTE-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the word replaced by a non-word | A5 |
| `S42615-UNSAFE-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the word replaced by a non-word | A5 |
| `S42616-THE-NOLINT-ATTRIBUTE-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the word replaced by a non-word | A5 |
| `S42617-A-LINTER-SWITCHED-OFF-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the `linter.` prefix narrowed | A5 |
| `S42618-ONLY-FALSE-SWITCHES-A-LINTER-OFF` | `scripts/tests/test_lean_escape_hatches.py` | `true` also read as off | A5 |
| `S42619-A-ZERO-HEARTBEAT-LIMIT-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the zero test moved to one | A5 |
| `S42620-THE-HEARTBEAT-ARM-READS-A-DOTTED-NAME` | `scripts/tests/test_lean_escape_hatches.py` | the dotted suffix not read | A5 |
| `S42621-A-ZERO-IN-ANOTHER-BASE-IS-ZERO` | `scripts/tests/test_lean_escape_hatches.py` | numerals read in decimal only | A5 |
| `S42622-A-LINE-COMMENT-IS-PROSE` | `scripts/tests/test_lean_escape_hatches.py` | the line-comment marker narrowed | A5 |
| `S42623-BLOCK-COMMENTS-NEST` | `scripts/tests/test_lean_escape_hatches.py` | the nesting count held at one | A7 |
| `S42624-AN-ESCAPED-CHARACTER-STAYS-IN-ITS-STRING` | `scripts/tests/test_lean_escape_hatches.py` | a backslash takes nothing with it | A7 |
| `S42625-AN-INTERPOLATION-IS-CODE` | `scripts/tests/test_lean_escape_hatches.py` | no string read as interpolated | A7 |
| `S42626-A-CHARACTER-LITERAL-IS-NOT-A-STRING` | `scripts/tests/test_lean_escape_hatches.py` | the character-literal arm never taken | A7 |
| `S42627-A-RAW-STRING-ENDS-AT-ITS-OWN-HASHES` | `scripts/tests/test_lean_escape_hatches.py` | the closing hashes not counted | A7 |
| `S42628-AN-UNTERMINATED-BLOCK-COMMENT-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the refusal answers nothing | A6 |
| `S42629-AN-UNTERMINATED-STRING-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the refusal answers nothing | A6 |
| `S42630-AN-UNTERMINATED-RAW-STRING-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the refusal answers nothing | A6 |
| `S42631-AN-UNTERMINATED-INTERPOLATION-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the refusal answers nothing | A6 |
| `S42632-THE-POPULATION-IS-EVERY-LEAN-FILE-UNDER-FORMAL-LEAN` | `scripts/tests/test_lean_escape_hatches.py` | the path prefix narrowed to `formal/lean/Formal/` | A4 |
| `S42633-A-LINK-AT-A-LEAN-FILE-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the link walk skipped | A4 |
| `S42634-AN-ALLOW-LIST-ENTRY-ADMITS-ONLY-ITS-OWN-COUNT` | `scripts/tests/test_lean_escape_hatches.py` | the count not compared | A9 |
| `S42635-AN-UNUSED-ALLOW-LIST-ENTRY-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | the stale arm answers nothing | A9 |
| `S42636-A-MODULE-THAT-NAMES-THE-KEY-IS-REFUSED` | `scripts/tests/test_lean_escape_hatches.py` | `key-off` answers nothing | A8 |

## 9. The oracle's planted files

`ORACLE_PLANTS`, written into the guard module exactly so; each value is one whole Lean file that
Lean elaborates alone. Its verdicts, `LEAN_WARNS`, are read in section 4 and never written here.

```python
ORACLE_PLANTS = {
    "a sorry in code": "theorem planted : True := sorry\n",
    "a sorry in a line comment": "theorem planted : True := trivial -- sorry\n",
    "a sorry in a block comment": "/- sorry -/\ntheorem planted : True := trivial\n",
    "a sorry in a nested block comment": (
        "/- outer /- inner -/ sorry -/\ntheorem planted : True := trivial\n"
    ),
    "a sorry in a doc comment": "/-- sorry -/\ntheorem planted : True := trivial\n",
    "a sorry in a string": 'def planted : String := "sorry"\n',
    "a sorry after an escaped quote in a string": 'def planted : String := "\\" sorry"\n',
    "a sorry after a string that holds a comment marker": (
        'def planted : String := "--" ++ toString (sorry : Nat)\n'
    ),
    "a sorry after a string that holds a block opener": (
        'def planted : String := "/-" ++ toString (sorry : Nat)\n'
    ),
    "a sorry after a quote character literal": (
        "def planted : Char × Nat := ('\"', (sorry : Nat))\n"
    ),
    "a sorry after an escaped character literal": (
        "def planted : Char × Nat := ('\\'', (sorry : Nat))\n"
    ),
    "a sorry in an interpolation": 'def planted : String := s!"{(sorry : Nat)}"\n',
    "a sorry in an interpolated string's text": 'def planted : String := s!"sorry {1}"\n',
    "a sorry in a raw string": 'def planted : String := r#"a " sorry"#\n',
    "a sorry after a primed name": "def planted (x' : Nat) : Nat := x' + (sorry : Nat)\n",
    "a quoted name spelled as the word": "def «sorry» : Nat := 1\ndef planted : Nat := «sorry»\n",
    "a theorem name that holds the word": "theorem no_sorry_admits : True := trivial\n",
}
```

## 10. References

ADR-440; SPEC-295 and ADR-295 (the settings file and its test module); ADR-305 D1 (the package
requires nothing); issue #472 (the library's shape) and issue #468 (the settings file); the
schematic `docs/schematics/lean-warnings-are-errors-and-the-escape-hatch-guard.md`.
