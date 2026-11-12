# Red-first record: SPEC-031

The alert-and-SLO schematic was redrawn and the tests committed together (55ac95e), beside stubs
that held the "before" this SPEC replaces:
- the alert script, its unit, the SLO evaluator, the memory watch and their units and timers were
  the observability pack's templates, copied as they stand;
- `deploy/slo.json` held R2's declaration with the 30-day burn rates, 6 and 1, in place of the
  28-day 5.6 and 0.9333;
- the kernel's logging still left Rust's default panic hook in place.

Each criterion was run there with the SPEC's own fenced command. Seven failed by assertion, each for
its own criterion's reason, not by a compile error, a missing file or an empty selection:
- A2's arithmetic refused both alerts' burn rates;
- A3's page went to the chat the template reads from a credential of its own, `telegram-alert-chat`,
  not to the owner's id;
- A4's census found the token in the URL on `curl`'s command line, the template's leak;
- A5's evaluator counted no response, since the template reads fields nested under `fields` and the
  kernel's events are flattened, so it paged nothing while the page window burned;
- A6's watch takes unit names as arguments and read no DeckStreak cgroup, so a new OOM kill paged
  nobody;
- A7 found the evaluator's and the watch's units paging through the template's `deckstreak-alert@`
  rather than DeckStreak's alert template;
- A8's child logged no event for its panic: the default hook wrote the panic, value and all, to
  stderr.

A1 passed at 55ac95e, disclosed below.

The implementation followed (fe74c1e): the kernel's panic hook, DeckStreak's alert script and unit,
the evaluator, the watch, their timers, the declaration's 28-day burn rates, the host budget's three
entries and the wiring's lifted deferrals. Every criterion was green there. Between 55ac95e and
fe74c1e one test file changed and no test changed what it asserts: clippy's pedantic
`map_unwrap_or` asked for a `let ... else` in A1's helper that reads the usage line.

Beyond the criteria, tests pin what R3 to R6 name, written with the criteria and green at fe74c1e:
the page quotes the failed run's last five lines and stays within 3500 bytes cut at a character
boundary; a ticket pages once and reads its own windows; a journal the evaluator cannot read pages
once; an alert burns only past its burn rate, in both windows; a window counts only its own
responses; the fixture holds the API's trace event shape; a new `max` event pages and the 90% line
logs once; a cgroup made since the last run counts every event as new; the fixture's ceilings are the
host budget's; and SPEC-032's template tests extended to the three units.

After the delivery, a514d7d added tests for the paths those left undriven, each green where it was
written and none a criterion's: a JSON error line reaches the page as written, and real `curl` reads
the script's configuration as the tests' parser does; a unit's first sight is its baseline; a watch
that finds no unit pages once; the 90% line is crossed only past it; a unit with no ceiling or
unreadable events is passed over; a declaration or a record the evaluator cannot read, and a missing
state directory; and a response event is read flattened or nested and nothing else.

```red-first
A1: not red: main already installed the kernel's logging as its first statement (SPEC-025, SPEC-027), so every role's first line was already a JSON event with its priority at 55ac95e; the test reads the roles and the jobs from the binary's own usage line, so it guards every role, and a role added later, against a line written before the logging
A2: red at 55ac95e: AssertionError: Lists differ: ['api-availability page 6h/30m: burn rate 6.0 does not spend 0.05 of the budget in 6h (that is 5.6000)', 'api-availability ticket 3d/6h: burn rate 1.0 does not spend 0.1 of the budget in 3d (that is 0.9333)'] != []
A2: green at fe74c1e
A3: red at 55ac95e: AssertionError: '987654321' != '123456789' : the page went to another chat
A3: green at fe74c1e
A4: red at 55ac95e: AssertionError: Lists differ: ['curl: a secret is on its command line'] != []
A4: green at fe74c1e
A5: red at 55ac95e: AssertionError: 0 != 1 : the evaluator did not page when the page window burned
A5: green at fe74c1e
A6: red at 55ac95e: AssertionError: 0 != 1 : a new oom_kill paged nobody
A6: green at fe74c1e
A7: red at 55ac95e: AssertionError: Lists differ: ['deck-streak-memory-watch.service names OnFailure=deckstreak-alert@%n.service, not deck-streak-alert@%n.service', 'deck-streak-slo.service names OnFailure=deckstreak-alert@%n.service, not deck-streak-alert@%n.service'] != []
A7: green at fe74c1e
A8: red at 55ac95e: assertion `left == right` failed: one event must log the panic; left: 0, right: 1 (the default hook wrote the panic to stderr)
A8: green at fe74c1e
```

After `dev` brought SPEC-021's `data` role into this branch (merge 91a2583), A1's loop runs that role
through `data export`, because the role takes a command and a bare `data` is a usage error. A1's
assertions are unchanged; at 91a2583 A1 was red in CI for exactly that reason (`data: exit status: 2`).
