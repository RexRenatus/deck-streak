### Added

- A census counts the repository's stale branches, each one no open pull request names and with
  no commit within its 90-day window parameter, the default, release and protected branches left out, and holds the count
  to a recorded one. Its own workflow reads the branch and pull request lists read-only, on a
  schedule and on pull requests that change it. It starts at warn, refuses an empty list, and
  fails a record the count no longer reaches, so the record moves down (SPEC-674).
