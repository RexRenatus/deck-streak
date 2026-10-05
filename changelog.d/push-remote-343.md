### Added

- The push spike's senders (SPEC-343): a new adapter crate, `deck-streak-push`, builds an APNs
  notification over HTTP/2 with its ES256 provider token, and a web push message encrypted as RFC
  8291 says with its VAPID token, and reports each platform's answer as one typed outcome. CI holds
  every request against recording fakes on loopback ports that verify each token and decrypt each
  body with their own decryption. Keys are generated in memory by the tests; nothing composes the
  crate yet (#640). ADR-354 records the choices and what each was chosen against.
- The remote harness (SPEC-343): `/remote` reads the 8BitDo remote as a standard gamepad and as
  Anki's desktop keys, drives a simulated review, holds a screen wake lock while a review is open,
  a gamepad is connected and the page is visible, and logs the last 200 events for the owner's
  first device session (#629).
