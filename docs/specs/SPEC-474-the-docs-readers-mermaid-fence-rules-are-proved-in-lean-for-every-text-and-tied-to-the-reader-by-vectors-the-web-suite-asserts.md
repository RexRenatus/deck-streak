# SPEC-474: the docs reader's `mermaid` fence rules are proved in Lean for every text, and tied to the reader by vectors the web suite asserts

- **Issue:** #493 (follows #383). **Context(s):** `repo` (`formal/` and `web/app`'s test suite).
- **Decided by:** ADR-491 (this SPEC's own: the statement, its binding and what each was chosen
  against). The reader is SPEC-195's (ADR-195).
- **Status:** delivered. It holds `docs/red-first/SPEC-474.md`.
- **Read at:** every `path:line` below was read at `36c4783a`.

## 1. The problem, measured

The docs check's reader, `readMermaid` in `web/app/src/lib/docs-mermaid-read.js:192`, decides for
any text which `mermaid` fences it reads and which forms it refuses, each by name (SPEC-195 R9). No
machine-checked statement holds that claim, and the repository's formal path cannot name the
reader:

- `git grep -n '@phx covers ' -- formal` lists 254 cover lines, 47 of them in the 14 Lean entries,
  naming 235 `.rs`, 15 `.py` and 4 `.sh` files and no file under `web/`. The shared formal checker
  admits a cover only on those three kinds and fails its whole registry on any other path.
- No file outside `formal/` and `docs/` holds or reads an `@phx` line, no workflow names `formal`,
  and `scripts/tests/test_formal_config.py:52-61` reads only `config/formal.json`'s own fields: no
  check of this repository reads a covered source path.
- Of the 14 files under `formal/vectors/`, 12 are read by a test outside `formal/` (11 Rust tests
  under `crates/*/tests/`, one Python test); no web test reads any.
- The reader's fence rules are lines 286-302: the classification of each fence the parse opened
  (`:286-294`), the rule that names every line that may open a `mermaid` fence and was not read
  (`:295-301`), and the sort (`:302`). Its other rules (characters `:198-201`, depth `:228-230`
  and `:248-250`, raw HTML `:277-285`) and the parse itself (`:158-174`, `:204`) feed it.

## 2. Requirements

R1. **A Lean entry ports the reader's fence rules.** `formal/lean/Formal/DocsMermaidRead.lean`
ports, branch for branch and over text read as characters: the line split at a carriage return
and line feed, a carriage return or a line feed (`docs-mermaid-read.js:197`, `:89`); the opener
pattern, its prefix of blanks, `>` and list markers, then three or more backticks or tildes and an
info string up to the line's end (`:104`, `:58`); the pattern by which an info string may hold the
word, the word in any ASCII letter case or a `&` (`:66`); the pattern of the word alone with blanks
around it (`:69`); the not-blank test (`:291`); the classification (`:286-294`); the opener rule
(`:295-301`); and the sort by line that keeps the order of equal lines (`:302`). The parse's fences
(each a first line, the info string as its line holds it, and a text) and the other rules'
refusals are the reading function's arguments. Its header lines name its vectors file, cite #493
and #383, declare one theorem at `ramp=report` and two witnesses of it; it carries no cover line
(ADR-491 D3).

R2. **The claim holds for every text.** One proposition over a reading function states, for every
text, every list of fences and every list of the other rules' refusals: (1) every line that may
open a `mermaid` fence is the first line of a block read, or is refused at that line as "a line
that may open a `mermaid` fence and is not read as one"; (2) every fence whose info string may hold
the word is read as a block with its own line and text, or refused at its line as "a fence whose
info string is not `mermaid` alone" or as "a `mermaid` block that holds no diagram"; (3) every block
read is a fence whose info string is the word alone and whose text is not blank, at its line and
with its text; (4) every refusal is one of the other rules' or carries one of the three names with
the fact its name states true at its line; (5) every refusal of the other rules is kept; (6) the
refusals are in line order. The theorem
`every_line_that_may_open_a_mermaid_fence_is_read_or_refused_by_name` states it of the port.

R3. **Two witnesses catch a planted fault each.**
`the_reader_without_its_opener_rule_leaves_a_mermaid_line_unnamed` states the claim false of the
port with lines 295-301 removed, at the text whose `mermaid` opener stands inside a tilde fence's
body (the five lines `~~~`, ```` ```mermaid ````, `flowchart TD`, ```` ``` ```` and `~~~`, each
ended by a line feed, with the one tilde fence the parse opens there).
`the_reader_without_its_info_rule_leaves_a_mermaid_fence_unnamed` states it false of the port with
the refusal at line 289 removed, at the text ```` ```Mermaid ````, `flowchart TD`, ```` ``` ````
with its one fence. Each witness is the negation of the theorem's own statement, with the port
renamed.

