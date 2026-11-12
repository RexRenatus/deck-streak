### Changed

- The append-only check now also holds `freeze_events`, `sync_runs`, `notification_decisions` and
  `agent_runs`: each is written only by inserts, its owner's erase and, for `agent_runs`, its
  retention prune, so any other update or delete of their rows fails the check by name (SPEC-432,
  #895).
