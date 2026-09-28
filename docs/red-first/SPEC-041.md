# Red-first record: SPEC-041

The re-plan came first (6d9ae40): before any test was written, A1 to A3, which had planned public
tests over the notifications-policy pack's rows, were re-planned in place as tests of DeckStreak's
own behaviour, and the rows moved to §3a, which the box run judges (SPEC-056, ADR-069).

The golden came next (3c05c76): `tools/parity-oracle/registry/spec_041.py` registered
`in_quiet_hours`, and the generator wrote it from the predecessor's own `quiet_hours.py:in_quiet_hours`
at `27ee2bc`. The registry module's sha256 read the same before and after the run, and the
predecessor's checkout was left as it was, with no bytecode written into it. Only the new golden was
written into the tree; every other golden the run regenerated was byte for byte the committed one,
but for one adapter's note naming the interpreter, and none of them was kept.

The tests were committed (04802ca) beside a notifications crate whose public API was in place and
whose behaviour was stubbed: the typed policy wrote back without its ladder, celebration budgets,
streak-break cap and near-miss bounds; the router's pass had a public field; `route` sent every
occasion to its surface with no rule, no claim and no record; `flush` did nothing; the quiet window
was never quiet; the data-rights port declared no table; the feed route served any caller; the bot's
side of the port failed every push; the sync cycle did not call the flush; and the policy file had
no `reading_ready` kind and no deviation. The migration, the register of DeckStreak's own tables,
the ports' registry, the symmetry probe's seeds, `privacy.json` and `PRIVACY.md` were committed with
the tests. Each criterion was run there with the SPEC's own fenced command, selecting one test, and
failed by assertion for its own criterion, not by a compile error, a missing fixture or an empty
selection; A2's red is the compile-fail harness reporting that the planted call compiled.

The implementation followed (b5b66ca), restoring the behaviour the stubs stood in for and adding the
`reading_ready` kind and its deviation. Between the red commit and the green one no test changed.
After green, eight hand-proved rows (7cc649d) held the constants and the unique key cargo-mutants
cannot reach, and four more (a3028ea) the checks inside the occasion's and the dedupe key's
constructors, with a test of the declared tier that kills one of them. All twelve were proved on
the committed tree: every row KILLED, each target restored byte for byte. A test of the flush step
that asserted only an absence gained its positive artifacts (b87946d).

The diff's mutation run then missed six of the mutants it examined. Keys that tell the date
matcher's year check apart (f082254), and tests of the in-app feed's read and of a recap naming both
causes (dd09a39), kill three. The other three were equivalent: the quiet window's `<` after its
equal-bounds return, and two fields of a held row that nothing read after a retry. Refactors removed
them (c5fa6f3, 7591e5b), and a test of a quiet hold given up by failed sends pins the reason the
abandonment now carries. No test changed what an acceptance criterion asserts.

