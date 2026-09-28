### Added

- The `deckstreakd` binary and its `api` role: `/api/livez` and `/api/readyz` on a loopback
  address, every request under a request id, a trace naming its matched route, sensitive headers
  marked, a panic catcher, a 10-second timeout, a shed past 64 requests in flight and a 2 MiB body
  limit; `READY=1`, the watchdog's heartbeat and `STOPPING=1` for systemd, and a drain on SIGTERM.
- Every role opens the database under an open lock, so two roles starting together on a fresh
  database both start.
