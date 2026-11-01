### Removed

- The install no longer carries any Telegram surface: the bot's unit, the owner's `/sync` doorbell
  and its request directory, the second route's unit and timer, the held flush's credentials
  drop-in, the deploy's restarts of the bot, the Mini App and doorbell settings, and the edge's
  Telegram framing origin. No unit loads a Telegram credential.

### Changed

- With no Telegram credential held, the API starts with Telegram off: the handshake, the launch
  validation and the link-code routes answer 404 `telegram_off`, and the passkey routes answer
  `linking_off`.
- The alert template keeps its name and now writes one line at error priority to the journal,
  naming the failed unit and its result; the SLO alerts route to `journal`.
- The held flush keeps what it holds when no transport is joined.
