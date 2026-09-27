# DeckStreak: instructions for coding agents

You are probably a DeckStreak builder in a git worktree, working one issue under a SPEC. Read
this file, [CHARTER.md](CHARTER.md) and [docs/CONTEXT-MAP.md](docs/CONTEXT-MAP.md) before you
write code. They are short, they are decided, and re-deriving them is unasked work. If you believe
one is wrong, say so with the measurement that refutes it, and build it as specified anyway.

**Write the files. Do not describe what you would write.** A run that leaves the tree unchanged is
not a delivery.

## The order of work (the sdd pack)

1. **SPEC**: `docs/specs/SPEC-NNN-slug.md`. The problem measured, the requirements, acceptance
   criteria with a fenced `acceptance` block of commands, the file manifest, what it does NOT do
   (each exclusion cites an issue `#N`), and the risks. A wave's SPECs are written ahead of time in
   `docs/specs/planned/`; your delivery moves yours into `docs/specs/` (ADR-016).
2. **SCHEMATIC**: `docs/schematics/<slug>.md`, in mermaid, before the code, when you add or change
   a component, a data flow or a state machine.
3. **ADR**: `docs/decisions/ADR-NNN-slug.md`, one per real decision, and it names what it was
   chosen against. Numbers come from the architect; never pick the next free one yourself.
4. **TESTS, RED FIRST**: write the test the criterion names, run it before the implementation, and
   read WHY it fails. It must fail by assertion for the criterion's reason, not by a missing
   fixture or a compile error. Record it in `docs/red-first/SPEC-NNN.md`:
   `A1: red at <sha>: <failure>` then `A1: green at <sha>`, or `A1: not red: <why>`.
5. **IMPLEMENTATION**, inside the SPEC's manifest. A file the manifest does not name needs a SPEC
   amendment first.
6. **GATE**: `bash scripts/check.sh`, whole, green. A stage that examined nothing is not a pass.

## Boundaries (the ddd pack): the compiler is the boundary

- One crate per bounded context under `crates/`, named `deck-streak-<context>`. The crate graph IS
  the context map, and `scripts/ddd-probe.py` holds them equal in both directions.
- **Never add a dependency edge to a `Cargo.toml` to make code compile.** The edge is the design.
  If your work needs an edge the map lacks, you have put the code in the wrong crate, or found a
  real design question: write it up and stop.
- Domain contexts depend on `kernel` (and `ingest` where they read Anki data), never on each other.
  A use case that crosses contexts lives in `coordination`. A trait of one context implemented for
  a type of another lives in `crates/daemon/src/wiring.rs` behind a newtype.
- One concept has one name ([docs/LEXICON.md](docs/LEXICON.md)). If the schema says `verdict`,
  nothing calls it a result.
- Every table is owned by the context that writes it; register a new table in the context map's
  ownership register in the same change.

## Tests (the tdd pack)

- Assert a positive artifact, never only an absence. Pair an absence census with a planted fixture
  it must refuse.
- Never derive the expected value from the code under test. Game math is proved against the
  parity oracle's goldens (`tools/parity-oracle/goldens/`), which v9's own functions produced.
- Every test that enumerates (a glob, a directory walk) reports `examined N` and refuses zero.
- Name a test for the behaviour it pins, as a sentence: `a_reading_is_granted_xp_once`.
- No test depends on time passing: inject the clock.

## The gate

```sh
bash scripts/check.sh      # fmt · clippy -D warnings · nextest · doctests · web · oracle · packs · secrets
```

All stages, no exceptions, and never `--no-verify`. The same script runs in CI. `scripts/pack-rows.py`
runs every row of every vendored pack (`.packs/`); `.packs/wiring.json` says which packs are
enforced and which wait for a named issue.

## Conventions

- Rust edition and toolchain are pinned (`rust-toolchain.toml`); do not change them to make
  something compile.
- `#![forbid(unsafe_code)]`, `#![deny(unused_must_use)]`, `#![warn(missing_docs, clippy::all)]`,
  and the workspace lint table (pedantic on). Public items get doc comments that say WHY.
- Libraries return `thiserror` types; only the binary uses `anyhow`.
- SQLite: the kernel's repository base sets WAL, foreign keys, the busy timeout and
  `BEGIN IMMEDIATE`. Never open a connection around it. Every table has `created_at`.
- Secrets arrive as systemd credentials (`$CREDENTIALS_DIRECTORY`), never environment variables,
  argv, logs or rows.
- Use Context7 (`resolve-library-id`, then `query-docs`) before writing against any library API.
- Conventional commits, `type(scope): description`, no attribution trailers.
- No `TODO`, `FIXME`, `HACK` or `XXX` in committed code. Resolve it or file an issue.

## This repository is public

- No IP address, hostname that encodes one, cloud project id, secret name, chat id, VM name, host
  path, or personal data (review data, deck or note text, study targets, dates, plans) in any file,
  issue, pull request or commit message. A personal default is configuration with a neutral example
  value. Test fixtures are synthetic.
- No forward-looking date or timeline in any public text or any AI output.
- Private values live outside this repository and reach the VM through the private deploy rail.

## Writing a DeckStreak skill pack (the pack-wave method)

Every DeckStreak pack (an agent duty, an engineering practice or a persona) is built this way, and
so is every future one:

- **R0 research.** WebSearch and Context7 are mandatory. Use primary sources, each with its URL and
  access date, and record every Context7 library id that answered. Note deprecations and their
  dates. For a persona pack, research the subject's pedagogy (for a language: its CEFR descriptors,
  comprehensible-input evidence, its script and pronunciation references, learner-error corpora;
  for law: the bar subject's outline, IRAC and its grading, the Socratic method, issue spotting;
  for the LSAT: the official sections and question types). For a duty pack, research the
  discipline behind the duty (learning science, feedback, assessment design, notification ethics).
- **R1 coverage matrix, in the pack's SPEC.** Every practice maps to a check (blocking or
  advisory) or to an exclusion with its reason, citing a tracked issue `#N`.
- **R2 portable, runnable checks** that validate the agent's OUTPUT or the repository through
  `--root` or `--subject`, each printing an examined count; zero examined is VOID, never a pass.
- **R3 severity**: blocking for facts, privacy and safety (a cited source for law, the CEFR ratio,
  no dates or timelines, the scrubber's deny list); advisory for voice and heuristics.
- **R4 to R6**: existing ids keep their meaning; no new dependency without an ADR; tests red first,
  each check proved by a planted-defect fixture.
- **R8 documents**: a SKILL.md with the exhaustive catalog and exact counts, a `checks.json`, a SPEC
  with the coverage matrix and References, and an ADR naming what it was chosen against.
- **Hand-back**: the coverage numbers and SKILL FEEDBACK.

`.packs/scripts/pack-lint.py` (the pack-authoring pack) judges a pack's shape.
