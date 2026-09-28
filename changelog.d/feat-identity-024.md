### Added

- The owner signs in to the Mini App: `POST /api/session` takes Telegram's launch data, checks its
  `WebAppData` HMAC on the server in constant time, refuses it when it is more than an hour old or
  more than a minute ahead, pins its user to the owner, and opens a session behind a
  `__Host-deckstreak_session` cookie (`Path=/`, `Secure`, `HttpOnly`, `SameSite=Strict`, eight
  hours); `GET /api/me` answers the server's study day to that session alone; `DELETE /api/session`
  ends the session on the server and clears the cookie.
- Sessions live in memory, each id new at every sign-in and kept only as its SHA-256; one ends
  after 30 minutes without a request or eight hours after it began, and at most eight are kept.
- The `api` role reads the owner's user id and the bot token as credentials at start, and refuses
  to start without either, naming the one that is missing.

### Security

- A state change that is cross-site or not JSON is refused, sign-ins are bounded at 30 a minute
  and 16 KiB of body, and neither the launch data, its hash, the bot token nor a session id reaches
  a log line: a refusal is logged by its reason code alone.
