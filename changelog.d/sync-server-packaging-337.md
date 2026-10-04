### Added

- The release builds the sync server (SPEC-337, ADR-347, #617): a step after the tag guard and the
  protobuf compiler reads the fork and the commit from the engine's patch entry, refuses anything
  but a full commit before cargo runs, builds the server with the fork's own lockfile, and the
  tarball carries it as `bin/anki-sync-server`, so the manifest, the digests and the attestation
  cover it.
- Tests that hold the step's order and run its text under bash with cargo stubbed, over a pinned
  manifest, three it must refuse and the tree's own, with mutation rows for the guard, the lockfile,
  the second copy of the commit, the fork and the tarball line.
- The sync server runs as its own unit (SPEC-337, ADR-347, #617): `deck-streak-sync-server.service`
  runs a launcher that reads the owner's and the staging user from the credential socket, refuses
  an entry that is not a user name and a pbkdf2-sha256 hash, clears what the server would read, and
  execs the server on a loopback address with its data in the unit's own state directory.
- DeckStreak's share is 1152M, recorded by amendments of ADR-064 and ADR-032, and its two
  processors are split among the five daemons: the API 75%, the sync server 50%, the bot, the
  replicator and the MCP server 25% each.
- Tests that hold the unit's hardening, its budget entry, the share, its two credentials and the
  absence of any password hash in the tree, and that run the launcher over its starts and its
  refusals with the server stubbed, with mutation rows for each of its checks.
- The sync server is served on the web app's origin under `/anki-sync/` (SPEC-337, ADR-347, #617):
  the bare path redirects to the slash form, the prefix is stripped, the server's health route
  answers 404 at the edge, the request body is bounded at the server's own payload limit and the
  proxy reads with a larger buffer; render-caddy fills a fourth key, the sync server's loopback
  upstream, checked as the API's is and refused under its own name.
- Tests that render the block with the route and run render-caddy over the fourth key, with mutation
  rows for the key, its check, the refusal's name and each line of the route.
- The sync server's cutover runbook, `docs/runbooks/sync-server-cutover.md` (SPEC-337, ADR-347,
  #617): one step per state of the schematic's sequence, the data moved by a final sync and
  desktop's full upload into an empty store, the server's collection read back before any retry,
  each host step on the owner's go (#161), the old server untouched as the rollback, and the
  cutover's full upload held until the sync server's processor share is decided.
- A test that reads the runbook against the schematic's states, their order and each step's go.
