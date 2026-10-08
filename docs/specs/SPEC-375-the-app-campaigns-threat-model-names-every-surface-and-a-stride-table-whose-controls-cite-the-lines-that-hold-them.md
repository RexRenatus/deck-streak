# SPEC-375: the app campaign's threat model names every surface and a STRIDE table whose controls cite the lines that hold them

- **Issue:** #653, the app campaign's threat model, which SPEC-341 §5 leaves to it.
  **Context(s):** docs (`docs/schematics/`) and the repository's Python checks (`scripts/`); no
  crate, app or workflow changes.
- **Decided by:** ADR-386 (the threat model is a schematic declared by a neutral trace and held
  line by line by a reader CI runs).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-375.md`. **Mutation band:** S37500-S37599. **Model:** none (ADR-386 D8).

## 1. The problem, measured

Read at dev `6d71bec0`. The campaign has no threat model, and nothing in the tree could read one.

- **The need.** #653 asks for a schematic that declares the threat-model trace and holds a STRIDE
  table for each surface, which the security checks read. SPEC-341 line 185 leaves the campaign's
  threat model to #653, as do SPEC-349 line 144, SPEC-355 line 184, SPEC-361 line 302 and SPEC-350
  line 304.
- **No schematic declares anything.** The first line of each schematic read, 107 of the 108 files
  `git ls-tree --name-only 6d71bec0 docs/schematics/` lists, counted for `---`, reads 0 (the one
  left unread is outside this model).
- **No reader exists.** The tree's only mentions of `threat_model` or `threat-model` are in the
  planned SPEC-134 (lines 33, 66, 80-83, 97, 123, 133-138, 152, 160-163, 189 and 206-209) and in
  ADR-134 (lines 27-35, 61-64 and 78), by `git grep -n -e threat_model -e threat-model 6d71bec0 --
  .` with the files this review does not open excluded at traversal. The schematic readers that do
  exist read one named file each: `web/app/src/lib/card/planted-coverage.test.ts:29` and
  `scripts/tests/test_sync_server_runbook.py:14`.
- **What SPEC-134 planned.** SPEC-134 (planned, #60) puts its model at
  `docs/schematics/threat-model.md` (its R3, line 66) and the reader at `scripts/threat_model.py`
  with `scripts/tests/test_threat_model.py` (R4, line 80; manifest lines 160-163). It keeps the
  trace a named exception until a check reads a neutral declaration (R7, line 97), and keeps a
  foreign check's row identifier out of public text (R10, line 112). ADR-134 lines 27-35 refused
  carrying that identifier.
- **CI already runs every module under `scripts/tests`.** `.github/workflows/ci.yml` line 349, in
  the `hygiene` job (line 296), runs `bash scripts/check.sh python scrub secrets`, and
  `scripts/check.sh` lines 178-196 (`stage_python`) run
  `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`.
- **CI mutates every script.** The Python mutation population is every `scripts/<name>.py`
  (`scripts/mutation_python.py` lines 9-11), each mapped to its test modules in
  `scripts/mutation-python.json`: 12 entries at dev, keys sorted
  (`git show 6d71bec0:scripts/mutation-python.json`, its keys counted), held by
  `scripts/tests/test_mutation_python.py` line 431
  (`test_the_committed_map_passes_and_prints_its_count`). CI's `mutation-python` job (`ci.yml`
  line 529) runs a changed script's mutants with `--failfast` (line 560): a string constant
  becomes empty, an integer grows by one, and comparisons, boolean operators and returns are
  replaced; f-strings and docstrings are skipped (`mutation_python.py` lines 184-200 and 342-358).
- **The controls exist.** Each of the six STRIDE classes has a control at dev, and a test that
  pins it, on each of the five surfaces: the schematic's 34 rows cite 101 lines in 52 files, each
  read at `6d71bec0` (`examined 101 held 101 refused 0 files 52`).

## 2. Requirements

R1. **The model is declared.** The schematic
    `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md`
    is the campaign's threat model: its line 1 is `---`, its front matter carries
    `trace: threat-model`, and a later `---` closes the front matter. No other schematic declares
    the trace.
R2. **Its surfaces are named.** Its `## 2. Surfaces` table, headed `surface | entry point`, names
    five surfaces and cites each one's entry: the iPhone and iPad client; the web client (the page,
    the card frame, the Worker, and the route that releases the sealing key); the sync service; the
    engine and its boundaries; and the build and release lanes.