```red-first
A1: red at 04802ca: assertion `left == right` failed: every key of the file, and nothing else; left: the policy written back without ladder, celebration_budgets, streak_break and near_miss, right: the file with them
A1: green at b5b66ca
A2: red at 04802ca: Expected test case to fail to compile, but it succeeded.
A2: green at b5b66ca
A3: red at 04802ca: assertion `left == right` failed: reading_ready is a nudge of tier T2, per study day, behind its own switch; left: None, right: Some((Nudge, [T2], None, PerStudyDay, Some("reading_ready_enabled")))
A3: green at b5b66ca
A4: red at 04802ca: assertion `left == right` failed; left: Sent { surface: Bot, tier: T2 }, right: Deferred { surface: Bot, hold: Quiet }
A4: green at b5b66ca
A5: red at 04802ca: assertion `left == right` failed; left: Sent { surface: Bot, tier: T2 }, right: Deferred { surface: Bot, hold: Quiet }
A5: green at b5b66ca
A6: red at 04802ca: assertion `left == right` failed; left: Sent { surface: Bot, tier: T2 }, right: Withheld { surface: Bot, reason: QuietHours }
A6: green at b5b66ca
A7: red at 04802ca: assertion `left == right` failed; left: (Sent { surface: Bot, tier: T2 }, Sent { surface: MiniApp, tier: T2 }), right: (Sent { surface: Bot, tier: T2 }, Withheld { surface: MiniApp, reason: AlreadyRecorded })
A7: green at b5b66ca
A8: red at 04802ca: assertion `left == right` failed: in_quiet_hours({"end_min":450,"local_minutes":0,"start_min":1380}); left: Bool(false), right: Bool(true)
A8: green at b5b66ca
A9: red at 04802ca: assertion `left == right` failed: sent on the lapse's day 0, 3 and 6; inside the gap on 1 and 2; past the cap on 9 and 20; left: seven Sent, right: Sent, BudgetSpent, BudgetSpent, Sent, Sent, BudgetSpent, BudgetSpent
A9: green at b5b66ca
A10: red at 04802ca: assertion `left == right` failed: a lapse suppresses nudges, not its comeback and not a celebration; left: (Sent, Sent, Sent), right: (Withheld { surface: Bot, reason: Lapse }, Sent, Sent)
A10: green at b5b66ca
A11: red at 04802ca: assertion `left == right` failed; left: Sent { surface: Bot, tier: T2 }, right: Deferred { surface: Bot, hold: Send }
A11: green at b5b66ca
A12: red at 04802ca: assertion `left == right` failed; left: Sent { surface: Bot, tier: T2 }, right: Deferred { surface: Bot, hold: Quiet }
A12: green at b5b66ca
A13: red at 04802ca: assertion `left == right` failed; left: [200, 403, 200], right: [401, 403, 401]
A13: green at b5b66ca
A14: red at 04802ca: assertion `left == right` failed: the router's tables are the owner's data: exported and erased (CHARTER 13); left: [], right: the five tables, each ExportAndErase
A14: green at b5b66ca
```

In A1 the stub policy wrote back four sections short. In A2 the stub pass's public field let the
planted call compile. In A3 the policy file declared no `reading_ready`. In A4, A5, A11 and A12 the
stub router sent a celebration it should have held: at 23:30, and after a failed send. In A6 and A10
it sent a nudge in quiet hours and in a lapse. In A7 it delivered one key on both surfaces. In A8 the
stub window was never quiet. In A9 it sent every comeback, past the cap and inside the gap. In A13
the stub route served the feed to callers with no session. In A14 the stub port declared no table.

## Fix round

The first review planted three defects the tests let through: a pass made by `Default` or kept by
cloning a borrowed one, deliveries around the port that never take a pass, and a comeback's gap
counted in UTC calendar days. The SPEC re-planned A2 and A9 and added A15 first (168e626). Then,
for each fix, the new test was committed with the review's plant as its stub, red by assertion for
its criterion and selecting one test, and the plant was removed in the next commit.

- A2's two new cases, `tests/ui/pass_by_default.rs` and `tests/ui/pass_kept_by_a_clone.rs`, each
  with the refusal its `.stderr` records, were committed with `Pass` deriving `Default` and `Clone`
  (eb2a552), where both compiled. Green at 8cb1b11, without the derives.
- A9's case across the rollover hour runs five hours west of UTC with the owner's quiet window off:
  a comeback at noon on study day D, then one at 03:00 on the third calendar day (study day D+2,
  before the 04:00 rollover) and one at 05:00 (study day D+3). It was committed with the claim's
  study day and the gap both taken from the UTC calendar day of the clock (1e68374). Green at
  5ad0c43.
- A15 was committed with the review's three deliveries around the port in the tree, written so the
  tree still builds (3380d04). Green at a331c8f, without them.

A2's and A9's new cases are not criteria of their own, since the fence above records both
criteria, so their record stands outside the `red-first` fence:

