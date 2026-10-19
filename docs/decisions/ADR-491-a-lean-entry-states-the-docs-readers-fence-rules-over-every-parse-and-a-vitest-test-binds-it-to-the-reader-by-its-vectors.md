---
status: accepted
decision-makers: "the DeckStreak architect"
---

# A Lean entry states the docs reader's fence rules over every parse, and a Vitest test binds it to the reader by its vectors

ADR-491. Decides SPEC-474 (#493, which follows #383). Every `path:line` below was read at
`36c4783a`.

## Context and Problem Statement

The docs check (SPEC-195, ADR-195, #383) reads every Markdown file under `docs/` through one pure
reader, `readMermaid` in `web/app/src/lib/docs-mermaid-read.js:192`. Its claim covers every input:
for any text it decides which `mermaid` fences it reads and which forms it refuses, each by name.
#493 asks for a formal path for such a reader: a machine-checked statement of its property, bound
to the reader by vectors the statement writes and the web tests assert, with a witness the check
must catch on a planted fault.

Measured at `36c4783a`:

- The repository's formal entries name their source by `@phx covers` lines. Under `formal/` there
  are 254 such lines (47 in the 14 Lean entries), and they name 235 `.rs`, 15 `.py` and 4 `.sh`
  files and no file under `web/` or `ios/` (`git grep -n '@phx covers ' -- formal`, its third
  field's suffix counted). The shared formal checker, which lives outside this repository, admits a
  cover only on a `.rs`, `.py` or `.sh` file, and any other path fails its whole registry, so a
  cover naming `docs-mermaid-read.js` would void every entry's check.
- No file of this repository outside `formal/` and `docs/` holds an `@phx` line or reads one, and
  no workflow under `.github/workflows/` names `formal` (a `git grep` of each).
  `scripts/tests/test_formal_config.py` reads only `config/formal.json`'s declared fields (its table
  at lines 52-61: `k`, the budgets, the axioms, the signers path, the slot and the toolchain
  identity). So none of this repository's own checks reads a covered source path, and none has to
  change to admit a web one.
- Of the 14 vectors files under `formal/vectors/`, 12 are read by a test outside `formal/`: 11
  Rust tests under `crates/*/tests/formal_vectors_*.rs` and one Python test,
  `scripts/tests/test_mutation_verdict_python_kills.py:416`. The other two are read by no test.
  No web test reads any.
- The mutation row tables take a killer of two kinds only: a cargo test (`MUTATIONS`,
  `CARGO_KILLED_SCRIPT_MUTATIONS`) or a Python test id (`SCRIPT_MUTATIONS`), over the 134 band
  files under `scripts/mutation-rows.d/`. No row names `docs-mermaid-read.js` or any formal vectors
  file. Web code is mutated by StrykerJS (`web/app/stryker.config.json:7-17`, `break` 100 at
  line 18).
- `web/app/src/lib/docs-mermaid.order.test.ts:60` pins the tests of `docs-mermaid.test.ts` at nine,
  each once, and lines 80-85 pin the generated population (8,994 members, 2,934 read, 6,060
  refused, the read set's digest).

## Decision Drivers

- The statement covers every text, not a sample of texts.
- The statement and the code stay tied by a test that runs the reader itself, in the suite that
  already runs on every pull request.
- A planted fault in the reader is caught, by the formal check and by the web test.
- No line of the reader changes, and no figure an existing pin examines moves.
- Nothing the shared formal checker would refuse enters the tree.

## Decision

### D1. The mechanism

`readMermaid(text)` takes a string and returns `{ blocks, refused }`: each block is `{ line,
source }`, the fence's first line counted from 1 and the text the diagram is drawn from; each
refusal is `{ line, form }` (`docs-mermaid-read.js:176-185`, `:192`). In order:

1. It splits the text at each line end, a carriage return and line feed, a carriage return or a
   line feed (`:197`, `LINE_END` at `:89`).
2. It refuses each line that holds a character of its list (`:198-201`, the list at `:23-32`).
3. It parses the text with the pinned CommonMark parser, whose fenced code block start it wraps so
   that each fence the parse opens is recorded with its info string as the line holds it, before
   any trim or decoding (`:158-174`, `:203-204`). The parse, not the reader's own code, finds every
   fence: in block quotes, in list items, after a paragraph, closed or not.
4. Walking the parse, it refuses a block in more than 99 block quotes and list items (`:228-230`)
   and a block whose bound passes its nesting limit (`:248-250`); then each line outside every
   fence's body that may open raw HTML (`:280`) or holds a tag a CommonMark reading may hide
   (`:283`).
5. It classifies each fence the parse opened, in order (`:286-294`). A fence whose info string may
   hold the word (`MERMAID_INFO`, `:66`: the word in any letter case, or a `&`) but is not the word
   alone (`MERMAID_WORD`, `:69`: the word with blanks around it) is refused as "a fence whose info
   string is not `mermaid` alone" (`:289`). A fence whose info string is the word alone is read as
   a block, unless its text holds no character that is not a blank (`:291`), when it is refused as
   "a `mermaid` block that holds no diagram". Any other fence is not the reader's: no block, no
   refusal.
6. It names every line that may open a `mermaid` fence and is not the first line of a block it
   read (`:295-301`): a line the opener pattern matches (`OPENER`, `:104`, built from
   `FENCE_PREFIX`, `:58`: blanks, `>` and list markers, then three or more backticks or tildes and
   an info string) whose info string may hold the word is refused as "a line that may open a
   `mermaid` fence and is not read as one".
7. It sorts the refusals by line, keeping the order of equal lines (`:302`).

So the fences it cannot classify are the parse's to open and the last rule's to name: an unclosed
fence is closed by the parse at the end of the document and classified like any other; a marker
indented four columns or more, a marker inside another fence's body and a backtick fence whose info
string holds a backtick open no fence, and each such line is named by step 6; a marker in a block
quote or a list item is a fence when the parse opens one there, and step 6's prefix admits `>` and
list markers, so a container line no fence opened is named too. The unclosed case is the parse's
behaviour and is not visible in the reader's own lines; the vectors of D2 measure it.

`readMermaid` is pure: the module imports only the parser (`:15`), and lines 1-303 read no file,
clock or network. The two patterns that carry the global flag (`MARKUP`, `:107`; `OPENS`, `:109`)
are used only through `matchAll` and `match`, which keep no state between calls. Whether it is total
over every text rests on the parser and is not measured beyond the tests that run it. Its callers
are the docs check, `docs-mermaid.test.ts:50` and `:55`, and the order guard,
`docs-mermaid.order.test.ts:77`. The generator `docs-mermaid-fences.js:12-22` imports only the
reader's tables, and the recorder `web/app/scripts/record-docs-mermaid-fences.js:24` imports the
generator and never calls the reader (it reads the clock and the network, `:34-41`).

Chosen against:
- A reading in which the reader's own line scan finds the fences: it does not, because step 6
  only names lines the parse did not open as a read block, and every block comes from the parse.

### D2. The proof

A Lean entry, `lean/DocsMermaidRead`, states the reader's fence rules for every text. It ports
steps 1, 5, 6 and 7 branch for branch, over text read as characters: the line split, the opener
pattern with its prefix, the two info-string patterns, the not-blank test, the classification, the
opener rule and the stable sort by line. It does not port the parse or the rules of steps 2 and 4:
the port takes the parse's fences (each a first line, a raw info string and a text) and the other
rules' refusals as arguments, as the chest entry takes the draw's comparison as one, and the claim
holds for every value of them. So the claim does not rest on any fact about the parser.

The claim, one proposition over the reading function, for every text, every list of fences and
every list of the other rules' refusals:

1. every line of the text that may open a `mermaid` fence is the first line of a block read, or is
   refused at that line as "a line that may open a `mermaid` fence and is not read as one";
2. every fence whose info string may hold the word is read as a block with its own line and text,
   or is refused at its line as "a fence whose info string is not `mermaid` alone" or as "a
   `mermaid` block that holds no diagram";
3. every block read is a fence whose info string is the word alone and whose text is not blank, at
   that fence's line and with its text;
4. every refusal is one of the other rules' refusals, or carries one of the three names with the
   fact its name states true at its line;
5. every refusal of the other rules is kept;
6. the refusals are in line order.

The theorem `every_line_that_may_open_a_mermaid_fence_is_read_or_refused_by_name` states the claim
of the port, at `ramp=report`. Two witnesses each state it false of the port with one rule removed,
each at one input the reader meets: without the opener rule, at a text whose `mermaid` opener
stands inside a tilde fence's body (the planted fault #493 names; conjunct 1 fails); without the
info-string rule, at a fence whose info string is `Mermaid` (conjunct 2 fails). The other
conjuncts ride in the same claim, as the chest entry merges a part the code already satisfies.

The vectors are the reading of the port over a fixed population of texts. A support module writes
them: it gives each text the fences a CommonMark parse opens in a text with no container (a model,
not a port, of the parse's fence rules for such a text), and prints each text with its blocks and
refusals. The population is every text of one to three lines drawn from nine lines (an opener of
each kind the reader tells apart, a closer of each kind, an indented opener, a diagram line and a
blank line), 819 texts, and 22 named texts: none (an empty text and prose), read fences (backtick,
tilde, blanks around the word, a four-backtick fence holding a three-backtick line, a three-space
indent, carriage returns, an unclosed fence at the end), refused fences (a capital letter, a second
word, a character reference, an empty and a blank block, a backtick in either fence's info string),
indented markers (four spaces, a tab), markers inside another fence (tilde, four backticks,
unclosed), and one text that reads a block and refuses another. 841 vectors. A Vitest test asserts
each vector against `readMermaid` itself, so a reader that reads or refuses otherwise than the
statement fails it, and a model that opens fences otherwise than the parse fails it too.

Chosen against:
- A Lean port of the CommonMark block parse: it is a dependency's code, not the reader's, many
  times the reader's size, and a proof about a port of it would say nothing the vectors do not.
- A TLA+ model: the reader is one pure function of one text, with no actors over shared state, so
  no interleaving exists to model.
- A property-based test in the web suite: it samples texts and proves nothing for the texts it
  does not draw, and the issue asks for a machine-checked statement.
- A claim over the opener rule alone: it would leave the two fence names unstated, so a reader
  that dropped a fence's name and kept the line's would pass.

### D3. The path and the tests

The statement is bound to `docs-mermaid-read.js` by the vectors test in the web unit suite,
`web/app/src/lib/docs-mermaid-read.formal.test.ts`, and by no cover line. The entry names the
reader and its function in its own text and in its vectors' header, and cites #493 and #383.

None of this repository's own checks reads a covered source path (Context), so the smallest change
here that admits a web source is none. The shared checker's grammar is what refuses one; a cover
on `docs-mermaid-read.js` waits for that grammar to admit a web file, a change outside this
repository that this delivery does not make.

No existing test line changes. The new tests live in a new file, so `docs-mermaid.test.ts` keeps
its nine tests and the order guard's pins (nine tests; 8,994, 2,934 and 6,060; the read digest)
examine what they examined. The reader, the generator, the recorder, the recorded rendering and
both web configurations are unchanged. No row is owed: no production line changes, and no row
table takes a Vitest killer.

Chosen against:
- A cover line naming `docs-mermaid-read.js`: the shared checker fails its whole registry on a
  path that is not `.rs`, `.py` or `.sh`, which would void the check of every entry.
- A digest of `readMermaid`'s lines pinned by the web test: it would fail on every edit to the
  reader, a comment included, stricter than the covered entries' report ramp, and it would be a
  second stale check beside the shared one.
- A Python mirror of the reader, covered as a `.py` file: it would prove a property of a copy no
  check runs, and its tie to the reader would still be a test.
- The new tests inside `docs-mermaid.test.ts`: the order guard pins that file's tests at nine, so
  the count would move and the guard's restatement would be owed.

### D4. Shape

One delivery under SPEC-474 and this ADR, small enough for one builder: a Lean entry and its
writer, one arm in the vector writer's `main`, the vectors, one web test file, the documents and
the fragment. Its row band, S47400 to S47499, stays unused. SPEC-195 and ADR-195 are not amended:
neither states anything about a formal statement, and the delivery changes none of their
requirements, criteria, files or figures.

Chosen against:
- Two deliveries, the entry first and the web test after: the first would land vectors no test
  reads, as two existing vectors files are, and #493's acceptance would wait for the second.
- An insert-only pointer in SPEC-195: it would add no requirement and change no criterion, and
  SPEC-474 already names SPEC-195 as the reader's own.

### D5. Tests and rows

Each criterion is red first, and each red fails every time. The red commit holds the documents,
the web test, the writer, the arm and the entry with its port lacking the opener rule (the first
witness's port), and the vectors that port writes. At it the web test fails by assertion: the
reader refuses lines the vectors do not, and the reader without its opener rule agrees with every
vector, so the planted fault is not caught. The entry at that commit proves the claim false of its
own port, which is the first witness read before the port proves it. The green commit adds the
opener rule to the port, the theorem and both witnesses, and the vectors the full port writes;
every test is then green. No row is owed (D3). The web mutation pass has no changed production file
to mutate, so the delivery measures instead the reader's lines the statement ports with the vectors
test alone; it is a measurement, not a gate. One push: every red is read locally.

Chosen against:
- A red by absence, the test committed before any vectors file: its red would be a missing file,
  a fixture's red and not the criterion's.
- A red read in CI: every red here is measured locally, so a second push would buy nothing.

### D6. Not in this delivery

The web app's other readers, the docs check's other rules (characters, depth, raw HTML), the
diagram parser, a cover for any web or other source, a CI formal job and anything only a later CI
run can measure. SPEC-474's "What this does NOT cover" names each with its issue.

Chosen against:
- One delivery for every pure reader in the web app: #493 asks for one reader's path, and each
  further reader is its own statement.

## What it was chosen against

- D1, a reader whose own line scan finds the fences: the scan only names lines, and every block comes from the parse.
- D2, a Lean port of the CommonMark block parse: it is a dependency's code, many times the reader's size.
- D2, a TLA+ model: the reader is one pure function with no actors over shared state.
- D2, a property-based test: it proves nothing for the texts it does not draw.
- D2, a claim over the opener rule alone: it leaves the two fence names unstated.
- D3, a cover line naming the reader: the shared checker voids its whole registry on a web path.
- D3, a digest of the reader pinned by the web test: it fails on every edit, a comment included, beside the shared stale check.
- D3, a Python mirror covered as `.py`: it proves a property of a copy no check runs.
- D3, the new tests in `docs-mermaid.test.ts`: the order guard pins that file's tests at nine.
- D4, two deliveries: the first would land vectors no test reads.
- D4, a pointer amendment of SPEC-195: it adds no requirement and changes no criterion.
- D5, a red by absence: a missing file is a fixture's red, not the criterion's.
- D5, a red read in CI: every red is measured locally, so a second push buys nothing.
- D6, every pure reader in one delivery: #493 asks for one reader's path.

## Consequences

- Good, because a statement about every text now holds for the docs check's reader, and the web
  suite fails when the reader and the statement disagree on any of 841 texts.
- Good, because the planted fault #493 names is caught twice: the first witness proves the claim
  false of the reader without its opener rule, and the web test reads that reader otherwise than
  the vectors.
- Good, because no reader line, no existing test line and no pinned figure moves.
- Bad, because the binding is by behaviour only: an edit to the reader that changes nothing the
  841 texts show leaves the port unread, and no stale finding names it until the shared checker
  admits a cover on a web file.
- Bad, because the entry has no port the shared checker can derive from a cover, so a later edit to
  a definition the theorem reaches reads as a change of the statement, owed its base form.
- Bad, because the vectors' fences come from a model of the parse, so a text with a container is
  outside the population; the docs check's own generated population still judges those.

## Confirmation

SPEC-474's A1 to A3: the entry builds, proves its theorem and both witnesses, and the formal
checker reads it clean with both witnesses caught and its vectors byte-equal; the web test answers
all 841 vectors; and the reader without its opener rule reads otherwise on them, at the first
witness's text.

## What would make this wrong

- A vector the reader answers otherwise than the port: either the model opens a fence the parse
  does not, or a port step is not the reader's; the test names the vector, and the model or the
  port is re-read, never the vectors edited.
- The shared checker admitting a cover on a web file: the entry then gains a cover on
  `readMermaid`, a strengthening, and the reader's edits are named stale like every other cover's.
- A form in which the parse opens a fence the reader's opener rule would not name, for example a
  container this repository's documents begin to use: the docs check's generated population is the
  evidence there, and such a form would reopen D2's scope.
