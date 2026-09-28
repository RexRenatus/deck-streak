### Added

- The API has an availability objective sized for one owner's traffic, with burn-rate alerts: a
  page when the error budget burns fast over hours, and a ticket when it burns slowly over days.
- One alert path for every failure: any service, scheduled job, the SLO evaluator or the memory watch
  that fails sends the owner one Telegram message naming the unit, its result and its last error
  lines. The bot token never appears on a command line.
- A memory watch pages on a new out-of-memory kill or a unit reaching its memory ceiling, and logs
  throttling and usage past 90% of the ceiling. Each page is sent once per event or episode.

### Changed

- A panic in any role is logged as one structured error event, with registered secrets redacted,
  instead of plain text on standard error.
- The observability pack's rows that waited for this work are enforced in the gate, except the one
  that asks the bot for an SLO, which waits until the bot's traffic is measured.
