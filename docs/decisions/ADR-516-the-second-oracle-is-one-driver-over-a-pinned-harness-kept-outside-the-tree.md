# ADR-516: The second oracle is one driver over a pinned harness, kept outside the tree

| field | value |
|---|---|
| status | accepted |
| spec | SPEC-498 |
| builds on | ADR-012, ADR-029 (none amended) |
| measured at | `5c44700b` |

## Context

DeckStreak's parity oracle calls its predecessor's Python functions over seeded synthetic inputs and commits what they
return (ADR-012, ADR-029). The engine DeckStreak ingests through today is a second old system, written in Rust and
pinned in the root manifest at `a078820fd4fec3068ed7863344cd50effddabc6b`. Replacing its import, export, rendering,
media, search and review history needs the engine's own outputs, captured once, over inputs nobody owns. This ADR
decides how that capture is driven, where its pieces live, and what keeps it honest.

## D1. One driver over a pinned harness, and what it was chosen against

`tools/parity-oracle/anki/generate.py` is the one driver. It starts a harness, a small program on the engine's API,
and talks to it in JSON lines: a hello that returns the engine's commit and the stock texts, one call per case, and a
bye. The driver writes the goldens in `phx.parity-golden.v1` with closed keys, `kind: engine`, the engine's commit,
its own sha256, the registry module's sha256, `inputs: synthetic`, the corpus's seed and a note naming the function's
id and the interpreter that drew the cases. The driver pins the sha256 of the harness's source tree
(`HARNESS_SOURCE_SHA256`), so `generator_sha256` covers the harness without a key of its own, and refuses an unpinned
or different harness (exit 2). A refusal from the engine is recorded as a short label, never its words. A source the
run could write to is refused before the harness starts (exit 2), and a failed run writes nothing (exit 3).

Chosen against:
- **A Rust generator that writes the goldens itself.** It puts the golden format, the census and the provenance in
  code that only the run can build, so none of it is tested in public CI. The driver keeps them in Python that CI
  tests over a stand-in harness.
- **Two generators, one for rendering and one for import and export.** The owner allowed one sandboxed run; one
  driver, one corpus and one registry make that one session.

## D2. One corpus and one registry, and what it was chosen against

`anki/corpus.py` holds the seed (498), every edge class with what its cases hold, and the builders of
DeckStreak-authored note types, field texts and collections. `anki/registry/spec_498.py` registers 33 functions, each
`{id, classes, note, cases}`, and pins the corpus's sha256, so a corpus edit edits the registry and stales the
goldens it registered. The driver draws each builder twice from `random.Random(SEED)` and refuses a registry whose
declared classes lack a case or whose cases carry an undeclared class, before anything runs. A collection names its
rows by corpus index and a time is a number, so a golden holds no id the engine assigns and no clock of the run.

Chosen against:
- **Classes read off the builders.** Coverage would hold by construction and prove nothing; a declared list checked
  against the drawn cases can fail.
- **A committed corpus file of cases.** The goldens already record every input; a second copy could drift from the
  builders that made it.

## D3. The harness outside the tree, and what it was chosen against

The harness's source is kept in the owner's private evidence, never in this repository, with its sha256 pinned in the
driver. Its manifest has an empty `[workspace]` table and the package name `parity-oracle-anki-harness`. A census
(`workspace_refusals`) holds that no workspace member covers `tools/parity-oracle/anki/harness` or
`tools/render-oracle`, that the root lock names no `parity-oracle-anki-harness`, and that no workflow names either
path, the package or the driver.

Chosen against: **the harness under `tools/parity-oracle/anki/harness/`, outside the workspace.** It would add
engine-API lines to the tree that a later census of the engine's API counts down to zero.

## D4. The one run and the two roles, and what it was chosen against

The run is the owner's or the maintainer's, once, in the sandbox `anki/RUNBOOK.md` builds: no route out, with a
planted connection refused inside and accepted outside; the sources mounted read-only; one writable output folder;
an unprivileged user. The run record and the clean-room record stay in the owner's private evidence. The dirty side
(the harness's author and the run's operator) may read the engine's API and source and writes only the harness and
the run record. The clean side (whoever writes a port that replaces an engine module) reads the documentation, this
tree and the goldens, never the engine's source.

Chosen against: **running the oracle in CI.** CI would fetch and build the engine on every run and widen who reads
its source; the goldens are committed once and CI reads them.

## D5. The stock-text census, and what it was chosen against

