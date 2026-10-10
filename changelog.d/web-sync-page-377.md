### Added

- The web client runs a full sync in the browser after the owner's choice (SPEC-377 part c1,
  ADR-388). The engine reads and writes the collection for an upload or a download through its
  database, where the browser gives it no file system; the native engine is unchanged (ADR-058's
  note).
- A sync screen at `/sync` signs in once and keeps neither the user nor the password, shows the
  sync's status in the owner's words and the reviews waiting to sync, offline too, and signs out.
- When a sync needs a full sync, the choice screen counts what each offered direction replaces,
  preselects neither, and asks for one tap to confirm; every step before the write can be
  cancelled. An upload waits until the service has a snapshot of the server's copy, and a server
  that changed since the count returns to the counts with a sentence saying why.
- A collection the browser lost is offered the download alone, never an upload over the server's
  copy, and the sync screen says so and restores it by a download the owner taps.
- A study session syncs once when it starts and once when it ends.
