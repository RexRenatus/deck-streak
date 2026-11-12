# CI lock flags: the census's data flow

Decided by ADR-694 for SPEC-671.

```text
.github/workflows/*.yml ──► run_blocks (job scope) ──┐
        │                                            │
        └─► script paths named by run texts          ├─► joined ─► outside_heredocs ─► simple commands
              ├─► *.sh ─► shell_commands (function scope) ─┘                              │
              └─► *.py ─► python_commands (syntax tree, function scope)                   │
                                                                                          ▼
                              tool_words (any position) ─► judge ─► site | clean | none | undecided
                                                                                          │
                                         keyed (file, scope, tool, subcommand, ordinal) ◄─┘
                                                              │
                                     verdict(LISTED, LEVEL) ─► WARN / STALE lines, failures
```

- `population` refuses an empty workflow set (VOID) and every link in a path.
- `judge` holds each tool's lock flag and the forms that are not sites.
- `verdict` fails a STALE entry at both levels and an unlisted site only at `fail`.
