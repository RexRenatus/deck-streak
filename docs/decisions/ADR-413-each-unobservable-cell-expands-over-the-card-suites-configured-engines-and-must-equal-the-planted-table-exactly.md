---
status: accepted
decision-makers: "the DeckStreak architect"
---

# Each unobservable cell expands over the card suite's configured engines and must equal the planted table exactly, both ways

## Context and Problem Statement

SPEC-341 R8 makes the planted suite's declared UNOBSERVABLE table equal the measured set, and
`web/app/tests-card/card.spec.ts` holds that per engine (its blind check, from line 44). Section 3
of `docs/schematics/card-frame-channels.md` names the same set in its cells, in two spellings: two
every-engine cells (`UNOBSERVABLE:`, lines 107 and 118) and five cells that name engines
(`UNOBSERVABLE in <Engine>,`, lines 105, 106, 108, 115 and 124; two of them name two engines). The
coverage test, `web/app/src/lib/card/planted-coverage.test.ts`, reads only the first spelling (line
128) and only one way (lines 130-133), over an engine list kept by hand (line 19). Issue #661 asks
for both spellings, both ways, with the examined count printed and a red seen first on a planted
mismatch. Every line number here is read at `dev` cfa7279991a9; SPEC-399 section 1 holds the
commands.

## Decision Drivers

- The design record (section 3) and the suite's declaration (`web/app/tests-card/planted.ts`) must
  not drift apart unseen, in either direction or for any engine.
- One source of engines: the card suite's configuration, whose project name the suite itself reads
  as the engine (`card.spec.ts` line 40).
- A cell the parse cannot read is refused, never skipped.
- Red first on a planted mismatch, never on an edit to a shipped file.
- The smallest change: one unit-test file and its documents.

## Decisions, and the alternatives each was chosen against

### D1. The population is section 3's channel rows, the declared table and the present checks; the difference is empty, so no side is corrected

The population is section 3's 29 channel rows (lines 97-125), every cell of each; the seven
UNOBSERVABLE cells among them; the thirteen declared pairs (`planted.ts` lines 207-219); and the
coverage test's present check, six one-way inclusions (lines 126-133) plus the duplicate and
membership checks on the declared side (lines 134-141). Expanded over the three configured engines,
the seven cells name exactly the thirteen declared pairs, so the delivery edits neither section 3
nor `planted.ts`. It adds the hold.

Chosen against:

- Correcting a side first: rejected because the two sides agree at the base, thirteen pairs each, so an edit to either would change a measured fact in order to prove a test.
- Moving the engine-named notes into a column of their own: rejected because it rewrites the 29 rows of a table that open work also edits, for a parse the present spelling already supports.
- Reading only the last cell, the one the test keeps today: rejected because a note written in another cell of the row would then be skipped silently.

### D2. The parse reads two spellings only, takes its engines from the configuration's text, and requires exact equality both ways

Each `UNOBSERVABLE` token in a channel row, counted case-insensitively, must be `UNOBSERVABLE:`,
which yields one pair for every configured engine, or `UNOBSERVABLE in <Name>` followed by a comma
or a closing parenthesis, which yields one pair for the configured engine whose project name equals
`<Name>` case-folded. Any other token, or a name no project carries, is refused with its row's id.
The engines are the literal `name` of each project object in the `projects` array of
`web/app/playwright.card.config.ts`, read as text, and the test's engine list is that list. Pairs
are keyed `<engine> <id>`, as line 140 already spells them. The test asserts the refusals empty,
then the two one-way differences empty (missing: a named pair not declared; extra: a declared pair
no cell names), so a failure names the pair, then the two sorted lists equal, so a pair held twice
on either side fails too.

Chosen against:

- A one-way subset check, the present form at line 132: rejected because a declared pair that no cell names, the drift in the other direction, passes it.
- A looser pattern that reads `UNOBSERVABLE` anywhere as every engine: rejected because it turns each engine-named cell into an every-engine claim and demands declarations the engines measure as observable, which `card.spec.ts` then refuses.
- A looser pattern that reads `in <word>` with no terminator: rejected because `UNOBSERVABLE in Chromium and WebKit` would read as one engine and drop the other pair unseen; the terminator makes it a refusal.
- Comparing counts only: rejected because thirteen against thirteen passes when one pair is swapped for another.
- Set differences without the sorted-list equality: rejected because a pair named twice in section 3 would pass them, and the present duplicate check (line 141) covers only the declared side.
- Keeping the engine list by hand (line 19): rejected because a configured engine the list lacks is never expanded, which is the drift this issue closes.
- Importing the configuration module into the unit test: rejected because it runs the configuration's code (its framework import, its device presets, an environment read) inside the unit-test process to learn literal names its text already holds, and the test reads its other sources as text (lines 40, 97 and 146).

