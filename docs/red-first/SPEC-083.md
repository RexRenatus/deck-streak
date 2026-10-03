# Red-first record: SPEC-083

The order of work: SPEC-083 promoted and the recorder's control (3a0961c); the goldens
(f8d584a); the record's tests beside a stub store and stub pure functions, with the two migrations
(2808dca); their implementation, the rights port, the registrations and the rows (8075b52).

The goldens were generated from the predecessor's own functions at `27ee2bc`, and the predecessor's
checkout was left as it was.

The control A33 is disclosed as not red: its red was measured, not committed. With the classifier
stubbed to return nothing, the test failed by the assertion "the classifier marks an upload", on
commit 3a0961c, so every proof that rests on the recorder's zero rests on a layer shown to see a
planted upload.

Each other criterion was run at its red commit with the SPEC's own fenced command, selecting one
test, and failed by assertion for its own criterion. The stubs compiled: a store that recorded
nothing and refused nothing, a day spec of the bounds as given, a search that was not wrapped or
held, a summary of zeros, and a port that declared no skip table. A2's red is the migration
without its key.

```red-first
A33: not red: measured red at 3a0961c with the classifier stubbed to return None: the classifier marks an upload; committed green only
A1: red at 2808dca: a second take for the day is refused: Ok(SkipId(0))
A1: green at 8075b52
A2: red at 2808dca: the key refuses a second pending or applied skip for one study day
A2: green at 8075b52
A3: red at 2808dca: a skip to undo: NothingToUndo
A3: green at 8075b52
A4: red at 2808dca: assertion `left == right` failed: only an applied skip not undone covers its day (R4); left: []
A4: green at 8075b52
A7: red at 2808dca: assertion `left == right` failed; left: "2-2", right: "2"
A7: green at 8075b52
A8: red at 2808dca: assertion `left == right` failed; left: 1, right: 0
A8: green at 8075b52
A23: red at 2808dca: skip_days is the owner's data: exported and erased (CHARTER 13); left: None
A23: green at 8075b52
```

## The record's second half: the preview, the tariff, its refund and the game (#108)

A33's red was re-measured after the house merge by one named plant, beside the disclosure above,
which stays as it is. At 65b5b0c, in `crates/ingest/tests/support/recording.rs`'s `local_change`,
`carries.then(|| format!("{}: {}", request.method, body))` became
`carries.then(|| format!("{}: {}", request.method, body)).filter(|_| false)`. A33's fenced command
then failed, rc 101, by its assertion "the classifier marks an upload"
(`crates/ingest/tests/recorder_control.rs:66`). The file was restored byte-equal from a copy before
any commit or proof (its sha256 `325e3d6d80d2bd311eaecebcbb3574dca5f33807f0e9ad24df3a64b4377fe5d4`
before and after), and no commit holds the plant.

The criteria A10 to A18, A45 and A46 were written at 8dc8649 against stubs that compile: a tariff
with an empty ladder, a price of 0 and nothing paid; a preview of defaults and settlements that
answer nothing; an empty skip set; and four of the skip constants set one off (the default search
stayed as it is, since `crates/ingest/tests/settings.rs` pins its literal). 16712c8 re-asserted
A15 and A18 where the fold dates a spent freeze, on the day the walk meets the gap, so that each
fails for its own criterion. Each was then run at 16712c8 with its fenced command, selecting one
test, and failed by its own assertion; each passed at 2669bc8.

2669bc8, the green commit, also edits four test files, none of them in an assertion a criterion
here judges: A18's range `D0 - 3..=D0 - 1` is written `D0 - 3..D0`, the same days, for clippy;
`crates/coordination/tests/lapse.rs`'s two calls pass `open_lapse` its new skip set, empty;
`crates/economy/tests/wallet_census.rs` admits `crates/economy/src/tariff.rs`, which reads what a
skip paid, among the files that name the coin ledger; and `crates/coordination/tests/relight_order.rs`
counts 18 statics, the tariff's ladder among them.

