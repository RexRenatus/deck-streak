# ADR-476: The other client's change stamps one past the stamp it found, and its sync is checked

Status: accepted, for SPEC-459 (the skip write's take test sees the other client's change every
time). Decides SPEC-459. Refs #918.

Base: every `path:line` below was read at dev `83846ea3929ab3805c1e95db7291097fde559cd2`.

## Context

The test `skip_write::a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows`
(`crates/ingest/tests/skip_write.rs:936-1037`) failed once in the `mutation-rust` baselines of one
pull-request run: leg 138's baseline (job 116852801329) failed it after 37.885 s at
`crates/ingest/tests/skip_write.rs:972:9`, and the fail-fast profile then stopped three sibling
tests (SIGTERM after 57.127 s, 4.264 s and 3.719 s in the same job). The panic printed the whole
answer:

    left: Accepted { moved: [1700100000001, 1700100000002, 1700100000003], left_alone: [], read_back: [] }
    right: Accepted { moved: [1700100000001, 1700100000002, 1700100000003], left_alone: [1700100000012], read_back: [] }

So the take moved the right cards and read nothing back; only the card the preview did not list,
`SKIP_FLOOR + 12`, was missing from the converged copy's selection.

The test plays another client between the preview and the take
(`crates/ingest/tests/skip_write.rs:958-968`): it copies the private copy (`Scene::other`,
`skip_write.rs:687-691`), makes `SKIP_FLOOR + 12` due today with `synthetic::change_cards`
(`crates/ingest/tests/support/synthetic.rs:1147-1172`) and sends the change with
`support::sync_on_another_client` (`crates/ingest/tests/support/mod.rs:780-785`). The take then
converges a working copy with the server (`crates/ingest/src/skip_write.rs:304`) and selects on it
(`skip_write.rs:312`, `select` at `skip_write.rs:458-487`): `left_alone` is every selected card the
preview did not list (`skip_write.rs:478-485`).

The sibling test `a_card_that_changed_between_the_preview_and_the_converge_is_left_alone`
(`crates/ingest/tests/skip_write.rs:1591-1627`) plays the same other client with the same two
helpers (`skip_write.rs:1608-1609`). These four lines are the only callers of the two helpers in
the tree (a fenced `git grep` read 6 hits: the four calls and the two definitions).

## Decision

### D1. The dependence

The other client's change can leave that client's collection stamped exactly as the server's,
and then its sync exchanges nothing and nothing says so.

1. `change_cards` writes the collection's modified stamp as the clock's whole second times 1000
   (`synthetic.rs:1149-1155` reads whole seconds, `synthetic.rs:1169` writes `now * 1000`), without
   reading the stamp the copy holds.
