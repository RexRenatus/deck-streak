---
status: accepted
decision-makers: "the DeckStreak architect"
---

# The Formal library makes every warning an error, and a lexing guard refuses the escape hatches under formal/lean outside an empty allow-list

Decides SPEC-426. Read at the development branch's commit `ac7ebd75`.

## Context and Problem Statement

- **A warning does not fail the Lean build.** `formal/lean/lakefile.toml:1-8` declares one Lean
  library, `Formal`, built by the glob `Formal.*` as the package's one default target, with no
  option set: no `leanOptions`, no `moreLeanArgs`. With no option set, Lean prints a warning and
  the build's exit code stays 0, so a module that elaborates with a warning passes every build of
  the package, by hand or by the formal check. `formal/lean/lake-manifest.json` holds no package
  (`"packages": []`), so the package depends on nothing but core Lean.
- **Nothing in the tree refuses an escape hatch.** A proof can rest on `sorry`, on a new `axiom`,
  on `native_decide`, or switch a linter off, and no test, stage or workflow of this repository
  reads the Lean sources for it. At `ac7ebd75`, `formal/lean` holds 28 `.lean` files and 6164
  lines, and none of the hatches below occurs as a word in them; the word `decide`, which the
  kernel checks, occurs 99 times.
- **Where the build runs.** No file at `ac7ebd75` holds `lake build`, and no workflow under
  `.github/workflows/` names Lean: the only matches of `lean` there are the word `clean`.
  `config/formal.json` names the formal checker's Lean budget, `budgets.lean_seconds`. So the
  package is built where the formal checker runs, outside this repository, and by hand. The python
  stage of `scripts/check.sh` runs in CI on every pull request into dev, and no Lean runs there.

## Decision Drivers

- A rule that fails closed: a warning, a hatch, an empty population or a missing tool is a red,
  never a pass.
- The rule lives in the tree, where a reviewer reads it and a test pins it.
- No new dependency for the package and no new CI job.
- Text a hatch only names, in a comment or a string, is not a use of it.

## Decision Outcome

- **D1. Warnings are errors, for the Formal library.** The `[[lean_lib]]` table named `Formal` in
  `formal/lean/lakefile.toml` gains `leanOptions = { warningAsError = true }`, directly after its
  `globs` line (`lakefile.toml:8`), under one comment line that cites this ADR; lines 1 to 8 stay
  byte for byte. Lake's documentation says of `leanOptions`: "Additional options to pass to both
  the Lean language server (i.e., `lean --server`) launched by `lake serve` and to `lean` while
  compiling Lean source files", and of a library's own options: "`leanOptions`,
  `moreServerOptions`: Merges them and the library's takes precedence." `warningAsError` is Lean's
  own option for treating warnings as errors; Lean's own build passes it as
  `-DwarningAsError=true`, beside Lake's `--wfail`, when it fails on warnings. The library is the
  package's one default target (`lakefile.toml:2`), so the key reaches every module the build
  compiles and no target beyond it. Whether the key, spelled so, takes effect is decided by D3's
  measurement, never by this text: the planted module must fail the build at the head.
- **D2. The pin.** `scripts/tests/test_formal_config.py` gains a test class, `LakefileWarnings`,
  beside `FormalConfig` and outside it. It reads `formal/lean/lakefile.toml` as the tree stores it
  (a link at `formal`, at `formal/lean` or at the file is refused by assertion naming the
  component, through `_support.refuse_link_components`), parses it with the standard library's
  `tomllib`, and holds it two ways. The parsed document equals a declared document, `LAKEFILE`,
  written in the test as data, so any other key, a second library, an option set for the whole
  package or an argument array changes the document and fails. A reader,
  `warnings_are_errors(doc)`, returns each problem by its arm: `no-formal-library`,
  `formal-library-twice`, `key-absent`, `key-not-true` and `key-in-args`. It answers nothing for
  the committed document, and each arm is held by planted documents, each derived from `LAKEFILE`
  by one edit. The class sits outside `FormalConfig` because SPEC-295's presence harness runs
  exactly `FormalConfig`'s tests under each planted settings file
  (`scripts/tests/test_formal_config_presence.py:177`); the reader is not named `read` and no new
  test calls `load`, the two bare names that harness's syntax-tree census reads (`:24-25`). The
  key switched off inside a module, by `set_option warningAsError false`, is a switch no lakefile
  reading can see, so the guard module of D4 refuses any module whose code names `warningAsError`
  (`key-off`), as a separate test: it pins D1, and it adds nothing to D4's list.