```red-first
A10: red at 16712c8: skip_flow.rs:210, the preview's day spec for a golden case; left: "", right: "1-3"
A10: green at 2669bc8
A11: red at 16712c8: skip_flow.rs:277, the settlement for a golden case; left: None, right: Some(Settlement { price: 0, paid: 0, unfunded: false })
A11: green at 2669bc8
A12: red at 16712c8: skip_flow.rs:327, the skip applies and records its shortfall; left: (Pending, false), right: (Applied, true)
A12: green at 2669bc8
A13: red at 16712c8: skip_flow.rs:376, the undo refunds what the skip paid, for a golden case; left: 0, right: 50
A13: green at 2669bc8
A14: red at 16712c8: skip_tariff.rs:27, the ladder is economy.json's; left: [], right: [0, 50, 100]
A14: green at 2669bc8
A15: red at 16712c8: skip_effects.rs:283, the skip day spends no freeze; left: [(19999, -1, "consumed")], right: []
A15: green at 2669bc8
A16: red at 16712c8: skip_effects.rs:332, a missed day tiers the run down; a skip day leaves it as it was; left: [1, 1], right: [1, 2]
A16: green at 2669bc8
A17: red at 16712c8: skip_effects.rs:411, a skip inside three silent days leaves two, and no lapse; left: [Some(19997), Some(19997)], right: [Some(19997), None]
A17: green at 2669bc8
A18: red at 16712c8: skip_effects.rs:442, while the skip stands its day is bridged; left: [(19999, -1, "consumed")], right: []
A18: green at 2669bc8
A45: red at 16712c8: skip_record.rs:297, constants.SKIP_SPREAD_MIN_DAYS equals the predecessor's; left: Number(2), right: Number(1)
A45: green at 2669bc8
A46: red at 16712c8: skip_flow.rs:430, the tariff is charged once, on the skip's day; left: [], right: [(20105, -100)]
A46: green at 2669bc8
```

## D13 is pinned by a test of a later skip (round 1 of the review)

