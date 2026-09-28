### Added

- Scheduled jobs run as systemd timers through one runner, `deckstreakd job <id>`, over one job
  table: the daily `sync` at the rollover hour, minute 7, the daily `maintenance` at minute 28, and
  the hourly `liveness` watch at minute 14, all clear of the predecessor's minutes while both run
  (SPEC-027, ADR-027).
- The cron-fire ledger (`cron_fires`) claims every daily fire once, even when a timer fires twice,
  records `missed` for a catch-up fire more than six hours late, and releases a claim only after a
  failed delivery; its claim, release and record, and the catch-up decision, equal the predecessor's
  goldens.
- The liveness job pages once when no sync has succeeded for one study day plus the catch-up
  window, and once when the maintenance fire drifts more than 30 minutes off its slot; a failing
  scheduled sync pages on its first failure of an episode, and a repeat only logs.
- The maintenance job prunes ledger rows past 90 days, refreshes the planner's statistics and
  truncates the write-ahead log. The ledger is exempt from export and erase, so an erase can never
  re-arm the double-send guard.
