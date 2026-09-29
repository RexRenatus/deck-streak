### Added

- The owner's `/sync` is served by the sync job. The bot stores the request and touches a request
  file, a path unit (`deck-streak-job@sync` (path unit)) starts the sync job on a change of that file, and
  the job runs the owner's cycle, with the five-minute reuse window, before its scheduled run. The
  owner is answered with the outcome, or told the sync is still running once the bounded wait ends.
  A sync needs more memory than the bot's limit, so the bot no longer runs one.
- A tmpfiles snippet creates the request directory, which only the service user owns and only the
  bot unit may write.

### Changed

- The bot unit no longer loads the sync login. Requests during a running sync start at most one more
  run, and the job table keeps its one scheduled sync per study day.
- The bot flushes its notification router when it observes the owner's sync succeed, as it did when it
  ran the sync itself.
