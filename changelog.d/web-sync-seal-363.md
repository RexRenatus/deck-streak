### Added

- The core decides, in one rule, when the web client keeps a sync login, when it may send its key,
  which answer is the sync server's refusal, when a refusal drops the key, and what a removal does
  to the stored generation. A login in flight across a sign-out is discarded, a failure never drops
  the key, and the generation never wraps (SPEC-363, ADR-374).
- The web engine exports that rule to the browser's Worker, each export calling the core and
  deciding nothing itself.
- The service releases the web client's sealing key for one seal id to the owner's session alone,
  with its own bound of 30 a minute, `Cache-Control: no-store`, the cross-site guard and a 256-byte
  body limit. No seal secret, seal id or released key reaches a log line.
- A TLA+ model of the key's life, with five witnesses, checks the rule's order against a login in
  flight, a sign-out, two Workers, a refusal after a rotation, a restart and the session's end.

### Security

- The release is off, answering 404 `sync_seal_off`, while the service holds no seal secret. A seal
  secret under 32 bytes refuses start by its credential role, `sync-seal-secret`, and an absent one
  turns the release off rather than refusing start. The reader is not yet composed into the running
  API, so the release stays off until a later change composes it.
