### Added

- The web app's Worker syncs the collection itself (SPEC-364 part b2, ADR-375). A sync login sends
  the sync user and password through the engine to the sync server and keeps only the host key
  it answers, sealed in the browser; a normal sync sends that key and answers which sync the
  collection needs next. Two Worker operations, `sync-login` and `sync`, each answer a status word,
  and no reply carries the key, the password or the host key.
- The engine reaches the sync server through two exports of its own beside the study calls, each
  pinned to its engine method; a normal sync from the browser asks for no media.
- A server's refusal drops the key, so the owner is asked to sign in again; a lost network keeps
  it. A redirected answer is refused and nothing is synced. Every send settles, and a sync runs on
  the session's queue.
- Every session start asks the browser to keep the app's storage.
- The browser tests run a login and a normal sync against the engine's own sync server, built from
  the workspace and reached through the page's own origin.

### Changed

- The engine pin moves to a new commit of the fork that adds three patches for the browser: the
  sync request is a synchronous request from the Worker, a sync on the browser starts no timer and
  no thread, and the sync reads the collection's size from its database instead of the file
  system. The native engine is unchanged (ADR-058's note).
