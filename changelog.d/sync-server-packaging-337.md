### Added

- The release builds the sync server (SPEC-337, ADR-347, #617): a step after the tag guard and the
  protobuf compiler reads the fork and the commit from the engine's patch entry, refuses anything
  but a full commit before cargo runs, builds the server with the fork's own lockfile, and the
  tarball carries it as `bin/anki-sync-server`, so the manifest, the digests and the attestation
  cover it.
- Tests that hold the step's order and run its text under bash with cargo stubbed, over a pinned
  manifest, three it must refuse and the tree's own, with mutation rows for the guard, the lockfile,
  the second copy of the commit, the fork and the tarball line.