2. The copy it changes is a byte copy of the private copy (`skip_write.rs:687-691`), and the
   private copy came from the owner's sync, a full download of the server's collection
   (`crates/ingest/src/sync.rs:332-347`; `synthetic.rs:1188-1191`: "A full download then carries no
   pending change into the private copy"). Its stamp before the change is the server's.
3. A sync whose stamps are equal on both sides exchanges nothing: the fixture states it
   (`synthetic.rs:1141-1142`), `synthetic::modified` states it ("which a sync that changed nothing
   leaves alone", `synthetic.rs:354`), and production reads a completed exchange from the stamp
   moving (`crates/ingest/src/engine.rs:330-342`).
4. `sync_on_another_client` checks only that the sync returned (`mod.rs:756-771`, `780-785`), where
   `review_on_another_client` asserts that the stamp moved (`mod.rs:414-430`).
5. With nothing on the server, the converge brings nothing and the selection is the preview's
   three cards: `left_alone` is empty, which is leg 138's answer (job 116852801329, 37.885 s).

So when the server's stamp is a whole second and the change falls in that second, the change
never reaches the server. The take is not at fault: it leaves alone what its converge saw.

Not measured: the two stamps in leg 138 (no line of the job prints them); why the server's stamp
was a whole second there (its source is the engine's, which this decision did not read); the rate
of the coincidence. The census below reads one failure.

What differs in a mutation baseline from the `rust` job, as read:

- the profile: the baseline runs nextest's `mutants` profile, `fail-fast = { max-fail = 1,
  terminate = "immediate" }` (`.config/nextest.toml:8-9`), so one red stops the run and terminates
  the tests still running; the `rust` job keeps the default profile (`.config/nextest.toml:1-2`)
  and runs past a red (14 failures in one run's test stage, job 117043344169, 2406 s);
- the tree: the leg runs in place (`--in-place`, `.github/workflows/ci.yml:533`), so no copied tree;
- the wrapper and the build: the leg runs under a wrapper script with
  `CARGO_INCREMENTAL: "0"` (`ci.yml:470`, `ci.yml:533`); the `rust` job runs the test stage of its
  check script (`ci.yml:67-70`);
- test threads: neither sets them (no `test-threads` in `.config/nextest.toml`, none on either
  command), so both run nextest's default; both run each test in its own process, so the target's
  zone lock and `TZ` are one process's (the `--exact` command line in job 116852801329, 37.885 s);
- load: the test's own durations are alike: 70.475 s to 98.243 s in eight sampled baselines of the
  same run (each figure beside its leg's artifact in SPEC-459's census table), and 78.437 s and
  95.348 s in two `rust` jobs (job 117031548517, a 2059 s test stage, and job 117043344169, a
  2406 s test stage);
- the count: one run held 192 baselines, each running the test once, where a run holds one `rust`
  job. A rare coincidence surfaces where the test runs most.

Routes the reads exclude: an unordered read (the engine's card read orders by id,
`engine.rs:442-443`, and `select` sorts `left_alone`, `skip_write.rs:484-485`); a day rollover
(`moved` equals the preview's list, and a changed list answers `PreviewChanged` at
`skip_write.rs:398-425`); the zone (set once before the first step and waited for,
`skip_write.rs:53-68`, in a process of its own; a zone fault answers a day-and-zone failure, never
`Accepted`); a concurrent writer (each step blocks to completion, and the take holds the exclusive
collection lock, `skip_write.rs:234-240`); the server's card merge (the change is pending with a
modification time of now, over a server card inserted with modification time 0,
`synthetic.rs:546-548`, newer under any rule).

Chosen against: a day or zone race, an unordered read and a concurrent writer, each excluded by
the reads above; an engine fault, which no read shows (see What it was chosen against).

### D2. The cure

Two fixture changes, together:

- R1: `change_cards` sets the collection's modified stamp to one past the stamp it found in that
  collection. The copy's stamp is the server's from its last sync (D1.2), so the change's stamp can
  never equal it, whatever the clock reads.
- R2: `sync_on_another_client` reads the collection's stamp before and after the sync and refuses,
  by name, a sync that left it where it was: the engine's answer for an exchange that carried
  nothing. A precondition the take tests assumed is now checked where it is made.

No assertion of either take test changes, no test is skipped, filtered or retried, and no bound
moves.

Chosen against: a retry of the other client's sync, a wait for the next whole second, either
change alone, a check through the recording layer in the two tests, and a production change (see
What it was chosen against).

### D3. Production or test

Test code only: `crates/ingest/tests/support/synthetic.rs`, `crates/ingest/tests/support/mod.rs`
and two new tests in `crates/ingest/tests/skip_write.rs`. No file under `crates/ingest/src/`
changes, so no production behaviour is pinned anew; the take's selection rule
(`skip_write.rs:455-487`) and its covered spans stay as they are.

Chosen against: a production change, because no production nondeterminism was read: a converge
that pulls nothing is a legitimate answer the take must keep giving.

### D4. Shape and split

One delivery: SPEC-459, this ADR, rows band `S45900-S45999`, one new schematic, one red-first
record and one changelog fragment, sized for one builder. No SPEC or ADR on dev is amended: SPEC-083
A39's fixture sentence ("the collection's modified stamp moves, so that client's next sync sends
it", `synthetic.rs:1140-1141`) stays true and becomes enforced.

Chosen against: two deliveries (R1, then R2), because either alone leaves the test random (R2 alone)
or silent (R1 alone) between them; an insert-only amendment of SPEC-083, because the rule is a test
fixture's, which SPEC-459 owns whole; an amendment of an existing schematic, because the listing of
the schematics directory held fenced names, so none was opened, and a new schematic owns this flow.

### D5. Tests and rows

- A1, red first and every time: `change_cards` on a copy whose stamp is a year ahead of the clock,
  and on the copy's own built stamp, leaves the stamp exactly one past each. At the base the stamp
  is the clock's whole second, below the stamp a year ahead, so the exact-value assertion fails on
  every run.
- A2, red first and every time: the seam the flake needed, forced. A served scene's other client
  changes `SKIP_FLOOR + 12`, its stamp is then set to the private copy's (the server's), and its
  sync runs under `std::panic::catch_unwind`. The test asserts the refusal names "the other
  client's sync carried no change to the server", then that the recording layer saw that sync's one
  `meta` request and no `finish`. At the base the helper returns, so the first assertion fails on
  every run; after R2 the refusal fires and the protocol counts show the engine exchanged nothing.
  A2 green is the measurement of D1.3 that no log printed.
- A3, not red: the two take tests keep every assertion byte for byte and pass at the base and
  after; they are the subject made deterministic, not new evidence.
- Rows S45900 to S45902 (SPEC-459 section 8). No wall-clock assertion and no loop that hopes to
  meet the race: each red is decided by values the test builds.
- FORMAL: not applicable for TLA+ and for Lean (SPEC-459 section 9).
- One push: both reds are read locally at the red commit, so no red is owed to CI.

Chosen against: a red that loops the take test until it fails, and a red that asserts elapsed time,
because each fails at random or measures the clock; a `#[should_panic]` test for A2, because its
run line is not a plain test result and a red-first reader counts no new test from it.

### D6. Not in this delivery

The progression censuses' red baselines (#917) and their speed (#692), the Python runner test
helpers' temp files (#833), the simulator harness (#839), and any other suite. SPEC-459's "What
this does NOT cover" lists each with its issue.

Chosen against: folding #917's census timeouts in, because they are another test's budget: each of
the six red baselines sampled in the same run was red on the progression census, terminated at the
leg's timeout, while this test passed in it.

## What it was chosen against

- D1, a day or zone race: the take answered `Accepted` with the preview's three cards, which a day
  change or a zone fault cannot produce (`skip_write.rs:398-425`, `skip_write.rs:53-68`).
- D1, an unordered read or a map's order: every read on the path orders by id or sorts
  (`engine.rs:442-443`, `skip_write.rs:484-485`).
- D1, a concurrent writer: every step blocks to completion and the take holds the exclusive lock
  (`skip_write.rs:234-240`).
- D1, an engine fault in the server's merge: no read shows one, and the change is newer than the
  server's card under any merge rule (`synthetic.rs:546-548`).
- D2, retrying the other client's sync until the server holds the change: it hides the no-exchange
  answer instead of removing its cause, and a retry is a gate narrowing.
- D2, waiting for the next whole second before the change: it is a wall-clock wait that moves the
  coincidence and still stamps whole seconds.
- D2, R2 alone: the test would still fail at random, only with a clearer message.
- D2, R1 alone: any other cause of an exchange that carries nothing would stay silent.
- D2, asserting through the recording layer in the two take tests: it duplicates R2 in two places,
  where the helper serves both callers once.
- D3, a production change that refuses a converge which pulled nothing: it would refuse a
  legitimate converge, and no production nondeterminism was read.
- D4, two deliveries: either half alone leaves the test random or silent until the other lands.
- D4, an insert-only amendment of SPEC-083: SPEC-083 A39's sentence already states the intent; the
  rule that enforces it is SPEC-459's.
- D4, amending an existing schematic: none was opened (the directory listing held fenced names),
  and a new schematic owns this flow alone.
- D5, a looped or timed red: it fails at random or measures the clock, not the behaviour.
- D5, `#[should_panic]` for A2: its run line is not a plain test result, so a red-first reader
  counts no new test from it.
- D6, curing the progression censuses here: another test's budget, tracked by #917 and #692.

## Consequences

- The take tests no longer depend on two stamps differing by chance: the other client's change is
  stamped past the server's by construction, and a sync that carries nothing fails by name in the
  helper rather than as a misleading `left_alone` in the take.
- The two helpers keep their signatures; their four call sites are unchanged.
- The rows S45900 to S45902 hold R1 and R2 against reversal.
- A future caller of `sync_on_another_client` that syncs an unchanged copy now fails by name. That
  is intended: the helper's contract is "sends what it changed".

## What would make this wrong

- A2 stays red after R2: then an exchange with equal stamps does carry the change, D1.3 is wrong,
  and the dependence is elsewhere.
- The private copy's stamp differs from the server's after the owner's sync: A2's forced equality
  then does not hold, the sync exchanges, and A2 stays red, which says so.
- The take test fails again after this lands with `left_alone` empty: the other client's sync then
  passed R2's check, so the change reached the server, and the loss is in the converge.
