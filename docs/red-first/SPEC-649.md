# SPEC-649: the red-first record

```red-first
```

A1 to A3 were committed with each support module's `stop` keeping the base behaviour it moves out
of `Drop`, which closes the child's standard input, kills the child at once and waits, with every
input kept but the missing grace; on a local run each read red by its first assertion, the planted
child ended by the kill's signal before its exit's work was done.
