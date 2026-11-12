---
status: accepted
decision-makers: "the DeckStreak architect"
---

# The package import is one allow-listed engine call behind an inner member bound

## Context and Problem Statement

The iPhone and iPad app has no route to a deck without sync. The engine already imports an Anki
deck package (`ImportExportService.ImportAnkiPackage`), but none of its package reads bounds an
inner member's decompressed bytes, and the native adapter admits only the calls its allow-list
holds (ADR-356 D3). SPEC-681 states the requirements and the measurements.

## Decision Drivers

- One engine call per learner act, admitted by both tables and held equal by the parity test.
- A package is untrusted input: no inner member may decompress without a bound, and a refusal
  comes before any row is read or written.
- The native adapter's `EngineRefusal` and its text stay as they are (ADR-356 D3).
- The planned engine removal: the import must leave with the engine without changing the app's
  request or words.

## Decision Outcome, and what each decision was chosen against

### D1. The import is the engine's own package import, through one allow-listed call

The app sends `ImportExportService.ImportAnkiPackage` with the path of its copy of the picked
file and options that keep scheduling. The engine writes the notes and cards.

- Chosen against: a DeckStreak package reader that writes notes itself, because it would rebuild
  the engine's merge rules in a second place that the planned engine removal has not yet designed.
- Chosen against: `ImportCollectionPackage`, because it replaces the whole collection, which an
  import into the learner's decks must never do.

### D2. The pair is service 39, method 2, an ordinary row on the Native column

The engine numbers its backend services in descriptor order, and a backend service answers its own
methods first, so the import is service 39, method 2. The allow-list takes it as
`ImportExportService.ImportAnkiPackage`, and the core's `ORDINARY` table takes it with the Native
column true and the Web column false. The table's doc sentence changes to say the native client
adds notes only by this import.

- Chosen against: an exempt or answered row, because the import is a learner's ordinary act with
  no owner gesture and no answer.
- Chosen against: the Web column as well, because the web client has no file importer.

### D3. One bound of 1 GiB per member, counted as the bytes decompress, before the engine runs

Before the call reaches the engine, the native adapter opens the package and reads every zip entry
through its decompressor, and every entry that opens with the zstd frame magic through a zstd
stream decoder, each under a reader that stops one byte past `1 << 30`. A count past the bound, or
a package it cannot read, refuses the call before the engine runs, so no row is read or written.
The bound equals the one `crates/anki-format/src/opener.rs` holds for a foreign file.

- Chosen against: trusting the sizes the zip declares, because a declared size is the archive's
  own claim and a hostile package can state any number.
- Chosen against: a bound inside the engine, because the engine is pinned and takes no change here.

### D4. The refusal is worded in the engine refusal's message

A refusal is `EngineRefusal::Engine` carrying an encoded `BackendError` whose message holds the
words, which the app already shows as the refusal's sentence.

- Chosen against: a fourth `EngineRefusal` case, because the harness switches over the three
  cases with no default, so a fourth breaks its build, and ADR-356 D3 keeps the refusal unchanged.

## Consequences

- The native client can add notes through this one call, and the table's doc sentence says so.
- Every list that pins the tables carries the new pair, and the parity test holds the two tables
  equal.
- A package past the bound, or one the reader cannot read, is refused in words before the engine
  runs.

## Confirmation

SPEC-681's acceptance criteria, each read by name in this change's CI, and its red-first record.

## What would make this wrong

- An engine release that bounds its own package reads, which would make D3 a second read of the
  same bytes.
- A renumbered engine service, which reds the import's tests by name.
- The engine's removal, which replaces D1 with the replacement's own import under the same request
  and words.

## More Information

SPEC-681; ADR-356 (the per-transport tables); ADR-151 (the package format).
