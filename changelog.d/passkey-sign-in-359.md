### Added

- Passkey sign-in on the web, the server side (SPEC-359, #627, part one of two): the owner mints a
  single-use link code from a fresh Telegram session, redeems it in the browser, and registers a
  passkey there; a later sign-in over the owner's own passkeys opens a session of its own. Every
  doubtful assertion is refused by name (another origin, no user verification, a reused, expired
  or foreign ceremony, a signature that does not verify, a counter that does not advance), the
  counter advances only by a compare-and-swap, and removing a passkey from a fresh Telegram session
  ends the sessions it opened. The routes answer `linking_off` until the deployment sets its public
  origin.
