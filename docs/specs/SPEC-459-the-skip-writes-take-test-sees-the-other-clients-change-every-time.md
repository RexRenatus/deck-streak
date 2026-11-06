# SPEC-459: The skip write's take test sees the other client's change every time

Status: accepted, one delivery, decided by ADR-476. Refs #918; the delivery's pull request closes
it.

File: `docs/specs/SPEC-459-the-skip-writes-take-test-sees-the-other-clients-change-every-time.md`.
Base: every `path:line` below was read at dev `83846ea3929ab3805c1e95db7291097fde559cd2`.

## 1. The problem, measured

- In one pull-request run's `mutation-rust` legs, leg 138's baseline (job 116852801329, 37.885 s)
  failed `deck-streak-ingest::skip_write a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows`
  at `crates/ingest/tests/skip_write.rs:972:9`, "fsrs false: the card the preview did
  not list is left alone". The answer was `Accepted { moved: [1700100000001, 1700100000002,
  1700100000003], left_alone: [], read_back: [] }` where `left_alone: [1700100000012]` was expected.
  The `mutants` profile then terminated three sibling tests (57.127 s, 4.264 s and 3.719 s, same job).
- The card `SKIP_FLOOR + 12` is made due today by another client between the preview and the take
  (`skip_write.rs:958-968`): `synthetic::change_cards` (`crates/ingest/tests/support/synthetic.rs:1147-1172`)
  then `support::sync_on_another_client` (`crates/ingest/tests/support/mod.rs:780-785`).
- `change_cards` stamps the collection's modified time as the clock's whole second times 1000
  (`synthetic.rs:1149-1155`, `synthetic.rs:1169`) and never reads the stamp it replaces. The copy it
  changes is a byte copy of the private copy (`skip_write.rs:687-691`), whose stamp is the server's
  after the owner's full download (`crates/ingest/src/sync.rs:332-347`; `synthetic.rs:1188-1191`).
- A sync with equal stamps on both sides exchanges nothing (`synthetic.rs:1141-1142`;
  `synthetic.rs:354`; production reads a completed exchange from the stamp moving,
  `crates/ingest/src/engine.rs:330-342`), and `sync_on_another_client` checks only that the sync
  returned (`mod.rs:756-771`), where `review_on_another_client` asserts the stamp moved
  (`mod.rs:414-430`).
- So the change can stay on the other client, the take's converge (`crates/ingest/src/skip_write.rs:304`)
  brings nothing, and the selection (`skip_write.rs:458-487`) lists only the previewed cards: the
  answer leg 138 printed. ADR-476 D1 gives the steps and the routes the reads exclude; the two
  stamps in leg 138 were not printed, so the coincidence there is inferred, not measured.
- The census (section 7): 1 failure; 10 passes read by their own lines; 137 passes by their job's
  verdict; 98 baselines of the failing run whose result for this test was not read.

## 2. Requirements

R1. `synthetic::change_cards` sets the changed collection's modified stamp to exactly one past the
stamp the collection held when it was called, whatever the clock reads; each changed card keeps its
new due, a modification time in whole seconds and the unsynced mark, as now.

R2. `support::sync_on_another_client` reads the collection's modified stamp before and after the
other client's sync and refuses, with a panic whose message contains "the other client's sync
carried no change to the server", a sync that left the stamp where it was.

R3. `a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows` and
`a_card_that_changed_between_the_preview_and_the_converge_is_left_alone` keep every assertion and
every call byte for byte; only the helpers under them change.

R4. No test is skipped, ignored, filtered, retried or weakened; no timeout, profile or bound moves;
no file under `crates/ingest/src/` changes.

## 3. Acceptance criteria of SPEC-459

| id | criterion | decided by |
|---|---|---|
| A1 | `change_cards` leaves the collection's stamp exactly one past the stamp it found, for a stamp a year ahead of the clock and for the copy's own built stamp, printing `examined 2 stamp(s)` (R1) | `skip_write::a_change_on_another_client_stamps_its_copy_one_past_the_stamp_it_found` |
| A2 | an other client's sync whose stamp was set to the server's after its change is refused by name, and the recording layer saw that sync's one `meta` request and no `finish` (R2) | `skip_write::an_other_clients_sync_that_carries_no_change_is_refused_by_name` |
| A3 | the two take tests pass with their assertions unchanged (R3) | `skip_write::a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows` and `skip_write::a_card_that_changed_between_the_preview_and_the_converge_is_left_alone` |

```acceptance
A1: cargo test -p deck-streak-ingest --test skip_write -- --exact a_change_on_another_client_stamps_its_copy_one_past_the_stamp_it_found
A2: cargo test -p deck-streak-ingest --test skip_write -- --exact an_other_clients_sync_that_carries_no_change_is_refused_by_name
A3: cargo test -p deck-streak-ingest --test skip_write -- --exact a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows
A3: cargo test -p deck-streak-ingest --test skip_write -- --exact a_card_that_changed_between_the_preview_and_the_converge_is_left_alone
```

A1 is red first on every run: at the base the stamp is the clock's whole second, below a stamp a
year ahead, so its exact-value assertion fails. A2 is red first on every run: at the base the
helper returns, so `catch_unwind` answers `Ok` and the refusal assertion fails. A3 is not red: it
passes at the base and after, and guards that the subject itself was not edited. A2 is a plain
test over `std::panic::catch_unwind`, never `#[should_panic]`.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/ingest/tests/support/synthetic.rs` | 1 | `change_cards` reads the stamp it found and writes one past it (R1); its doc names SPEC-459 R1 |
| `crates/ingest/tests/support/mod.rs` | 1 | `sync_on_another_client` reads the stamp before and after the sync and refuses an unmoved one by name (R2) |
| `crates/ingest/tests/skip_write.rs` | 1 | the two new tests A1 and A2, and the module doc naming SPEC-459; the two take tests unchanged (R3) |
| `scripts/mutation-rows.d/S45900-S45999.json` | 1 | new band, rows S45900 to S45902 (section 8) |
| `docs/specs/SPEC-459-the-skip-writes-take-test-sees-the-other-clients-change-every-time.md` | 1 | new: this SPEC |
| `docs/decisions/ADR-476-the-other-clients-change-stamps-one-past-the-stamp-it-found-and-its-sync-is-checked.md` | 1 | new: the decision |
| `docs/schematics/the-skip-write-tests-other-client-change-always-reaches-the-server.md` | 1 | new: the take's path from the preview to the read, the dependence and the cure |
| `docs/red-first/SPEC-459.md` | 1 | new: the red-first record |
| `changelog.d/skip-write-preview-459.md` | 1 | new: the changelog fragment |