A52's red was measured by one named plant, not by a committed stub. In
`crates/coordination/src/skip/mod.rs`'s `earlier_in_month`, `.filter(|record| record.day < day &&
month.is_some())` became `.filter(|record| record.day != day && month.is_some())`, D13's own rejected
alternative. A52's fenced command then failed, rc 101, by its assertion "a retry is priced as its
first attempt" (a free skip's retry was charged 50 after a later skip applied), where A46 and the
whole `skip_flow` target had passed. The file was restored byte-equal (its sha256
`cab3b04df4e339dc25962c2c2de27f1793b3131e4ed22463a4e977c73a3bcb70` before and after), and no commit
holds the plant.

```red-first
A52: not red: measured red by the plant above, at 0d4236da, with the earlier-day filter planted to day != day: a retry is priced as its first attempt; committed green only
```


## The take's write, its backup, counts and stop (E4b, #108)

The order of work: the zone pin and A55's census (d4b60742, A55 red; 3a7e628d, green); the
take-side collections (82e9207a); the engine write census alone (10a72541, A24 red); A9 against the
write port's stubbed reschedule (c46626ab); the write port (091bb53b, A9 and A24 green); the class's
stop and the preview's list (002d901f red, daee15fc green); E4a's final head absorbed by merge
(d3c09f6a); the take's criteria against stubs (0fa34058); the take (5677cf5a); A5's case of a card
that joined after the preview (7f07d484); A40's later daylight rule given a window that exists
(448cde99); `read_counts` answering `Counts`, behaviour-neutral (e90c6530); the rows (60c4fc4b);
the covers stamped (1c3cb009); the changelog fragment (5ca21228); three cure tests beside their plants (e0574a8d), the plants removed
(d9da8860); the zone pin's redundant shape check removed (3a0e1a76); a quoted zone name holding a sign pinned beside
its plant (00bcf056), the plant removed (d1a26f22).

Each criterion below was run at its red commit with its fenced command, or the take arm's own line
of section 3c, selecting one test, and failed by assertion for its own criterion. At 0fa34058 the
stubs compiled: a take that answered `Failed(engine_failed)` after its stop and lock, a preview
that checked neither the zone nor the engine's day, a restore check that passed nothing, counts
that never moved and an erase that removed nothing.

A28, A40 and A44 hold two arms each, and each criterion carries one red line. A28's preview arm
went red at 002d901f and green at daee15fc, and that pair is its line. Its take arm went red at
0fa34058, where `confirm None: the take did not answer preview_changed` failed at line 369. A40's
and A44's take arms refuse before any engine call, so their lines quote the preview arms that
0fa34058 left red.

5677cf5a, the green commit, also edits the test side. None of these edits weakens an assertion a
criterion judges, and each was measured by the first full run on that commit:
- `support/synthetic.rs`: `change_cards` also moves the collection's modified stamp, since a sync
  whose stamps are equal on both sides exchanges nothing; `as_served` marks a served collection's
  unsynced rows synced, since a server never holds one; `build_skip` sets the last unbury day to the
  build's day.
- `support/mod.rs`: `review_card_on_another_client`, which answers Good on another client and syncs.
- `skip_write.rs`: the ledger is read at the fixture's own path; `Scene::build` serves the collection
  through `as_served`; A34's other client reviews a card the take moves; A40's other client syncs
  under the collection's own zone before the process zone returns; A25 also asserts that no
  snapshot row and no backup file exist in both arms.
- `skip_zero_upload.rs`: the served collection goes through `as_served`; the wait runs inside the
  runtime; the owner's sync clock moves past the debounce between syncs.

After the green, A5's test gained one more case at 7f07d484: after the preview, another client
makes a card due today that the preview did not list, and the take must leave it alone while its
push carries exactly the previewed cards. It pins the selection that row S08310 mutates.

A40 and A6 are green at 5ca21228, the first commit a full run of the five targets measured after
5677cf5a (`skip_write` 20 passed, `skip_zero_upload` 1 passed). At 5677cf5a A40 failed only at its
`later` case: the rule `AAA-1BBB,J365/22,J365/23` has no daylight window under chrono's rule math,
which reads a rule's start in standard time and its end in daylight time, so both fall at 21:00
UTC. 448cde99 corrected that fixture, never an assertion: `git diff 7f07d484 448cde99` adds 4 and
removes 3 lines of `skip_write.rs`, which move the rule's text to `J365/20`, add a comment that
says why, and move one doc line from `rezone` to `examined`; none of them is an `assert` line. A6
never ran at 5677cf5a, since cargo stops at a failed binary, so its first green on this branch is
at 5ca21228.

```red-first
A5: red at 0fa34058: skip_write.rs:740, Failed(engine_failed) != Accepted
A5: green at 5677cf5a
A6: red at 0fa34058: skip_zero_upload.rs:193, a refused take (preview_changed) answered Failed(engine_failed)
A6: green at 5ca21228
A9: red at c46626ab: skip_write.rs:49, examined 0 review-log row(s) the reschedule wrote
A9: green at 091bb53b
A24: red at 10a72541: examined 0 engine card write call(s) inside impl CollectionWrite for RslibEngine
A24: green at 091bb53b
A25: red at 0fa34058: skip_write.rs:687, the take answers full_sync_required: Failed{EngineFailed}
A25: green at 5677cf5a
A26: red at 0fa34058: skip_write.rs:844, download true: only the other client's upload; left: 0, right: 1
A26: green at 5677cf5a
A28: red at 002d901f: the preview arm, examined 0 previewed card(s)
A28: green at daee15fc
A29: red at 0fa34058: skip_write.rs:881, the planted stop ended the take
A29: green at 5677cf5a
A34: red at 0fa34058: skip_write.rs:924, AfterSnapshot: the other client answered one card; left: 0, right: 1
A34: green at 5677cf5a
A36: red at 0fa34058: skip_write.rs:687, too_many_cards answered as Failed{EngineFailed}
A36: green at 5677cf5a
A38: red at 0fa34058: skip_write.rs:997, Failed{EngineFailed}
A38: green at 5677cf5a
A39: red at 0fa34058: skip_write.rs:1028, Failed != Accepted
A39: green at 5677cf5a
A40: red at 0fa34058: skip_write.rs:1194, the engine's rollover hour differs: the preview refuses; left: Listed, right: Refused(EngineDayDiffers)
A40: green at 5ca21228
A44: red at 0fa34058: skip_write.rs:1194, unset: the preview refuses; left: Listed, right: Refused(ZoneNotPinned)
A44: green at 5677cf5a
A47: red at 0fa34058: skip_write.rs:687, backup_check_failed answered as Failed{EngineFailed}
A47: green at 5677cf5a
A48: red at 0fa34058: skip_write.rs:1083, Failed{EngineFailed}
A48: green at 5677cf5a
A49: red at 0fa34058: skip_write.rs:687, writes_stopped answered as Failed{EngineFailed}
A49: green at 5677cf5a
A53: red at 0fa34058: skip_write.rs:1143, Failed{EngineFailed}
A53: green at 5677cf5a
A54: red at 002d901f: left: Listed { cards: [], digest: "" }, right: Stopped(Stopped { set_by: Counts, reason: "review_log_rows", .. })
A54: green at daee15fc
A55: red at d4b60742: Lists differ: [] != ['planted.yml:6: replicates /var/lib/deck-streak/skip-backup-1.anki2', ... 8 elements]
A55: green at 3a7e628d
A56: red at 0fa34058: skip_write.rs:1179, it answers what it removed; left: 0, right: 3
A56: green at 5677cf5a
```

## Mutation cures (ruling 76)

Three survivors' classes had no test that observed them, so three tests join the criteria they
belong to. They are replays of A36, A44 and A48, not new criteria, so their lines are quoted here
and not in the fence above. Each was committed at e0574a8d beside a planted copy of weaker code,
run there with `--exact`, one test each, and seen to fail by assertion; d9da8860 removed the plants
(the module's bytes equal 5ca21228's) and all three passed.

```text
A44 the_pin_refuses_a_rule_outside_the_posix_grammar: red at e0574a8d (plant: in_range's upper bound widened to 99): skip_write.rs:1751, UTC25: the pin refuses it; left: Listed {..}, right: Refused(ZoneNotPinned)
A44 the_pin_refuses_a_rule_outside_the_posix_grammar: green at d9da8860
A36 the_card_guard_admits_a_set_of_exactly_its_bound: red at e0574a8d (plant: the guard reads >=): skip_write.rs:725, the take answers backup_check_failed: Failed { reason: TooManyCards, preview: None }
A36 the_card_guard_admits_a_set_of_exactly_its_bound: green at d9da8860
A48 the_counts_name_the_first_count_a_reschedule_does_not_explain: red at e0574a8d (plant: the review-log sum multiplies): skip_write.rs:1374, left: Some("review_log_rows"), right: None
A48 the_counts_name_the_first_count_a_reschedule_does_not_explain: green at d9da8860
A44 the_pin_refuses_a_rule_outside_the_posix_grammar: red at 00bcf056 (plant: zone_name's quoted arm refuses both signs): skip_write.rs:1763, <+00>0 is pinned
A44 the_pin_refuses_a_rule_outside_the_posix_grammar: green at d1a26f22
```

3a0e1a76 removes `zone_pin`'s shape check, whose four `&&` mutants were equivalent: the grammar
refuses every value the shape refused (an empty value or one led by `:` or `/` has no zone name,
`localtime` has no offset after its name, and a `/` sits only before a transition's time, which
must be digits). A44's inputs that only the shape refused before (`""`, `localtime`,
`:/etc/localtime`, `/etc/localtime`, `Etc/../UTC`) are each still refused at 3a0e1a76.