```text
A2 Default and clone cases: red at eb2a552: Expected test case to fail to compile, but it succeeded. (tests/ui/pass_by_default.rs, tests/ui/pass_kept_by_a_clone.rs)
A2 Default and clone cases: green at 8cb1b11
A9 rollover case: red at 1e68374: assertion `left == right` failed: a comeback at noon on day 0; at 03:00 on the third calendar day, before the rollover, the study day is 2 and inside the gap; at 05:00 it is 3 and past it; left: [(0, Sent { surface: Bot, tier: T2 }), (2, Sent { surface: Bot, tier: T2 }), (3, Withheld { surface: Bot, reason: BudgetSpent })], right: [(0, Sent { surface: Bot, tier: T2 }), (2, Withheld { surface: Bot, reason: BudgetSpent }), (3, Sent { surface: Bot, tier: T2 })]
A9 rollover case: green at 5ad0c43
```

A15 is a new criterion:

```red-first
A15: red at 3380d04: assertion `left == right` failed: a delivery goes around the port; left: ["crates/api/src/notifications_routes.rs:44: names api.telegram.org", "crates/api/src/notifications_routes.rs:44: names sendMessage", "crates/daemon/src/role_bot.rs:59: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/role_bot.rs:69: names api.telegram.org", "crates/daemon/src/role_bot.rs:69: names sendMessage"], right: []
A15: green at a331c8f
```

Each plant was then installed again on a331c8f, alone and exactly as the review wrote it, and its
criterion's test run selecting one test, with the file restored byte for byte after:

- `#[derive(Debug, Default)]` on `Pass`: A2 red, `tests/ui/pass_by_default.rs` compiled.
- `#[derive(Debug, Clone)]` on `Pass`: A2 red, `tests/ui/pass_kept_by_a_clone.rs` compiled.
- The claim's study day and the gap from the UTC calendar day: A9 red, with the lines above.
- A celebration through the bot's `send_html` in the bot's role: A15 red, `calls send_html in
  celebrate_around_the_router, not a named call site`.
- A raw request to the Bot API's `sendMessage` in the bot's role, and the same in the API: A15 red
  for each, `names api.telegram.org` and `names sendMessage`.

After A15's green, f89d330 changed one helper of its census: `shipped` reads a test file's inner
extension (`.test`, `.spec`) through `Path::extension` instead of `str::ends_with`, as clippy's
pedantic `case_sensitive_file_extension_comparisons` requires. The same files are left out, and the
census still examines 170 shipped sources; A15's body is unchanged.

The census is test code, which cargo-mutants never mutates, so rows S04113 and S04114 hold its two
refusals (da492cf). Both were proved with `mutation_rows.py prove` on the committed tree at
da492cf, and again after f89d330: each time examined 2, killed 2, survived 0, VOID 0; each control
selected one test and passed, each mutant selected one test and failed, and the target was restored
byte for byte.

DISCLOSURE, A2 (`a_delivery_call_outside_the_router_does_not_compile`): its criterion changed at
168e626 and its body at eb2a552, after its green commit, b5b66ca. The body gained
`cases.compile_fail("tests/ui/pass_by_default.rs");` and
`cases.compile_fail("tests/ui/pass_kept_by_a_clone.rs");`, and the criterion names the three ways a
pass could be had outside the router, its field, `Default` and a clone of a borrowed one, with one
case each. The first case, and the refusal it records, are unchanged.

DISCLOSURE, A9 (`a_comeback_past_the_cap_or_inside_the_gap_is_withheld`): its criterion changed at
168e626 and its body at 1e68374, after its green commit, b5b66ca. The body gained the case across
the rollover hour: a second router on a rule five hours west of UTC, three comebacks and one
assertion. The criterion says the gap is counted in the owner's study days. The seven comebacks on
a UTC rule, and what they assert, are unchanged.

## Second fix round

The second review planted ten deliveries around the port that A15's census let through, and found a
comeback that A9 never exercised. The SPEC re-planned A9 and A15 first (00e8b6d). Then each fix went
red first: its test's new cases were committed, with the review's plants as their text, while the
census's model, or for A9 the router, was still the one the review measured. Each red commit was run
over its whole test file, where only the criterion's own test failed, by assertion, and with the
SPEC's command selecting one test. The next commit made it green.

