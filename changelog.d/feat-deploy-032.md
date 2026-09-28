### Added

- Deploy templates for the API, the bot and the scheduled jobs: hardened systemd units, one timer
  per job of the job table, the Caddy site block with the Mini App's security headers, the settings
  example and DeckStreak's host budget. Each carries neutral values, and none is installed anywhere.

### Changed

- The durable-services and observability packs are enforced in the gate. The rows that wait on the
  backups and on the observability work are deferred to those issues.
