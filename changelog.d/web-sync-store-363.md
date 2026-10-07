### Added

- The web app's Worker keeps the sync key sealed in the browser (SPEC-363 part 2, ADR-374). A sync
  login asks the service for the sealing key first, so with no owner session the password never
  leaves the page; the key is sealed with AES-GCM under that released key, imported so it cannot be
  read out, and bound to the Worker's own origin's sync route, the user and the stored generation.
  Every step reads, asks the core's rule and writes in one transaction of the browser's database.
- The key goes only to the Worker's own origin's sync route. A Worker whose generation is stale
  sends nothing and forgets what it held; a refusal of an older generation keeps the newer key, a
  network failure keeps it, and a record that no longer opens is deleted.
- Two Worker operations, `credential-status` and `credential-forget`, each answer one status word:
  `absent`, `sealed`, `held`, `needs-sign-in` or `offline`. No reply carries the key, the password
  or the sealing key, and only the Worker's modules import the credential module.
- The page's sign-out forgets the sync key in the Worker first, telling every Worker of the origin,
  and then ends the web session, so the key is gone when the device is offline.
- The API role now composes the seal secret it reads, so an owner's session gets the release when
  the secret is configured; with none, the release stays off and the API still starts.
- `PRIVACY.md` says what the web app keeps in the browser, and how signing out and clearing the
  site's data remove it.

### Changed

- The web app's tests gain one development dependency, `fake-indexeddb`, an in-memory browser
  database for the credential store's tests.