The engine adds note types of its own when it makes a collection. The harness reports their texts in its hello and
deletes them before it builds anything. The driver keeps the sha256 of each stripped line of at least 12 characters,
refuses to write any golden that holds one of those lines, and writes only the digests (`stock-lines.sha256`).

Chosen against:
- **Committing the stock texts to check against.** The tree would carry the engine's text.
- **Matching every line or substrings.** A brace or a field name is common to any template; 12 characters keep the
  census to lines that name the engine's text.

## D6. A consumer that can fail, and what it was chosen against

`golden.rs` gains `Mismatch`, `mismatches` (each case whose recorded output differs from the port's) and
`assert_parity` (prints the examined count and panics naming every mismatch). `crates/ingest/tests/oracle_consumer.rs`
runs them over `anki/fixtures/consumer_can_fail.json`, a fixture with one wrong output, and holds that exactly that
case is reported; ingest is where the engine's goldens are first consumed.

Chosen against: **each consumer comparing on its own.** Each would need its own non-vacuity control; one comparison
in the shared reader is proved once.

## D7. A held clock, and a read before the one write, and what it was chosen against

The harness's engine reads the process's clock and zone: an import of a note both sides share by GUID, with no `mod`
on either, decides its updated or duplicate count by which note the clock stamped later, and a due date is counted
from the run's day rather than the case's. Two runs of the same driver, registry and pin could then write different
answers. The harness holds the process's real-time clock at each case's own recorded instant instead: it defines the
C library's real-time entry points itself (`clock_gettime`, `gettimeofday`, `time` and `timespec_get`), which answer
the held instant plus 1 microsecond a read, and it sets that instant before each call from the case's
`collection.now`, else `timing.now`, else `now`. The k-th engine collection a call opens, counted from 0, starts k
held seconds later, so two collections of one call never share the ids the engine takes from its clock. At its hello
the harness refuses to answer unless the standard library, the date library and SQLite each read the held instant
and the local zone is UTC. The driver starts the harness with `TZ=UTC`, and the registry gives a note the package
shares with the target one explicit `mod` on both sides. A run that writes goldens after the first reads before it
writes (SPEC-498 R19): two non-writing passes at the pin, one under a planted host instant, must be byte-equal, and a
semantic gate fixed before the run reads the previous pin's pass against the committed goldens and the new pin's pass
against the previous pin's; a reading off the gate's rule writes nothing.

Chosen against:
- **An LD_PRELOAD shim in production.** It reaches only the symbols a binary imports, and a run that forgets to load
  it answers with the run's clock without a sign. The shim is kept as a test plant only: under it, the harness's own
  definitions must still answer.
- **A third-party fake-time library.** One more dependency to pin, loaded the same way as the shim.
- **One process per case.** About twelve thousand process starts in one run, and the clock would still move inside a
  case.
- **Matching the shift.** Correcting a host-clock answer after the fact needs a model of each engine function; holding
  the clock needs none.
- **Harness arithmetic.** Recomputing dates in the harness re-derives the engine's answers instead of recording them.
- **A per-case zone.** It needs one process per case; one zone for the whole run, UTC, keeps the zone in the driver's
  bytes.
- **No per-collection phase.** Under a 1 microsecond tick a second collection opened in the same call could take the
  first one's clock ids, a regime no earlier run exercised.

## Consequences

- The run that writes the goldens pins the harness in the driver and commits the goldens, the census digests and the
  driver together.
- Every golden of the second old system is read by a Rust test through `mismatches` or `assert_parity`.
- Each engine golden is a function of its input: the harness pin covers the held clock, the driver's bytes the zone,
  and the registry's bytes the explicit mods.
- A run after the first writes goldens only after its non-writing passes and the semantic gate read as the rule
  fixed before it.

## Confirmation

SPEC-498's acceptance fence: the three Python test files beside `test_generate.py` and `test_goldens.py`, and
`cargo test -p deck-streak-ingest --test oracle_consumer`.

A14 in the same fence (the shared-GUID test in `test_anki_registry.py`, the UTC test in `test_anki_driver.py` and
the run-clock test in `test_goldens.py`) confirms D7's input half; A15's run record confirms the read before the
write.

## What would make this wrong

- A harness that answers with ids or clocks of the run: the goldens would not be reproducible. D7 answers it: the
  clock is held at each case's own instant and a re-run reads two passes before it writes. A clock read that
  bypasses the C library would still see the run's clock; R15's test bounds what it could put in a golden.
- An engine pin move without a new run: the goldens would record an engine DeckStreak no longer pins.