R4. **The proof has no hole.** No module under `formal/lean` holds the words for a native
evaluation, an unproved step or a new axiom, a comment included. Each theorem and witness depends
only on the axioms `config/formal.json` allows. The formal checker reads the entry clean, both
witnesses caught, and its vectors byte-equal to the writer's output.

R5. **The statement writes its vectors.** `formal/lean/Formal/DocsMermaidReadVectors.lean` and one
arm of `formal/lean/Formal/Vectors.lean`'s `main` print `formal/vectors/docs-mermaid-read.jsonl`:
a header line, `{"schema":"phx.formal.vectors.v1","entry":"DocsMermaidRead","reader":"web/app/src/lib/docs-mermaid-read.js","function":"readMermaid","vectors":841}`,
then one line per text, `{"name":...,"text":...,"blocks":[{"line":...,"source":...}],"refused":[{"line":...,"form":...}]}`,
whose blocks and refusals are the port's reading with no other rules' refusals and with the fences
a CommonMark parse opens in a text with no container (a model in the writer, not a port). The
texts are every text of one to three lines, each line ended by a line feed, drawn from the nine
lines of the table below (819 texts, named `grid.` and their line numbers), then 22 named texts:
`none.empty-text`, `none.prose`, `read.backtick-fence`, `read.tilde-fence`,
`read.blanks-around-the-word`, `read.four-backtick-fence-holds-three`,
`read.three-space-indent-strips-three`, `read.crlf-line-ends`, `read.unclosed-at-the-end`,
`refused.capital-letter`, `refused.word-and-more`, `refused.character-reference`,
`refused.empty-block`, `refused.blank-block`, `refused.four-space-indent`, `refused.tab-indent`,
`refused.backtick-in-a-backtick-info`, `refused.backtick-in-a-tilde-info`,
`witness.opener-in-a-tilde-fence`, `refused.opener-in-a-four-backtick-fence`,
`refused.opener-in-an-unclosed-fence` and `read.and-refused-in-one-text`. The file is written by
the writer and never by hand.

| line | text |
|---|---|
| 1 | ```` ```mermaid ```` |
| 2 | `~~~mermaid` |
| 3 | ```` ```Mermaid ```` |
| 4 | ```` ```text ```` |
| 5 | ```` ``` ```` |
| 6 | `~~~` |
| 7 | four spaces, then ```` ```mermaid ```` |
| 8 | `flowchart TD` |
| 9 | the empty line |

R6. **The web suite answers every vector with the reader itself.**
`web/app/src/lib/docs-mermaid-read.formal.test.ts` finds the repository root by walking up to
`pnpm-workspace.yaml` (as `docs-mermaid.test.ts:19-24` does, so the mutation tool's sandbox finds
it too) and reads the vectors inside the test. It asserts the header, the count 841, and that
`readMermaid(text)` equals each vector's blocks and refusals, listing the first three that differ
with how many differ; then that the vectors hold each of the three names, a read block and a text
with neither. It reports how many vectors it examined and refuses none.

R7. **A planted fault in the reader is caught.** The same file runs the reader without its opener
rule: the reader's reading with every refusal of that name removed, which is the reading of the
reader with lines 295-301 removed, because no other rule writes that name. It asserts that this
reader reads otherwise than the vectors at `witness.opener-in-a-tilde-fence`, and, as a literal
case, that `readMermaid` reads that text as no block and one refusal at line 2 of that name, and
the reader without the rule as no block and no refusal.

R8. **Nothing that exists moves.** The reader, the generator, the recorder, the recorded rendering,
`docs-mermaid.test.ts`, `docs-mermaid.order.test.ts`, `web/app/vite.config.ts`,
`web/app/stryker.config.json`, `config/formal.json`, `formal/lean/lakefile.toml` and
`formal/lean/Formal.lean` are byte-unchanged; SPEC-195's six fence lines pass; and no document this
delivery adds holds a form the docs check refuses.

R9. **The formal decision.** Lean is required (a pure function that decides the docs check's
verdict, for every text); TLA+ is not applicable (one pure function of one text, no actors over
shared state).

## 3. Acceptance criteria of SPEC-474

| id | criterion | decided by |
|---|---|---|
| A1 | the entry builds; its theorem holds of the port and each witness holds of the port with one rule removed; each depends only on the allowed axioms; the formal checker reads it clean, both witnesses caught, vectors byte-equal | `lake build Formal.DocsMermaidRead` from `formal/lean`, and the formal checker's card for `lean/DocsMermaidRead` |
| A2 | the web suite reads the 841 vectors and `readMermaid` answers each with the statement's reading, the vectors holding every class | `docs-mermaid-read.formal.test.ts` `answers every vector its Lean statement writes` |
| A3 | the reader without its opener rule reads otherwise than the vectors at the first witness's text, and the literal case reads as R7 states | `docs-mermaid-read.formal.test.ts` `once its opener rule is removed` |

```acceptance
A1: cd formal/lean && lake build Formal.DocsMermaidRead
A2: pnpm exec vitest run web/app/src/lib/docs-mermaid-read.formal.test.ts -t "answers every vector its Lean statement writes"
A3: pnpm exec vitest run web/app/src/lib/docs-mermaid-read.formal.test.ts -t "once its opener rule is removed"
```

Each fence line selects one test or one module, and no test is selected by two lines. The formal
checker's card is A1's second half: the repository's CI has no formal job, so the builder reads it
before the push, and it is read again when the pull request lands.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-474-the-docs-readers-mermaid-fence-rules-are-proved-in-lean-for-every-text-and-tied-to-the-reader-by-vectors-the-web-suite-asserts.md` | docs | added: this SPEC |
| `docs/decisions/ADR-491-a-lean-entry-states-the-docs-readers-fence-rules-over-every-parse-and-a-vitest-test-binds-it-to-the-reader-by-its-vectors.md` | docs | added: ADR-491 |
| `docs/schematics/docs-mermaid-reader-and-its-lean-statement.md` | docs | added: the data flow from a text to each fence read or refused, with the statement beside it |
| `docs/red-first/SPEC-474.md` | docs | added: the red-first record |
| `changelog.d/web-reader-formal-path-474.md` | repo | added: the fragment |
| `formal/lean/Formal/DocsMermaidRead.lean` | formal | added: R1 to R4 |
| `formal/lean/Formal/DocsMermaidReadVectors.lean` | formal | added: R5's writer and its model of the parse |
| `formal/lean/Formal/Vectors.lean` | formal | changed: one import and one arm of `main`, each in sorted place |
| `formal/vectors/docs-mermaid-read.jsonl` | formal | added: the header and 841 lines the writer prints |
| `web/app/src/lib/docs-mermaid-read.formal.test.ts` | web | added: R6 and R7 |