- **D3. The red-first proof is a local reading of the build.** CI builds no Lean, so the proof is
  read where Lean runs: by the builder, with the Lean that `formal/lean/lean-toolchain` names,
  whose `lean --version` is checked against that file before any reading (an off-pin tool is
  VOID). Each build runs in an export of the commit's `formal/lean`, outside the work tree, which
  never holds a planted module. The planted module is `Formal/PlantedWarning.lean`, which the
  library's glob builds because the file exists:

  ```lean
  def plantedUnused : Nat :=
    let unused := 1
    0
  ```

  1. At the base, the export with the plant: `lake build` exits 0 and prints the unused-variable
     warning.
  2. At the head, the export with the plant: `lake build` exits non-zero and prints the same
     diagnostic as an error.
  3. At the head, the export with no plant: `lake build` exits 0 with no warning and no error, so
     every committed module builds clean under the key.
  4. The cache: the base's export, already built with the plant, takes the head's lakefile and
     builds again with no clean; it exits non-zero, so a build cached before the key is not
     reused past it.
  5. The override: the head's export with the plant headed by `set_option warningAsError false`;
     its exit code and output are recorded as they are read. D2's `key-off` test rests on it.

  Each exit code and each diagnostic line, quoted relative to the package, goes into
  `docs/red-first/SPEC-426.md`, with the formal checker's Lean check of the head beside its base
  control (D8).
