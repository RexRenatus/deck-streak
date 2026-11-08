# SPEC-649: the red-first record

```red-first
A1: red at e4c03d0d: the stopped child was killed (signal: 9 (SIGKILL)) before its exit's work was done (local run)
A1: green at eac39115
A2: red at e4c03d0d: the stopped child was killed (signal: 9 (SIGKILL)) before its exit's work was done (local run)
A2: green at eac39115
A3: red at e4c03d0d: the stopped child was killed (signal: 9 (SIGKILL)) before its exit's work was done (local run)
A3: green at eac39115
```

A1 to A3 were committed with each support module's `stop` keeping the base behaviour it moves out
of `Drop`, which closes the child's standard input, kills the child at once and waits, with every
input kept but the missing grace; on a local run each read red by its first assertion, the planted
child ended by the kill's signal before its exit's work was done.

The grace replaced the base behaviour in eac39115, which changes only the three support files'
`stop` and adds `STOP_GRACE` and `STOP_POLL`, with every test unchanged, and A1 to A3 read green
there on a local run. No commit between the red commit e4c03d0d and its green changes a test
file, and the commit after eac39115 changes only this record.