- The census inside a source (3b34e57, green at 39ad254). Held as text: a send after a field
  compiled for tests alone, the bot's own send named through a variable, and a raw request from a
  new module of the bot; on a tree the test writes for the walker, a shipped module under
  `src/fixtures/`; and the transport's raw `sendDocument` as the seventh named send. The fix leaves
  out only a `#[cfg(test)]` module, any attributes stacked on it passed over; always enters a
  directory under a `src/`; reads every use of the bot's `send_html`; lets a send method be named
  inside the bot's sources only in the named send that makes its request; and counts a raw request
  as a send.
- The census's reach (aa56871, green at cbcaa3e). Held as text: the bot's `edit_html`, a
  fabricated update handed to the bot's command handler, and a raw request built on the bot's
  `DEFAULT_API_URL`; on the walker's tree, a script with no extension and a drop-in of the bot's
  unit. The fix guards `edit_html`, which has no named call site, and the handler, which only the
  long poll uses; refuses `DEFAULT_API_URL` outside the bot's sources; reads a script by its `#!`
  first line and a drop-in (`*.d/*.conf`); and makes `DEFAULT_API_URL` `pub(crate)`, so the bot's
  own test of the unset base URL now reads Telegram's URL as written.
- The Mini App's feed (afb644d, green at 7db0ef0). Held as text: the feed written from the API
  through the ledger's `FEED_TABLE`, from a script in SQL, and from a module beside the router
  through the ledger's `append_feed`. The fix refuses the three names outside the router's ledger,
  router and data-rights modules.
- A9 (2a0f692, green at ef01825): a comeback sent at 03:00 on the calendar day after day 0, before
  the rollover, then one at noon two calendar days later, on the rule five hours west of UTC with
  the owner's quiet window off. It was committed with the claim's study day taken from the UTC
  calendar day of the clock, the review's R20a, as its stub, and the stub was removed in the next
  commit.
- The held queue (9248ba2, green at 40b6028), by the architect's ruling on the round: a write to
  `notification_queue` from outside the router's modules is a delivery around the router, because a
  flush delivers what the queue holds. The SPEC re-planned A15 first (08494fb). Held as text: a
  celebration held on the queue from the flush step's crate through the ledger's `QUEUE_TABLE`,
  from a script in SQL (`Notification_Queue`), from a module beside the router through the
  ledger's own writes (`hold` imported under another name, and `relatch`), and through the
  ledger's `hold` re-exported at the crate's root. The modules that own the queue's writes were
  measured to be the router's three: the ledger holds the SQL of each write, the router is their
  one caller, and the data-rights port exports and erases the queue. The fix refuses the queue's
  table, in any case, and `QUEUE_TABLE` outside those modules; and because the ledger's writes to
  the queue are private to the notifications crate, it refuses the ledger named in that crate's
  sources outside them, the root's declaration of it aside. The feed's planted append from a module
  beside the router is now refused by that name too.

These are new cases of A15 and A9, not criteria of their own, so their record stands outside the
`red-first` fences. Each red is the first assertion of its test that failed; the cases after it in
the same test were not reached at that commit.

