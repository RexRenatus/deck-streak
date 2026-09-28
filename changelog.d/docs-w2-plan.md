### Added

- The first deploy (W2) is specified behind the owner's gates, as six planned SPECs, six proposed
  ADRs and one schematic (`docs/schematics/first-deploy-and-gates.md`).
  - SPEC-060: the host is inventoried and snapshotted before anything changes, and nothing on it is
    deleted without the owner's approval of that exact item (ADR-060).
  - SPEC-061: the private rail provisions every credential, setting and guard the public units need,
    with host values applied as drop-ins (ADR-061).
  - SPEC-062: DeckStreak's first tagged release is installed by one command and served from its own
    Caddy site block over HTTPS, once every host finding tracked privately under gate 8 is closed,
    and a deploy installs only a release whose provenance and digests verify (ADR-062).
  - SPEC-063: with the owner's route enabled, the agent reaches the proxy over a supervised reverse
    tunnel, with Claude Code from the vendor's signed package at one held version (ADR-063).
  - SPEC-064: DeckStreak's database is replicated, copied daily and restored weekly by its own units
    (ADR-064).
  - SPEC-065: the readings folder exists with group write, and every other writer is fenced from it
    before DeckStreak writes there (ADR-065).
- RELEASING.md says what to do when a hosted runner shuts down in the middle of a mutation shard:
  re-run the failed jobs, never the whole workflow, and never merge while a shard is missing.

### Changed

- SPEC-053's R8 cites ADR-065 for how the owner's go makes DeckStreak the readings folder's one
  writer, with a section 7 bullet, and ADR-053 carries the matching note.
- ADR-032 carries a note of the proposed ADR-061, which fills the Caddy block's placeholders by
  rendering the block at install.
- ADR-058's Confirmation records CI's warm path on `dev` after the engine's pin: the gate at 147
  to 161 s on warm pushes, against 176 s before the fork.

### Fixed

- SPEC-039's section 8 is corrected insert-only: 2,154 mutants at the battery's commit, the
  projection's direction, an `ingest` mutant's 126 s, and a squash merge's push, which is judged
  again on its first-parent diff; ADR-057 notes that its Confirmation's range is A1 to A40.
