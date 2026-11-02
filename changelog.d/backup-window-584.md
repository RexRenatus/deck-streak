### Changed

- Backups now keep at most seven days (`P7D`), up from three: the declared backups window, the
  privacy page and the records that stated the old window say so, and the copies kept today, a
  replica for 72 hours and daily copies for 3 days, fit inside it. The sealed offsite archives of
  the sync server are deleted within the same seven days instead of thirty. The privacy page also
  discloses two copies the service may keep, daily disk snapshots and a copy of its shared records
  in a second region, each gone within the same window. A new test holds the window, the
  disclosures and every amended record, and refuses any claim that an erase destroys a key before
  the per-learner keys exist (SPEC-584, ADR-603, #44).
