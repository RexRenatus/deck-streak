# Red-first record: SPEC-045

The order of work was: the SPEC promoted, ADR-045 accepted and the resolution drawn (b0d8ecc); A12's
census committed alone (a41b7f1); the goldens and the example taxonomy (5e3da60); the acceptance
tests beside stubs (277391c); the implementation (2c414de); the hand-proved rows (fc0a5eb); tests
for the diff's mutants (1d6a90e); and `dev` absorbed by a merge commit that resolved conflicts only
(a0e64ee).

A12's census was committed alone, before any code. It read the tree with the public scrub and
found no example taxonomy: its positive artifact, `deploy/config/readings-taxonomy.example.json`,
did not exist yet. The goldens came next: `tools/parity-oracle/registry/spec_045.py` registered
`resolve_day_sets`, `law_subject` and `digest_for_card_ids`, and the generator wrote them from the
predecessor's own functions at `27ee2bc`, the example taxonomy's synthetic names in place of every
private deck constant the functions read. The registry module's sha256 read the same before and
after the run, the predecessor's checkout was left as it was with no bytecode written into it, and
no other golden changed. The example taxonomy was committed with the goldens, which turned A12
green.

The other tests were committed (277391c) beside a readings crate whose public API was in place and
whose behaviour was stubbed: the taxonomy parsed to no root, no language and no writing root; no
deck had a law subject or a topic; the digest was empty; no answer was saturated; the resolver and
the universe were empty; the resolution resolved nothing and applied no budget; the last sync always
succeeded, the owner always studied and the pause window was today twice; every reason's class was
`rail_broken`, and every state counted as a failure and a refusal; the store wrote and read nothing;
and the data-rights port declared no table. The migration, coordination's use case and registry,
the symmetry probe's seeds, the register of DeckStreak's own tables, `privacy.json` and `PRIVACY.md`
were committed with the tests. Each criterion was run there with the SPEC's own fenced command,
selecting one test, and failed by assertion for its own criterion, not by a compile error, a missing
fixture or an empty selection.

The implementation followed (2c414de). Between the red commit and the green one no test changed
what it asserts: four test files gained the workspace's lint allowance for a test helper's
`expect`, one binding was renamed, a closure became a function path, and `RESOLVE_BUDGET - 1 ms`
became `RESOLVE_BUDGET.saturating_sub(1 ms)`, the same instant. After green, the twelve hand-proved
rows (fc0a5eb) were proved on the committed tree: every row KILLED, each target restored byte for
byte. Then every mutant cargo-mutants lists over the diff was read against the tests, and the ones
no test could tell apart were given one (1d6a90e): the taxonomy's refusals, accessors and debug
form, its path setting, the review read's floor, and the queue's and the read's failures. Three
impls no caller used were removed rather than tested.

```red-first
A1: red at 277391c: assertion `left == right` failed; left: [], right: ["language/qaa", "law/evidence", "law/torts"]
A1: green at 2c414de
A2: red at 277391c: assertion `left == right` failed: the resolution of the every-track case; left: no active topic and no unmapped deck, right: ten topics and four unmapped decks
A2: green at 2c414de
A3: red at 277391c: assertion `left == right` failed: the subject of "Casebook"; left: Null, right: String("Casebook")
A3: green at 2c414de
A4: red at 277391c: assertion `left == right` failed: the digest of []; left: String(""), right: String("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
A4: green at 2c414de
A5: red at 277391c: assertion `left == right` failed; left: {}, right: {11, 12}
A5: green at 2c414de
A6: red at 277391c: Some(Error)
A6: green at 2c414de
A7: red at 277391c: assertion `left == right` failed; left: [20000, 20000], right: [19999, 19998]
A7: green at 2c414de
A8: red at 277391c: assertion `left == right` failed: every state reads back as itself; left: [], right: the 28 topic days written
A8: green at 2c414de
A9: red at 277391c: assertion failed: saturated(1000, None)
A9: green at 2c414de
A10: red at 277391c: assertion `left == right` failed: the budget is waited out, no longer; left: 0ns, right: 30s
A10: green at 2c414de
A11: red at 277391c: assertion `left == right` failed; left: {}, right: {"language/qaa", "law/evidence", "law/torts"}
A11: green at 2c414de
A12: red at a41b7f1: AssertionError: False is not true : deploy/config/readings-taxonomy.example.json is missing: R1's synthetic taxonomy shows the file's shape
A12: green at 5e3da60
A13: red at 277391c: assertion `left == right` failed: the readings' record is the owner's data: exported and erased (CHARTER 13); left: [], right: [("reading_topic_days", ExportAndErase), ("reading_runs", ExportAndErase)]
A13: green at 2c414de
A14: red at 277391c: an absent route is a setting, not a failure
A14: green at 2c414de
```