```text
A15 inside a source: red at 3b34e57: assertion `left == right` failed: the bot's own send, named or called, raw requests to the Bot API, and a raw request from inside the bot, around the port; left: ["crates/api/src/notifications_routes.rs:3: names api.telegram.org", "crates/api/src/notifications_routes.rs:3: names sendMessage", "crates/daemon/src/role_bot.rs:4: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/role_bot.rs:10: names api.telegram.org", "crates/daemon/src/role_bot.rs:10: names sendMessage"], right: ["crates/api/src/notifications_routes.rs:3: names api.telegram.org", "crates/api/src/notifications_routes.rs:3: names sendMessage", "crates/bot/src/celebrate.rs:5: names sendMessage in celebrate, not a named call site", "crates/daemon/src/role_bot.rs:4: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/role_bot.rs:10: names api.telegram.org", "crates/daemon/src/role_bot.rs:10: names sendMessage", "crates/daemon/src/role_data.rs:3: uses send_html in celebrate_through_a_variable, not a named call site", "crates/daemon/src/role_job.rs:11: calls send_html in celebrate_around_the_router, not a named call site"]
A15 inside a source: green at 39ad254
A15 its reach: red at aa56871: assertion `left == right` failed: the bot's own send, named or called, its edit and its command handler, raw requests to the Bot API and on its base URL, and a raw request from inside the bot, around the port; left: ["crates/api/src/notifications_routes.rs:3: names api.telegram.org", "crates/api/src/notifications_routes.rs:3: names sendMessage", "crates/bot/src/celebrate.rs:5: names sendMessage in celebrate, not a named call site", "crates/daemon/src/role_bot.rs:4: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/role_bot.rs:10: names api.telegram.org", "crates/daemon/src/role_bot.rs:10: names sendMessage", "crates/daemon/src/role_data.rs:3: uses send_html in celebrate_through_a_variable, not a named call site", "crates/daemon/src/role_job.rs:11: calls send_html in celebrate_around_the_router, not a named call site"], right: ["crates/api/src/notifications_routes.rs:3: names api.telegram.org", "crates/api/src/notifications_routes.rs:3: names sendMessage", "crates/bot/src/celebrate.rs:5: names sendMessage in celebrate, not a named call site", "crates/daemon/src/lifecycle.rs:4: calls edit_html in celebrate_by_an_edit, not a named call site", "crates/daemon/src/main.rs:6: calls handle in celebrate_by_a_fabricated_command, not a named call site", "crates/daemon/src/role_bot.rs:4: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/role_bot.rs:10: names api.telegram.org", "crates/daemon/src/role_bot.rs:10: names sendMessage", "crates/daemon/src/role_data.rs:3: uses send_html in celebrate_through_a_variable, not a named call site", "crates/daemon/src/role_job.rs:11: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/wiring.rs:4: names DEFAULT_API_URL"]
A15 its reach: green at cbcaa3e
A15 the feed: red at afb644d: assertion `left == right` failed: the bot's own send, named or called, its edit and its command handler, raw requests to the Bot API and on its base URL, a raw request from inside the bot, and writes to the Mini App's feed, around the port; left: ["crates/api/src/notifications_routes.rs:3: names api.telegram.org", "crates/api/src/notifications_routes.rs:3: names sendMessage", "crates/bot/src/celebrate.rs:5: names sendMessage in celebrate, not a named call site", "crates/daemon/src/lifecycle.rs:4: calls edit_html in celebrate_by_an_edit, not a named call site", "crates/daemon/src/main.rs:6: calls handle in celebrate_by_a_fabricated_command, not a named call site", "crates/daemon/src/role_bot.rs:4: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/role_bot.rs:10: names api.telegram.org", "crates/daemon/src/role_bot.rs:10: names sendMessage", "crates/daemon/src/role_data.rs:3: uses send_html in celebrate_through_a_variable, not a named call site", "crates/daemon/src/role_job.rs:11: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/wiring.rs:4: names DEFAULT_API_URL"], right: ["crates/api/src/notifications_routes.rs:3: names api.telegram.org", "crates/api/src/notifications_routes.rs:3: names sendMessage", "crates/api/src/router.rs:6: names FEED_TABLE", "crates/bot/src/celebrate.rs:5: names sendMessage in celebrate, not a named call site", "crates/daemon/src/lifecycle.rs:4: calls edit_html in celebrate_by_an_edit, not a named call site", "crates/daemon/src/main.rs:6: calls handle in celebrate_by_a_fabricated_command, not a named call site", "crates/daemon/src/role_bot.rs:4: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/role_bot.rs:10: names api.telegram.org", "crates/daemon/src/role_bot.rs:10: names sendMessage", "crates/daemon/src/role_data.rs:3: uses send_html in celebrate_through_a_variable, not a named call site", "crates/daemon/src/role_job.rs:11: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/wiring.rs:4: names DEFAULT_API_URL", "crates/notifications/src/occasion.rs:3: names append_feed", "deploy/scripts/celebrate.py:9: names in_app_feed"]
A15 the feed: green at 7db0ef0
A15 the held queue: red at 9248ba2: assertion `left == right` failed: the bot's own send, named or called, its edit and its command handler, raw requests to the Bot API and on its base URL, a raw request from inside the bot, and writes to the Mini App's feed and to the held queue, around the port; left: ["crates/api/src/notifications_routes.rs:3: names api.telegram.org", "crates/api/src/notifications_routes.rs:3: names sendMessage", "crates/api/src/router.rs:6: names FEED_TABLE", "crates/bot/src/celebrate.rs:5: names sendMessage in celebrate, not a named call site", "crates/daemon/src/lifecycle.rs:4: calls edit_html in celebrate_by_an_edit, not a named call site", "crates/daemon/src/main.rs:6: calls handle in celebrate_by_a_fabricated_command, not a named call site", "crates/daemon/src/role_bot.rs:4: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/role_bot.rs:10: names api.telegram.org", "crates/daemon/src/role_bot.rs:10: names sendMessage", "crates/daemon/src/role_data.rs:3: uses send_html in celebrate_through_a_variable, not a named call site", "crates/daemon/src/role_job.rs:11: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/wiring.rs:4: names DEFAULT_API_URL", "crates/notifications/src/occasion.rs:3: names append_feed", "deploy/scripts/celebrate.py:9: names in_app_feed"], right: ["crates/api/src/notifications_routes.rs:3: names api.telegram.org", "crates/api/src/notifications_routes.rs:3: names sendMessage", "crates/api/src/router.rs:6: names FEED_TABLE", "crates/bot/src/celebrate.rs:5: names sendMessage in celebrate, not a named call site", "crates/coordination/src/sync_cycle.rs:6: names QUEUE_TABLE", "crates/daemon/src/lifecycle.rs:4: calls edit_html in celebrate_by_an_edit, not a named call site", "crates/daemon/src/main.rs:6: calls handle in celebrate_by_a_fabricated_command, not a named call site", "crates/daemon/src/role_bot.rs:4: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/role_bot.rs:10: names api.telegram.org", "crates/daemon/src/role_bot.rs:10: names sendMessage", "crates/daemon/src/role_data.rs:3: uses send_html in celebrate_through_a_variable, not a named call site", "crates/daemon/src/role_job.rs:11: calls send_html in celebrate_around_the_router, not a named call site", "crates/daemon/src/wiring.rs:4: names DEFAULT_API_URL", "crates/notifications/src/lib.rs:6: names ledger", "crates/notifications/src/occasion.rs:3: names append_feed", "crates/notifications/src/occasion.rs:3: names ledger", "crates/notifications/src/quiet.rs:1: names ledger", "crates/notifications/src/quiet.rs:7: names ledger", "deploy/scripts/celebrate.py:9: names in_app_feed", "deploy/scripts/hold.py:9: names notification_queue"]
A15 the held queue: green at 40b6028
A9 a comeback sent before the rollover: red at 2a0f692: assertion `left == right` failed: a comeback sent at 03:00 on the calendar day after day 0, before the rollover, is on study day 0, so one at noon two calendar days later is 3 study days after it and past the gap; left: [(0, Sent { surface: Bot, tier: T2 }), (3, Withheld { surface: Bot, reason: BudgetSpent })], right: [(0, Sent { surface: Bot, tier: T2 }), (3, Sent { surface: Bot, tier: T2 })]
A9 a comeback sent before the rollover: green at ef01825
```