## 5. What this does NOT cover

- The web app's other pure readers: #493's acceptance names one reader, and each further reader is
  its own statement (#493).
- The docs check's other rules, the characters, the depth and nesting bounds and raw HTML: their
  refusals are the reading function's arguments, and SPEC-195's generated population judges them
  (#383).
- Texts with a block quote or a list item: the writer's model opens fences only in a text with no
  container, and SPEC-195's generated population judges the container forms (#383).
- The diagram parser: whether a block read parses is SPEC-195's A1 (#383).
- A cover line on `docs-mermaid-read.js`, and a formal path for any other source language: the
  shared formal checker admits a cover only on `.rs`, `.py` and `.sh` files, a change outside this
  repository (#963).
- A formal job in CI: the formal checker is run by the builder and at land (#806).
- Anything only a later CI run can measure, such as the pull request's own `web` job over the new
  test (#493).

## 6. Risks

- **The writer's model opens a fence the parse does not.** The web test then names the vector, the
  reader's reading and the statement's; the model or the port is re-read and corrected, and the
  vectors are written again, never edited.
- **The pinned parser changes how it opens a fence.** The vectors test fails on the texts whose
  fences moved, which is the binding working; the parser's pin moves only by a reviewed change
  (SPEC-195 R3).
- **The shared checker refuses an entry with no cover line, or the vectors' header.** The builder
  reads the checker's registry and card before the push, and a refusal stops the delivery with its
  exact text; it is never worked around.
- **A behaviour-preserving edit to the reader.** It leaves the port unread and fails nothing, until
  a cover on the reader is admitted (ADR-491, Consequences).
- **A document this delivery adds trips the docs check.** No line of the SPEC, the ADR, the
  schematic or the red-first record begins, after blanks, `>` or a list marker, with a fence marker
  and an info string that may hold the word, except the schematic's one diagram; SPEC-195's fence
  lines run after each commit.
- **A sibling entry edits `formal/lean/Formal/Vectors.lean`.** Each arm and import is one line in
  sorted place, so two entries' edits merge by keeping both.
- **A census that expects every formal vectors test to be a Rust test.** The build re-measures the
  formal censuses on dev at its cut; a census that refuses a web vectors test stops the delivery
  with its text.

## 7. References

Issue #493; issue #383; ADR-491; SPEC-195 and ADR-195; `formal/lean/Formal/Chest.lean` (a port
whose claim holds for every value of an argument it cannot compute).
