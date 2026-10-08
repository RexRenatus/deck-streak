### Added

- The engine core admits the sync login and the normal sync on the web transport, and checks every
  normal sync and every one-way write against the login's one endpoint rule before the engine sees
  it; a normal sync that asks for media is refused until media sync lands (SPEC-364, ADR-375).
- The one-way sync is an exempt write: it needs the owner's tap, refuses to run from the gesture
  alone, and runs only with the full-sync choice's write, from a request the core builds itself.
- One driver both clients call runs the full-sync write in the order the choice checks: it fetches
  the server's copy only into an empty file, backs up the side the write replaces and reads the
  backup's ids from the written file, re-checks the server before an upload, and re-reads the
  device before a download. Nothing it makes is deleted.
- An evicted, empty device is offered the download alone and gets every server review back after
  the owner's tap.
- The web engine names the sync login and the normal sync as its sync calls, and the native
  adapter reads a one-way tap's refusal as needing the choice.

### Changed

- An upload's re-check compares the server copy's ids and one stamp read from the server's synced
  rows (their greatest usn and the collection's schema stamp) in place of the collection's
  modified stamp, which every download moved, so an unchanged server now lets the upload proceed
  and another client's sync or full upload still returns it to the counts.