Each of the review's plants was then installed again on ef01825, alone and exactly as the review
wrote it, and the notifications suite run (46 tests without the compile-fail test for A15's plants,
47 with it for A2's and A9's), every file restored byte for byte after:

- the ten plants the census had let through, each now refused by A15:
  - the bot's own send through a variable: `uses send_html in celebrate_through_a_variable, not a
    named call site`;
  - a request on `DEFAULT_API_URL` with its method assembled from pieces: `names DEFAULT_API_URL`;
  - a script with no extension, and a drop-in of the bot's unit: `names api.telegram.org` and
    `names sendMessage` in each;
  - a new module of the bot posting to `sendMessage`: `names sendMessage in celebrate, not a named
    call site`;
  - a send after a field compiled for tests alone: `calls send_html in
    celebrate_around_the_router, not a named call site`;
  - a shipped module under `src/fixtures/` calling the bot's send: the same refusal, at its path;
  - the bot's edit: `calls edit_html in celebrate_by_an_edit, not a named call site`;
  - an insert into the feed from the API: `names FEED_TABLE`;
  - a fabricated command handed to the bot's handler: `calls handle in
    celebrate_by_a_fabricated_command, not a named call site`;
- all ten at once: A15 red, with twelve refusals;
- the plants the tests already refused (the pass's two derives, the first review's three
  deliveries around the port, a `.sh` outside the alert path, a new unit, a new crate, a web
  source): each still red;
- R20a alone, the claim's study day from the UTC calendar day: A9 red by the new case, `left:
  [(0, Sent { surface: Bot, tier: T2 }), (3, Withheld { surface: Bot, reason: BudgetSpent })]`;
- R20b alone, the gap from the UTC calendar day: A9 red by the first rollover case, `left:
  [(0, Sent { surface: Bot, tier: T2 }), (2, Sent { surface: Bot, tier: T2 }), (3, Withheld {
  surface: Bot, reason: BudgetSpent })]`.

On 40b6028, two writes to the queue were planted in the tree at once, the notifications suite's
A15 run, and the files restored byte for byte after: a new module of the flush step's crate, never
declared, that inserts into `notification_queue`, and the ledger's `hold` re-exported in a module
beside the router. A15 was red at its assertion over the tree, `left:
["crates/coordination/src/hold_plant.rs:5: names notification_queue",
"crates/notifications/src/quiet.rs:80: names ledger"]`.

The census is test code, which cargo-mutants never mutates, so each of its new refusals takes a
row: S04115 to S04126 (5880072), and S04113 and S04114 were re-anchored to the lines that now hold
their refusals. All fourteen were proved with `mutation_rows.py prove` on the committed tree at
5880072: examined 14, killed 14, survived 0, VOID 0; each control selected one test and passed, each
mutant selected one test and failed, and the target was restored byte for byte. In a clean checkout
the census examines 171 shipped sources: the first fix round's 170, and the journald drop-in it now
reads. S04127 holds the held queue's refusal (307593d), and was proved with `mutation_rows.py prove
--row` on the committed tree at 307593d: examined 1, killed 1, survived 0, VOID 0; its control
selected one test and passed, its mutant selected one test and failed, and the target was restored
byte for byte.

DISCLOSURE, A15 (`no_delivery_goes_around_the_port`): its criterion changed at 00e8b6d and 08494fb
and its body at 3b34e57, aa56871, afb644d and 9248ba2, after its green commit, a331c8f. The body
gained the second review's planted cases, each with its expected refusal, the walker's tree and its
two assertions, the seventh named send, and the held queue's planted cases with theirs; the second
review's planted append to the feed gained a refusal, of the ledger's name. The census's helpers
changed at 39ad254, cbcaa3e, 7db0ef0 and 40b6028, into the model the criterion now describes, and
its module doc at 3ed6729 and 07d05d1. The first review's planted cases, their refusals, and the
assertions over the tree are unchanged.

DISCLOSURE, A9 (`a_comeback_past_the_cap_or_inside_the_gap_is_withheld`): its criterion changed at
00e8b6d and its body at 2a0f692, after its green commits, b5b66ca and 5ad0c43. The body gained the
case of a comeback sent before the rollover: a third router on the rule five hours west of UTC, two
comebacks and one assertion. The earlier cases, and what they assert, are unchanged.