No census reads the changed files: the skip census walks only `crates/*/src`
(`crates/ingest/tests/skip_census.rs:150-175`), and no mutation row and no formal cover names any
of the three test files this delivery edits, by full or crate-relative path, at the base.

## 5. What this does NOT cover

- The progression censuses that terminated at the leg's timeout in the same run's red baselines,
  tracked by #917.
- The progression census test's speed, tracked by #692.
- The Python runner test helpers' temp files, tracked by #833.
- The simulator harness, tracked by #839.
- Any other suite, and any other test's order or timing, which #918 does not name; a recurrence
  seen only by a later CI run is read against #918 when it is seen.

## 6. Risks

- The private copy's stamp may differ from the server's after the owner's sync. Then A2's forced
  equality does not hold, the sync exchanges, and A2 stays red after R2: the builder stops and
  reports, and the dependence is elsewhere (ADR-476, What would make this wrong).
- The engine may exchange a change despite equal stamps. Then A2 also stays red after R2, which
  refutes ADR-476 D1.3 by measurement.
- R2 makes a sync of an unchanged copy fail by name. The two callers always change first
  (`skip_write.rs:966-967`, `skip_write.rs:1608-1609`); a new caller that does not is refused on
  purpose.
- An open pull request changes `crates/ingest/tests/support/synthetic.rs` too (#769): a text
  conflict there is resolved at its land, keeping both changes.

## 7. The census

The test's result per execution, each figure beside its job or artifact on the same line.

| where | result | figure |
|---|---|---|
| leg 138 baseline, job 116852801329 | FAIL | 37.885 s, first loop (fsrs false) |
| leg 4 baseline, artifact 12034287584 | PASS | 98.243 s; that baseline red on the progression census, terminated at 1371.187 s |
| leg 39 baseline, artifact 12036596584 | PASS | 98.000 s; red on the progression census at 1380.297 s |
| leg 89 baseline, artifact 12040467680 | PASS | 96.073 s; red on the progression census at 1398.446 s |
| leg 137 baseline, artifact 12049712679 | PASS | 97.445 s; red on the progression census at 1366.862 s |
| leg 139 baseline, artifact 12049119648 | PASS | 95.583 s; red on the progression census at 1376.013 s |
| leg 190 baseline, artifact 12048525712 | PASS | 95.697 s; red on the progression census at 1366.459 s |
| leg 0 baseline, artifact 12035213593 | PASS | 70.475 s; baseline green, 2058.961 s |
| leg 2 baseline, artifact 12037005357 | PASS | 75.074 s; baseline green, 1930.017 s |
| `rust` test stage, artifact 12057161155 (job 117031548517, 2059 s stage) | PASS | 78.437 s |
| `rust` test stage, artifact 12057403758 (job 117043344169, 2406 s stage) | PASS | 95.348 s |

Counted by verdict, not by the test's own line: 85 more baselines of the failing run that its
mutation verdict did not void (each such baseline ran every test green); 52 more `rust` jobs over
the last 60 completed CI runs whose test stage was green (46 by the job's conclusion, 6 by their
`ok test` stage line with the job red on its dependency audit). Not counted: 98 baselines of the
failing run that the verdict voided, not read (a read bound); 5 cancelled `rust` jobs; one run with
no jobs. No other run in the 60 held a `mutation-rust` leg that runs this test. Every census read
took each run's latest attempt, and every run read was a first attempt.

## 8. Mutation rows (band `scripts/mutation-rows.d/S45900-S45999.json`)

Each row's find is copied from the delivery's code, occurs exactly once at its target, and its
replacement keeps every item used. The crate is `deck-streak-ingest`.

| row | target | mutant | killer |
|---|---|---|---|
| S45900 | `tests/support/synthetic.rs` | the stamp written is the stamp found, not one past it | A1 |
| S45901 | `tests/support/synthetic.rs` | the stamp written is the clock's whole second times 1000, as at the base | A1 |
| S45902 | `tests/support/mod.rs` | the refusal never fires (its condition always true, both stamps still read) | A2 |

`cargo mutants --list --in-diff` over a diff of test files lists no mutant, so these hand rows are
the delivery's only mutation evidence.

## 9. Formal

- TLA+: not applicable. The steps are strictly sequential: the test thread drives each step to
  completion before the next (the other client's change, its sync, the take), and the server child
  and the recording relay answer the requests that thread sends one after another. The dependence is
  a value coincidence between two stamps, not an order of actions. The three `@phx covers` lines
  under `formal/` name `crates/ingest/src/skip_write.rs` (`take`, `restore_check`, `moved_counts`),
  which this delivery does not edit; none names a file it edits.
- Lean: not applicable. The rule is a test fixture's single step, deciding no gate verdict, ledger,
  seal, ratchet, money or release. It is closed by A1's two stamps, which the test builds, and by
  A2's oracle, the engine's own sync answer read through the recording layer.

## 10. Red-first

The record is `docs/red-first/SPEC-459.md`: `A1: red at <sha>: <failure>`, `A2: red at <sha>:
<failure>`, `A3: not red: <why>`, then one `green at <sha>` line each for A1 and A2.