R3. **Each surface carries a STRIDE table.** Each surface has a section `## <n>. Surface: <name>`,
    its name equal to its row in the surfaces table, holding one table headed
    `id | threat | asset | control | pinned by`. A row's id is a STRIDE initial (`S`, `T`, `R`,
    `I`, `D` or `E`) and a number, unique in the model, and each surface has at least one row of
    each of the six classes. The `control` cell cites the line or lines that are the control, and
    the `pinned by` cell the test line or lines that hold it; the id, threat and asset cells carry
    no backticks. A citation is a backticked `path:line:quote` or `path:first-last:quote`: the path
    is repository-relative, and the quote, which holds no backtick and no `|`, is contained in one
    of the cited lines.
R4. **The model is public text.** It names each threat by its class and the asset it reaches, and
    the control that answers it. It names no host, address, provider, credential or store, states
    no capacity figure, and writes no threat that has no control.
R5. **A reader judges it.** `scripts/threat_model.py`, standard library only, exposes
    `judge(root: Path) -> Report` and `main(argv: list[str] | None = None) -> int`. `Report` is a
    frozen dataclass of `declared: list[str]` (the models' repository-relative paths, sorted),
    `surfaces: list[str]` (in model order), `categories: dict[str, frozenset[str]]` (each
    surface's classes), `rows: list[str]` (the ids, in model order), `citations: int` (every
    citation read, held or not) and `findings: list[str]`.
    - It reads every `docs/schematics/*.md`. A model is a file whose front matter carries
      `trace: threat-model` exactly.
    - In a `control` or `pinned by` cell, a backticked `path:line` or `path:first-last` whose quote
      is absent or empty has no quote, and any other backticked token that is not a citation is
      not a citation.
    - A finding is one line, `<doc>:<line>: <subject>: <reason>`, where `<doc>` is the file's
      repository-relative path. A row's finding names the row's line and its id cell as written; a
      surface's finding names the surface's row in the surfaces table and the subject `surfaces`;
      a section's finding names its heading's line and `sections`; an undeclared document's
      finding names the heading's line and `trace`.
    - The reasons are exactly: `quote not on a cited line: <citation>`; `names no file:
      <citation>` (the path is absent or a directory, a component of it is `.git`, `target` or
      `node_modules`, or it lies under a directory below the root that holds its own `.git`);
      `no quote: <citation>`; `not a citation: <token>`; `the control cites nothing`; `the pin
      cites nothing`; `not a STRIDE id`; `repeats an id`; `surface <name>: no row for <letters>`
      (the missing classes, in STRIDE order); `surface <name>: no table`; `section names no
      declared surface: <name>`; and, for a schematic that declares no trace but holds a table
      with a `control` column under a heading that contains "threat model" or "threat-model" in
      any case (up to the next heading of the same or a higher level), `a control table under a
      threat-model heading, and the document declares no trace`. A citation or token is written
      without its backticks.
    - `main` prints each finding, then `examined <m> model(s), <s> surface(s), <r> row(s), <c>
      citation(s)`. It exits 3 (VOID) when no model is declared or a model declares no surface,
      whatever else it found; otherwise 1 on a finding and 0 on none; and 2 on a usage error.
      `--root` defaults to the repository root.
R6. **Its tests are one module.** `scripts/tests/test_threat_model.py` (unittest; it imports no
    `subprocess`, and calls `main` in-process under `contextlib.redirect_stdout`) runs the reader
    over the real tree, and over planted models it builds per test in a
    `tempfile.TemporaryDirectory` from literal strings in the module; no fixture tree is
    committed. Every enumerating assertion prints its count and refuses an `examined 0`. No test's
    name is a substring of another's, since unittest's `-k` matches a substring of the fully
    qualified name. Each refusal test asserts its whole finding line or lines first, then that the
    same plant, corrected, reads clean with a non-zero examined count.
R7. **Its mutants are named.** `scripts/mutation-python.json` maps `scripts/threat_model.py` to
    `{"dir": "scripts/tests", "modules": ["test_threat_model"]}`, and the band
    `scripts/mutation-rows.d/S37500-S37599.json` holds four hand rows the generator cannot write:
    S37501 (the class alphabet loses R), S37502 (the quote is sought in the whole file), S37503
    (`target` leaves the skipped names) and S37504 (completeness is judged over the model's union,
    not per surface).
R8. **The change is announced.** `changelog.d/campaign-threat-model-375.md` holds `### Added` and
    one bullet naming the model and its reader (SPEC-375, #653).

## 3. Acceptance criteria of SPEC-375

| id | criterion | decided by |
|---|---|---|
| A1 | the campaign's model is the one schematic that declares the trace | `python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_the_campaign_model_is_the_one_declared_model` |
| A2 | every control and pin in the model cites a line that holds, over the model's 34 rows in order | `python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_every_control_cites_a_line_that_holds` |
| A3 | every declared surface has a row for each STRIDE class | `python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_every_surface_has_a_row_for_every_stride_category` |
| A4 | a quote on no cited line is refused, and a range citation holding it reads clean | `python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_citation_whose_quote_moved_is_refused` |
| A5 | a path naming no file (absent, a directory, or under `target`) is refused | `python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_citation_naming_no_file_is_refused` |
| A6 | a row whose control or pin cites nothing, a token that is not a citation, and a citation with no quote are refused | `python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_row_without_a_quoted_citation_is_refused` |
| A7 | a surface missing a class, a declared surface with no table, and a section naming no declared surface are refused | `python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_surface_missing_a_category_is_refused` |
| A8 | an id that is not a STRIDE initial and a number, or that repeats, is refused | `python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_row_id_outside_stride_or_repeated_is_refused` |
| A9 | a control table under a threat-model heading in a schematic that declares no trace is refused | `python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_an_undeclared_control_table_is_refused` |
| A10 | the command exits 0 on a holding model, 1 on a finding and 3 (VOID) with no model, each with its examined line | `python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_the_command_exits_by_its_verdict` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_the_campaign_model_is_the_one_declared_model
A2: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_every_control_cites_a_line_that_holds
A3: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_every_surface_has_a_row_for_every_stride_category
A4: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_citation_whose_quote_moved_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_citation_naming_no_file_is_refused
A6: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_row_without_a_quoted_citation_is_refused
A7: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_surface_missing_a_category_is_refused
A8: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_a_row_id_outside_stride_or_repeated_is_refused
A9: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_an_undeclared_control_table_is_refused
A10: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_the_command_exits_by_its_verdict
```

A1 to A3 run the reader over the tree's own root. A4 to A9 each plant a model in a temporary
directory and assert the whole finding line or lines, then that the same plant, corrected, reads
clean. A10 calls `main` in-process and reads its exit and its examined line.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-375-the-app-campaigns-threat-model-names-every-surface-and-a-stride-table-whose-controls-cite-the-lines-that-hold-them.md` | docs | added: this file |
| `docs/decisions/ADR-386-the-threat-model-is-a-schematic-declared-by-a-neutral-trace-and-held-line-by-line-by-a-reader-ci-runs.md` | docs | added |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | docs | added: the threat model |
| `docs/red-first/SPEC-375.md` | docs | added: the red-first record |
| `scripts/threat_model.py` | checks | added: the reader |
| `scripts/tests/test_threat_model.py` | checks | added: A1 to A10 |
| `scripts/mutation-python.json` | checks | changed: the reader's entry |
| `scripts/mutation-rows.d/S37500-S37599.json` | checks | added: rows S37501 to S37504 |
| `changelog.d/campaign-threat-model-375.md` | release | added |

## 5. What this does NOT cover

- It does not model native push (#640): it is not wired at dev (`crates/push` is named only by its
  own `Cargo.toml`), and its surface joins this model when it is.
- It does not model passkey sign-in on the web (#627): the web has no sign-in ceremony at dev, and
  the server's identity routes belong to the whole-product model (#60).
- It does not model the surfaces that predate the campaign (the Mini App, the bot, and the API
  beyond the route that releases the sealing key): SPEC-134's whole-product model owns them (#60),
  and this reader serves that model unchanged.
- It does not model the host the services run on (#679).
- It does not model card scripts on the web (#651): none run at dev, and the web client's I3 row
  and its E class change when they do.
- It does not pin the web controls in browsers beyond the ones CI runs (#652).
- It does not model remote input (#633).
- It does not model the XP and memory-scheduler crates (#635, #641): they do no I/O and sit behind
  the engine's boundary.
- It does not model the web sync screens still in flight (#631); they join this model by
  amendment.
- It does not model the engine framework build or the tag-caller workflows (`xcframework.yml`,
  `apple-on-tag.yml`), and it does not run the external security checks against this model: the
  campaign's security re-check owns both (#638).

Each omission lets a threat on that surface go without a row here until its issue adds one
(ADR-386 D1, #653).

## 6. Risks

- **Line drift.** An edit above a cited line reddens A2 in the editing delivery's CI, with a
  finding that names the row and the citation. Citing definition lines (signatures, constants and
  test names) keeps the drift rare.
- **A pin is a line, not a proof.** A pin shows that the test exists, not that it asserts the
  control. Each pin was read when the model was written, and the pinned tests' own mutation rows
  hold what they assert.
- **An external check reads another declaration.** A security check outside this repository that
  looks for its own declaration value does not find this model until it accepts the neutral one
  (ADR-386 D9). The reader CI runs is this repository's own.
- **A second declared model.** A later schematic that declares the trace, such as SPEC-134's
  whole-product model, reddens A1 until that delivery amends A1 to name it.
- **A planted model stays planted.** Every planted model lives in a temporary directory, so the
  tree never holds a second declared model by way of a test.
