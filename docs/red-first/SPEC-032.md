# Red-first record: SPEC-032

The tests were committed (89b699e) beside template stubs that named each role and nothing else,
holding the two designs the ADRs rejected:
- each unit loaded its credentials with `LoadCredentialEncrypted=` from a store on the host, as
  ADR-010 first decided and ADR-038 superseded;
- the settings example named the bot token as a variable, which ADR-010 rejected.

Beyond those, the timers fired at midnight UTC, the host budget declared the share and no unit, and
the Caddy block served files and nothing else. Each criterion was run there with the SPEC's own
fenced command and failed by assertion for its own reason, not by a compile error, a missing file or
an empty selection:
- A1's lint refused six blocking rows: `resources.budget`, `sandbox.identity`,
  `sandbox.protect-home`, `sandbox.protect-system`, `sandbox.syscall-filter` and `service.restart`;
- A7's test built and read the three stub timers, and the first calendar it compared was wrong.

The implementation followed (c2af48a): the hardened units, the three timers written from the job
table, the Caddy block, the budget, the settings example, `deploy/README.md`, and the wiring that
enforces durable-services and observability with their deferred rows. Every criterion was green
there. The box's web-security expectations (427d26a) change no criterion; SPEC-032 §7 records them.

Three tests beyond the criteria pin the values R1, R2 and R4 name, for the diff-scoped mutation job
SPEC-039 brings:
- `every_service_runs_its_role_with_the_lifecycle_r1_names`;
- `every_service_carries_the_hardening_r2_names`;
- `every_departure_from_an_advisory_is_waived_with_its_why`.

They were written with the criteria, red at 89b699e and green at c2af48a, and carry no line below.

```red-first
A1: red at 89b699e: AssertionError: {'service.restart': [{'detail': 'Restart=n[1248 chars]n'}]} != {} : the durable lint refused the templates (six rows: resources.budget, sandbox.identity, sandbox.protect-home, sandbox.protect-system, sandbox.syscall-filter, service.restart)
A1: green at c2af48a
A2: red at 89b699e: AssertionError: Lists differ: [] != ['deck-streak-api.service', 'deck-streak-bot.service', 'deck-streak-job@.service'] : budget against units
A2: green at c2af48a
A3: red at 89b699e: AssertionError: unexpectedly None : deploy/systemd/deck-streak-api.service has no MemoryMax, so it cannot be shown to fit
A3: green at c2af48a
A4: red at 89b699e: AssertionError: None != "frame-ancestors https://web.telegram.org; object-src 'none'; base-uri 'self'" : the block's Content-Security-Policy
A4: green at c2af48a
A5: red at 89b699e: AssertionError: unexpectedly None : no handle for /api/*
A5: green at c2af48a
A6: not red: the scrub it runs already judged every file, the stubs named no private value, and the planted template was refused before any real template existed; a committed private value would stay a finding of the history scrub, so no stub can plant one, and the test guards every later edit of deploy/
A7: red at 89b699e: assertion `left == right` failed: the sync job's timer against its slot in the table; left: Some(["*-*-* 00:00:00 UTC"]), right: Some(["*-*-* 04:07:00 UTC"])
A7: green at c2af48a
A8: red at 89b699e: AssertionError: <re.Match object; span=(12, 18), match='_TOKEN'> is not None : deploy/deck-streak.env.example:4: TELEGRAM_BOT_TOKEN names a secret
A8: green at c2af48a
A9: red at 89b699e: AssertionError: Lists differ: ['systemd/deck-streak-api.service:9: LoadCredentialEncrypted= is refused; use LoadCredential= (ADR-038)', ...] != [] (six credential lines, each refused)
A9: green at c2af48a
```