### D3. Red first on a planted mismatch, read locally over a planted copy of the old parse; the count lines through `examined()`; no mutation row

The first commit holds SPEC-399 A1 to A3 with their helpers holding a planted copy of the present
behaviour: the every-engine-only parse of line 128 and the hand-kept engines of line 19. A2's first
assertion is a planted mismatch in test-local fixture text: seven rows copied from section 3 and a
copied table without the `webkit prefetch` pair. Over the copy it reads red by assertion; A1 reads
red on the real tree's named-engine pairs, and A3 on an altered configuration text. The next
commit replaces the helpers' bodies, and every test of the file reads green. The reds are read from
a local run of the whole file at the first commit, so no red needs CI to be seen. Each count line
is printed by the existing `examined()` helper (lines 32-36), after the judgment it counts for. At
the head the tests run in the `ci` workflow's `web` job, through the `web` stage of
`scripts/check.sh`, under the aggregate `ci` check. No mutation row: the band's tables take script
and cargo killers, the web mutation tool's `mutate` list skips `*.test.*` files, and the mutation
selection reads a test file as no production file, so the diff yields no mutant. A2's plants are
the counterexamples a weakened parse meets, on every run.

Chosen against:

- A red read on an edit to the schematic or `planted.ts`, reverted after: rejected because it writes a false measurement into the history of a shipped file, and #661 asks for a planted mismatch.
- A red read only in CI, with the red commit pushed alone first: rejected because the unit test runs on the builder's tree with the pinned web tools, so a second push buys no evidence.
- A hand row in S39900-S39999: rejected because no table there takes a killer from this runner, and a row on a test file would mutate the proof, not the product.
- A test of the real tree alone: rejected because it is green at the base, so it could never be seen red for its criterion's reason.

### D4. No formal model and no proof

By surface: the change is one unit-test file and documents. The test reads three committed files
and writes nothing. It runs as one process with no second actor, no shared mutable state, and no
check followed by an act on something another actor can change. It adds no product actor, state or
step, and the card frame's behaviour is unchanged. No formal entry covers any file it edits.

Chosen against:

- A TLA+ model of the suite, the schematic and the table: rejected because nothing interleaves; the three files are fixed for the run.
- A Lean proof of the parse: rejected because the claim is about the concrete contents of two committed files, decided over the whole population on every run, and the parse is a test helper, not product logic.

### D5. The delivery cuts after the open work on its files lands, and pushes once

The third engine (#652) and the schematic's other amendments had landed on `dev` before this
delivery's cut, so the cut already holds Firefox as the card configuration's third project, the
schematic through its section 10, and a coverage test with its SPEC-398 engine check. The delivery
cuts from that `dev`, re-measures section 3, `planted.ts` and the configuration at its cut, and
states the cut's counts. The threat-model schematic's card rows cite `policy.js`,
`svelte.config.js`, `csp.test.ts`, `card-frame.test.ts` and `policy.test.ts` only, so no line it
cites moves. One push, because every red is read locally (D3).

Chosen against:

- Cutting before the third engine landed and resolving at merge: rejected because that work edited the same test's engine list and unobservable check, so one delivery would be rebuilt on the other's lines after its review.
- Two pushes: rejected because no red here needs CI to be read (D3).

## Decision Outcome

The coverage test reads every unobservable cell of section 3 in two spellings, expands it over the
engines the card configuration names, and requires the result equal to `planted.ts`'s declared
pairs, both ways, printing what it examined. It was seen red first on planted mismatches over a
planted copy of the old parse.

### Consequences

- Good: an edit to either side, in either direction, for any engine, reads red with the pair named,
  and a configured engine is expanded with no edit to the test.
- Bad: a writer of section 3 spells each note in one of two forms, and a note for two engines is two
  tokens.
- Neutral: the planted suite, its engines and the shipped card frame are unchanged.

### Confirmation

SPEC-399 A1 to A3 and `docs/red-first/SPEC-399.md`.

## What would make this wrong

- If the suite stopped reading the project name as the engine (`card.spec.ts` line 40), the
  expansion would read the wrong list.
- If a cell has to say something neither spelling can carry, the grammar is widened by an ADR, never
  loosened in place.
- If the configuration came to build its projects in code, the text read would refuse it, and an
  import-based read would be reconsidered.

## More Information

#661; SPEC-341 R8 and A8; ADR-352 D7; SPEC-399; the schematic's new last section.
