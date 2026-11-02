### Added

- The iPhone and iPad app syncs its collection, as the web client does: a normal sync with media
  off behind the endpoint rule and the minimum-client handshake, and a fresh install's first sync
  through the full-sync choice, showing what is not yet synced and what the download loses, then
  backing up and downloading. The host key is read from the app's Keychain item; no password is
  stored (SPEC-503, ADR-521).