- **D4. The escape-hatch guard.** `scripts/tests/test_lean_escape_hatches.py` refuses these in
  Lean code under `formal/lean`, each by its arm name at its line: `sorry`; the `admit` tactic;
  `axiom`; `native_decide`; `implemented_by`; the `@[extern]` attribute (`extern`); `unsafe`;
  `set_option linter.* false` (`linter-off`); `maxHeartbeats 0` (`heartbeats-zero`); and the
  `@[nolint]` attribute (`nolint`). `decide` is not on the list: the kernel checks it.
  - **The population** is every path `git ls-files` lists under `formal/lean/` ending in `.lean`,
    read from the work tree; a link at any component is refused by assertion. The guard prints
    `examined <n> Lean files under formal/lean` through `_support.examined`, which refuses 0, then
    `read <m> lines in <s> s`,
    and holds the root module `formal/lean/Formal.lean` among them, so a population that lost
    its root reads red.
  - **The allow-list** is `ALLOWED`, a tuple in the module with 0 entries, each entry a path, an
    arm, a count and a reason. A file's hits of one arm pass only when an entry names that path,
    that arm and exactly that count; an entry that admits no hit, or names another count, is
    refused as stale. The guard prints the list's size.
  - **The matcher** lexes each file into code, every removed character a space and every newline
    kept, so a hit keeps its line: (1) `--` starts a comment to the end of its line; (2) `/-` starts
    a block comment that ends at its matching `-/`, nesting counted, doc (`/--`) and module
    (`/-!`) comments alike; (3) `"` starts a string that ends at the next unescaped `"`, a
    backslash taking the next character with it; (4) a `"` that directly follows `!` (the `s!`,
    `m!` and `f!` forms) starts an interpolated string whose `{` to matching `}` spans are code,
    lexed by these same rules; (5) `r`, any number of `#` and `"`, where the `r` follows no
    identifier character, start a raw string that ends at a `"` followed by as many `#`; (6) a `'`
    that follows no identifier character, holding one character or one backslash escape and closed
    by `'`, is a character literal, and any other `'` is code (a name's prime); (7) an unterminated
    block comment, string, raw string or interpolation is refused by name, never read past. Code is
    then a stream of tokens, punctuation dropped: names (a letter or `_`, then letters, digits,
    `_`, `'`, `!` and `?`, dotted parts joined by `.`), a guillemet name `«…»` as one token, and
    numerals. A word arm is a token equal to its word. `linter-off` is `set_option`, a name that
    begins `linter.`, and `false`, consecutive. `heartbeats-zero` is `set_option`, a name that is
    `maxHeartbeats` or ends in `.maxHeartbeats`, and a numeral whose value is 0 (decimal, `0x`,
    `0b` or `0o`), consecutive.
  - **Known false positives** (refused though nothing escapes): FP1, a name spelled exactly
    `extern`, `nolint` or `implemented_by`, because the token is refused wherever it stands, not
    only inside an attribute; FP2, a quoted name such as `` `sorry `` in a metaprogram.
  - **Known false negatives** (an escape the matcher does not refuse): FN1, a hatch word in a
    comment or in string text, on purpose, since prose is no use; FN2, another spelling of an
    escape that is not on the list: `sorryAx`, the `stop` tactic, `decide +native` or its
    `native := true` configuration, `Lean.ofReduceBool`, `partial def` and `opaque`, each resting
    on a hole or an axiom the formal checker's axiom audit names for every declared theorem; FN3,
    a hatch assembled from data while a macro or a metaprogram runs; FN4, a linter or heartbeat
    setting made other than by a `set_option` command in a module (the lakefile's options, which
    D2's exact document closes, or `withOptions` in a metaprogram); FN5, a library function whose
    name begins `unsafe` (`unsafeBaseIO`, `unsafeCast`), since only the keyword is refused; FN6, a
    file git does not track, or a `.lean` file outside `formal/lean`, which the package does not
    build; FN7, a heartbeat limit written as an expression rather than a numeral.
  - **The oracle.** The matcher's lexing is held to Lean's own: a table of planted files,
    `ORACLE_PLANTS`, each placing `sorry` in one lexical position, is elaborated by the pinned
    Lean, and whether Lean printed `declaration uses 'sorry'` for each is recorded in the module as
    data, `LEAN_WARNS`. The guard's `sorry` arm must refuse exactly the plants Lean warned for.
- **D5. Placement and cost.** The guard is a test module the python stage discovers:
  `scripts/check.sh:185-186` runs `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`,
  in the `hygiene` job (`.github/workflows/ci.yml:298`, its step at `:351`), on every pull request
  into dev and every push to it. Neither `scripts/check.sh` nor any workflow changes. The python
  stage took 727 s in hygiene job 116828746762 at `ac7ebd75`, and 853 s in hygiene job 116733923723
  at `a3a052b2`. The guard reads 28 files and 6164 lines, and its examined line prints its own
  seconds, so its cost is read from the delivery's own run.
- **D6. Kill criteria.** For each of the two rules, warnings as errors and the escape-hatch guard:
  - **Live precision** is fixes ÷ (fixes + new suppressions or allow-list entries), counted over
    the last 30 merged pull requests. A fix is a change that removes a warning or a hatch the rule
    refused; a suppression is a `set_option` that switches a linter off, a `key-off` override or a
    new `ALLOWED` entry. Below 0.8 with at least 5 hits, propose a demotion.
  - **New zero-hit rules.** Both rules have 0 hits at `ac7ebd75`. If 2 or more of a rule's first 5
    hits are false, propose a demotion.
  - **A demotion is a weakening**, and it goes to the owner as a signed ruling.
  - **Tooling.** `examined 0`, or a tool that is missing or off its pin (a Lean other than the one
    `lean-toolchain` names, or an interpreter without `tomllib`), is VOID, which blocks. It is
    never a pass.
  - A kill window reads "the last N merged pull requests", never a number of days.
- **D7. Not in this delivery.** A TLA+ lint, the Batteries `#lint` command and a CI job that
  builds Lean or runs TLC are not part of this delivery. SPEC-426's exclusions name each, under
  the issue the maintainer files for it.
- **D8. Tests, rows and the formal reading.**
  - Each of A1 to A9 is red at the delivery's first commit for its own reason: the lakefile lacks
    the key, or an accept-all stub of the reader or the matcher admits a planted fault. Every stub
    keeps every input, the population, the documents and the planted files included, so no red is
    an empty selection. D3's local reading, SPEC-426 section 4, decides the build's rule outside the
    test suite. No assertion reads a clock.
  - Rows are the band `S42600` to `S42699`, one per arm, word, boundary and lexical rule, each
    killed by its criterion's test, and one on the lakefile's key line.
  - The Lean surface is the lakefile. It is a root file of the package and an input of each of the
    13 Lean entries, whose 41 theorems are every one at `ramp=report`. Its value differs at the
    base and the head, so each entry's clean-landing count restarts at this landing, and no
    theorem's mode moves. No `@phx covers` line names a file this delivery edits, so every cover's
    digest is equal at the base and the head. The ratchet reads `config/formal.json` and the
    entries, and neither changes; no vectors file changes. The formal checker's Lean check of the
    head, every entry built, audited and compared under the key, and its ratchet-only check, each
    beside its base control, are recorded in `docs/red-first/SPEC-426.md`.
  - The matcher decides a CI verdict. It is closed by an independent oracle, Lean's own
    elaboration of the planted files (D4), not by a machine-checked port of the lexer.
  - TLA+ is not applicable: one process reads committed text once, and the only value carried
    between builds, the package's build cache, is read and written by one build at a time;
    D3's cache control reads it.
  - Three pushes: the first commit alone, so CI reads every red; the implementation and the rows,
    so CI reads every green and every row; and the red-first record, which quotes both readings and
    D3's.
- **D9. One delivery, no amendment.** SPEC-426 and this ADR are the whole record. No SPEC or ADR on
  dev states a rule this changes: ADR-305 D1's "The package requires nothing" holds, since no
  dependency is added; SPEC-295's rules and its presence harness stand, since the new class sits
  outside the harness's population; and the formal checker's settings schematic draws the settings
  file, not the build. The build's flow is drawn in a schematic of its own.

## Alternatives, and what each decision was chosen against

- D1, against: Lake's `--wfail` on the formal checker's command line only: rejected, because the checker runs outside this repository, so the rule would be invisible to a reviewer, unpinnable by a test, and absent from every hand build and from the editor's server.
- D1, against: a package-level `leanOptions` for every target: rejected, because it is a global option that reaches every target the package ever declares, while the rule is the Formal library's, and the library's own options take precedence over it anyway.
- D1, against: `moreLeanArgs = ["-DwarningAsError=true"]` on the library: rejected, because an argument string is text a pin must parse, and it reaches the compiler only, never the language server.
- D1, against: `weakLeanArgs` on the library: rejected, because Lake's documentation says these "do not affect the trace of the build result", so a build cached before the key would be reused past it.
- D1, chosen: the library's own `leanOptions` with `warningAsError = true`, because it is a typed boolean a test reads, scoped to the one library, carried to both the build and the server.
- D2, against: no pin: rejected, because a later edit could drop or flip the key and every build would keep passing, a warning included.
- D2, against: a pin inside `FormalConfig`: rejected, because SPEC-295's presence harness runs exactly that class's tests under planted settings files, and a lakefile test there would answer for the wrong file.
- D2, against: a substring check for the key's text: rejected, because it admits the key in a comment, on another library or set to false.
- D3, against: a CI job that builds Lean to read the plant: rejected, because no such job is part of this delivery (D7), and the reading needs one build, not a stage.
- D3, against: a planted module committed and then removed: rejected, because the module would sit in the history of the work tree that the formal checker reads from its commits.
- D3, against: the formal checker's own card as the only reading: rejected, because the card reports each theorem, while the plant's own diagnostic line is what shows that a warning was raised to an error.
- D4, against: a raw-text grep: rejected, because a substring search reads 56 `admit` hits at `ac7ebd75` in four files, each one prose or the theorem name `rposition_some_admits`, and a word search refuses a comment that names a hatch, cannot place the two-token arms in code, and reads past an unterminated comment.
- D4, against: the Batteries `#lint` command: rejected, because it is a new lake dependency, it needs Lean where the python stage runs, and it lints declarations (docstrings, unused arguments) rather than refusing this list.
- D4, against: Lean's own parser driven by a metaprogram: rejected, because it needs Lean in CI, which no job runs.
- D4, against: the formal checker's axiom audit alone: rejected, because it audits only the declared theorems and what they reach, outside CI, and reads no `set_option` or attribute.
- D4, against: a machine-checked Lean port of the lexer: rejected, because the port would prove the port, while Lean's own elaboration of the planted files is the reading the lexer must agree with.
- D5, against: a new CI job or step for the guard: rejected, because the python stage already discovers every test module, so a workflow change would add a job for nothing the stage lacks.
- D5, against: a shell function in `scripts/check.sh`: rejected, because shell matching is the raw-text grep D4 refuses, with no planted controls.
- D6, against: a kill window in days: rejected, because a quiet stretch would empty the window and read as precision.
- D7, against: a TLA+ lint, the Batteries `#lint` command or a CI job that builds Lean, inside this delivery: rejected, because each is its own decision with its own cost (a lint of another language, a new lake dependency, a new job), and none is decided here.
- D8, against: one push of red and green together: rejected, because CI is the only place the reds are read, and a red read after its green proves nothing.
- D9, against: an insert-only amendment of SPEC-295: rejected, because none of its rules changes, and the new class is SPEC-426's, in its manifest.
- D9, against: an amendment of the formal-check-settings schematic: rejected, because that schematic draws the settings file, and the build has its own readers.

## Consequences

- A Lean warning anywhere in the library fails the build, by hand and in the formal check, so a
  delivery that adds a warning cannot land a passing formal reading.
- A pull request that adds a hatch under `formal/lean` is red in the python stage, named by file,
  line and arm, until the hatch is removed or an `ALLOWED` entry, a suppression D6 counts,
  admits it.
- The lakefile is an input of every Lean entry, so this landing restarts each entry's
  clean-landing count. No theorem is at `ramp=block`, so no verdict moves.
- FN2's spellings stay outside the list; the formal checker's axiom audit is what names them for a
  declared theorem.

### Confirmation

SPEC-426 A1 to A9 in CI, and its section 4's local reading in `docs/red-first/SPEC-426.md`.

## What would make this wrong

- D3's head build passing with the plant: the key, spelled so, would not take effect, and D1 would
  be reopened before the delivery lands.
- D3's override reading exiting non-zero: a module could not switch the key off, and D2's
  `key-off` test would guard nothing.
- A Lean release that lexes a comment, a string or a literal differently: the oracle's recorded
  verdicts would no longer be Lean's, and the oracle would be read again with that release.
- Live precision below 0.8 over the last 30 merged pull requests with at least 5 hits.

## More Information

SPEC-426; SPEC-295 and ADR-295 (the settings file and its test module); ADR-305 D1 (the package
requires nothing); the schematic `docs/schematics/lean-warnings-are-errors-and-the-escape-hatch-guard.md`.