In A1 and A5 the stub resolved no topic of the engine's queue; in A2 and A11 no deck mapped to a
topic; in A3 no deck had a law subject; in A4 the digest was empty; in A6 a failed last sync read as
succeeded; in A7 the pause window was today twice; in A8 the store read back nothing; in A9 a root of
1000 cards with no count read as complete; in A10 the resolution returned at once, with no budget;
in A12 the example taxonomy was missing; in A13 the port declared no table; and in A14 an absent AI
route counted as a failure.

## Fix round

The first review found that a budget passing during the throwaway copy left the copy in the scratch
directory and released the lock before the copy ended (R7). The order of work was: `dev` absorbed
by a merge commit that resolved conflicts only (b1b1f95); the new test committed alone (929593b);
the fix (ee5e423); its hand-proved row, S04513 (46b4d0b); the test's scratch listing passed through
the examined contract (e9affde); and this record with the documents.

`a_budget_passed_during_the_copy_leaves_no_copy_behind` (`--test day_set`) holds the copy part-way:
a named pipe stands at the throwaway's path, so the copy blocks once the pipe is full, and the test
drains it only after a 10 ms budget has passed and dropped the port's future, as `resolve` drops
it. It is not one of §3's criteria, so its record stands outside the `red-first` fence:

```text
R7 copy test: red at 929593b: a copy outlived its budget: ["readings-day-set-0.anki2"]
R7 copy test: green at ee5e423
```

At 929593b the split lifecycle's copy ran on, detached, to its full size, and nothing removed it.
With the test's last two assertions swapped, the same code also failed `the shared lock was held
through the copy, past the budget`, because the lock was released as the future dropped. Run 50
times at 929593b the test failed 50 times with that line; run 50 times at ee5e423 it passed 50
times. Row S04513 deletes the removal the copy's guard makes, and the test kills it.

The fence above now quotes A6's and A14's panic lines as the tests print them, measured again at
277391c: an `assert!` with a message prints only its message, so A6's line is the run it formats
(`{last:?}`) and A14's is its message.

DISCLOSURE, A1 (`the_day_set_is_the_schedulers_queue_per_root_attributed_by_original_deck`): its
body changed at 1d6a90e, after its green commit. `assert_eq!(day_set.note_ids, day_set.card_ids,
"{}", resolved.topic);` became `assert_eq!(day_set.note_ids, day_set.card_ids, "{:?}",
resolved.topic);`, because 1d6a90e removed `TopicKey`'s `Display`, which no caller used, and
derived its `Debug`. Only the assertion's message changed; what it compares did not.

DISCLOSURE, A7 (`two_days_without_study_pause_every_topic`): its body changed at 2c414de, its green
commit. `pause_window(support::today()).map(|day| day.epoch_day())` became
`pause_window(support::today()).map(StudyDay::epoch_day)`, because clippy's pedantic
`redundant_closure_for_method_calls`, which the gate denies, refuses the closure. The same values
are compared.

DISCLOSURE, A8 (`every_state_and_class_is_stored_and_read_back_distinctly`): its body changed at
1d6a90e, after its green commit. `assert_eq!(reason.class(), expected, "{reason}");` became
`assert_eq!(reason.class(), expected, "{reason:?}");`, because 1d6a90e removed `CouldNotTell`'s
`Display`, which no caller used. Only the assertion's message changed.

DISCLOSURE, A10 (`a_resolution_past_its_budget_is_rail_broken`): its body changed at 2c414de, its
green commit. `RESOLVE_BUDGET - Duration::from_millis(1)`, the early queue's delay, became
`RESOLVE_BUDGET.saturating_sub(Duration::from_millis(1))`, because clippy's pedantic
`unchecked_time_subtraction` refuses a `Duration` subtraction that could panic. Thirty seconds less
a millisecond is the same instant either way.

DISCLOSURE, A11 (`the_topics_come_only_from_the_configured_taxonomy`): its body changed at 2c414de,
its green commit. `let named: BTreeSet<String> = resolution` and `assert_eq!(named, expected);`
became `let resolved_topics: BTreeSet<String> = resolution` and
`assert_eq!(resolved_topics, expected);`, because clippy's pedantic `similar_names` refuses `named`
beside the test's `names`. The assertion is unchanged.

DISCLOSURE, the R7 copy test (not a criterion): its body changed at e9affde, after its green commit.
`let left: Vec<_> = fs::read_dir(&scratch)` and its chain became `let left =
examined_may_be_empty("entries left in the scratch directory", fs::read_dir(&scratch)` with the
same chain, because the tdd pack's examined contract asks a test file that walks a directory to
report how many entries it examined, and the contract's form that may be empty fits a directory
expected to hold nothing. The assertion is unchanged; the new body, run against 929593b's split
lifecycle, failed with the same line.

No other criterion test's body changed after its red commit: A2, A3, A4, A5, A6, A9, A12, A13 and
A14 read the same at their red commits and at this round's head, and the fix round changed none.
