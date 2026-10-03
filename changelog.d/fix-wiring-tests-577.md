### Fixed

- The daemon library's `wiring::tests` no longer fail on default parallel threads with `the
  database opens: Database(Io(Os { code: 11, kind: WouldBlock ... }))` (issue #577). The tests
  never shared a database path or a lock file: each multi-thread test built a runtime of one worker
  per core of the host, libtest ran them at once, and the SQLite driver's connection thread was
  refused when the process's task budget ran out. Each multi-thread test in the module now runs on
  two workers, a test reads that size back from the runtime, and a census refuses any multi-thread
  attribute in the file that drops the bound (SPEC-325, ADR-326).
