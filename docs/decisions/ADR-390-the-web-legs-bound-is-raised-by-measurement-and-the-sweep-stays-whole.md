---
status: "accepted"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-390: the web legs' bound is raised by measurement to hold the whole sweep, and the sweep stays whole

Decides SPEC-379.

## Context and Problem Statement

Two jobs run StrykerJS over the Mini App and the verdict reads its one report: the weekly
battery's `web` job sweeps every production file, and `ci.yml`'s `mutation-web` job mutates every
web production file a pull request changes, whole (ADR-057 D3), which on a release is most of the
app. Both declare `timeout-minutes: 60`. The weekly sweep of run 37924809374 was cut at that bound
(job 113826350088): 6425 of 6446 mutants tested, no report written, and the `survivors` job
113850137962 read `table: verdict: VOID`. A larger sweep of 6886 mutants completed in run
37924800926 (job 113801075582). A job cut at its bound judges no mutant, so a release whose web
leg is cut cannot be judged at all, and a battery whose web leg is cut files no survivor.

## Decision Drivers

- No fewer mutants and no verdict moved in the permissive direction (ADR-199's first driver,
  `docs/decisions/ADR-199-a-memory-scope-stops-a-runaway-mutant-and-the-timeouts-stay.md:21`).
- The verdict's reading of StrykerJS's report, through `judge`, `battery` and `table` in
  `scripts/mutation-verdict.py`, is not this delivery's to change.
- The bound is derived from measured runs by a stated rule, and a test holds it, as the engine,
  harness and release jobs' bounds are held.
- A hung test still ends in bounded time.

## Considered Options (the alternatives each was chosen against)

D1, what makes the sweep fit:

- Chosen: both legs' `timeout-minutes` is raised to the bound the rule gives, because it keeps
  every mutant, every StrykerJS setting and the verdict's one-report reading as they are.
- (B) Shard the web sweep, as #693 asks: rejected because it moves the verdict's reading from one
  StrykerJS report to several, through the `survivors` job and `judge`, `battery` and `table`,
  none of which this delivery edits; sharding stays #693's, open.
- (C) Raise StrykerJS's `concurrency` from 2: rejected because more test-runner processes contend
  for the same hosted runner, so more mutants reach their per-test timeout, and StrykerJS scores a
  timed-out mutant as detected, so a verdict can move in the permissive direction.
- (D) `ignoreStatic`, or a narrower `mutate` set: rejected because each examines fewer mutants,
  which is a weakening by ADR-199's first driver.
- Raise the weekly leg alone: rejected because the pull request's leg sweeps most of the app on a
  release (5542 mutants of 73 files in job 112096749167), and the app grows toward the 6886 the
  weekly leg measured; one rule for both legs keeps a release judgeable.
- Re-run a cut leg: rejected because a mutant cost 0.5464 s in the cut sweep (job 113826350088)
  and 0.4271 s in the completed one (job 113801075582), so a re-run draws on the runner's speed
  and every later run inherits the draw.

D2, the bound and the band:

- Chosen: 100 minutes, held inside 100 to 120 by a band test, because it is the rule's product and
  the ceiling is tied to the measured sweep.
- A bound of 95: rejected because the rule's product before rounding is 95.87 minutes, past it.
- A band whose floor is the product rounded up to the minute (96): rejected because the rule
  rounds up to the five, so a bound under 100 is under the rule's own result.
- A ceiling at the hosted job's own limit: rejected because a bound far above the need only lets a
  hung test hold a runner longer, and hides the hang.
- A ceiling of 130 or more: rejected because twice the projected whole sweep is 129.03 minutes,
  and rounding down to the ten keeps a hung test's wait within two hours.

D3, the reading of ADR-199's first driver:

- Chosen: "a raised or lowered timeout" there is the per-mutant test timeout, because that bound
  decides a mutant's verdict: StrykerJS's `timeoutMS` and `timeoutFactor`, and cargo-mutants'
  `--timeout`, each decide whether a mutant reads killed, timed out or survived. A job's
  `timeout-minutes` decides none: a job cut at its bound writes no report, and the leg reads VOID
  by name, never a pass. The repository's own records draw the same line: the `harness` job's
  bound rose from 90 to 150 minutes by measurement (SPEC-361 R15, ADR-372 D10), and the `release`
  job's from 60 to 100 (SPEC-367 R1, ADR-378, `RELEASE_TIMEOUT_MINUTES`), each with no ruling;
  every per-mutant budget raise was signed by the owner, the latest
  `docs/rulings/OWNER-RULING-2026-10-06-mutation-timeout-2300.md`.
- Reading the job's bound as a timeout under that driver: rejected because no verdict depends on
  it: a raise lets no mutant pass that a cut leg failed, since a cut leg judged no mutant at all.

D4, the row whose anchor the raise duplicates:

- Chosen: row `S36701`'s anchor and replacement name the `release` job, its mutant and killer
  unchanged, because the raise makes `timeout-minutes: 100` occur twice in `ci.yml` and the rows
  census refuses an anchor that does not occur exactly once.
- Setting the pull request's leg to 105 to keep the old anchor unique: rejected because the bound
  would then follow a row's text, not the rule, and the two legs would differ for no measurement.
- Leaving the anchor: rejected because the rows census would refuse `S36701` by its stem, so the
  release job's bound would lose its only row.

## Decision Outcome

Both web legs' `timeout-minutes` is 100, by this rule: the worst measured cost a mutant times the
larger measured count, one and a half times, plus the longest measured set-up and tail, rounded
up to the five minutes. The arithmetic, each figure with its job:

- worst cost a mutant: 0.5464 s, 3510.33 s over 6425 mutants (job 113826350088);
- larger count: 6886 mutants (job 113801075582);
- 0.5464 x 6886 = 3762.2 s; x 1.5 = 5643.3 s = 94.05 minutes (SPEC-327 R2's margin);
- longest set-up, job start to the initial test run's end: 1.63 minutes (job 113826350088);
- longest tail, last progress line to job end: 0.18 minutes (job 112096749167);
- 94.05 + 1.63 + 0.18 = 95.87 minutes, rounded up to the five: 100.

`WEB_MUTATION_TIMEOUT_MINUTES` in `scripts/tests/test_ci_workflows.py` holds both legs inside 100
to 120: 100 is the rule's bound, and 120 is twice the projected whole sweep (3762.2 s plus 1.63
and 0.18 minutes, 64.52 minutes, from jobs 113826350088, 113801075582 and 112096749167), 129.03,
rounded down to the ten. Six rows set each leg back to 60, one past the ceiling, and without a
bound; the band test kills each.

### Consequences

- Good: the cut sweep would have finished: at its own cost its 6446 mutants need 58.70 minutes
  of mutation testing plus 1.63 of set-up (job 113826350088), inside 100 with room to spare.
- Good: no StrykerJS setting, no verdict code and no other job changes, so every verdict is read
  exactly as before.
- Bad: a hung test now holds a runner for up to 100 minutes before the job is cancelled, not 60;
  StrykerJS's own per-test timeout still ends a mutant's hung test well before that.
- Neutral: a leg's typical duration is unchanged; only its limit moves.

### Confirmation

SPEC-379's A1 reads both legs' `timeout-minutes` as digit strings inside 100 to 120, red at the
base where both are 60; A2 plants each defect. The settings that decide a verdict are unchanged,
read at the fix commit `35a058db` against the base `e7ecf10d`. The first command prints nothing:

```
$ git diff --stat e7ecf10d..35a058db -- \
    web/app/stryker.config.json scripts/mutation-verdict.py
$
```

So StrykerJS's `concurrency`, `timeoutMS`, `mutate` and `thresholds`, its absent `timeoutFactor`
and `ignoreStatic`, and the verdict's script are byte-identical. The second prints one one-line
hunk in each workflow, the `mutation-web` job's line 704 and the `web` job's line 190, so no line
of the `survivors` job, or of any other job, moved:

```
$ git diff -U0 e7ecf10d..35a058db -- \
    .github/workflows/mutation-weekly.yml .github/workflows/ci.yml
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index a7a6633f..5280e53d 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -704 +704 @@ jobs:
-    timeout-minutes: 60
+    timeout-minutes: 100
diff --git a/.github/workflows/mutation-weekly.yml b/.github/workflows/mutation-weekly.yml
index 8fd478a4..bb848def 100644
--- a/.github/workflows/mutation-weekly.yml
+++ b/.github/workflows/mutation-weekly.yml
@@ -190 +190 @@ jobs:
-    timeout-minutes: 60
+    timeout-minutes: 100
```

A1's red and green readings are recorded in `docs/red-first/SPEC-379.md`.

## What would make this wrong

- A completed web leg past two thirds of the bound (66:40): re-measure and re-derive by the rule.
- A leg cut at 100: a measurement this rule did not hold; it is taken to the owner's architect as
  evidence for #693's sharding, not answered by a second raise.
- A StrykerJS change that makes a job's bound decide a mutant's verdict: then D3's reading no
  longer holds and the bound is a timeout under ADR-199.

## More Information

- SPEC-039 R3 and R18 and its section 8, and SPEC-362's section 5, each amended by a section naming
  SPEC-379; ADR-057 D3 (whole files on a pull request), unchanged; ADR-373 D1's amendment, whose
  Python legs are sized by shards the verdict already reads one by one, unlike the web report.
- SPEC-327 R2 (the 1.5 margin); SPEC-361 R15 with ADR-372 D10, and SPEC-367 with ADR-378 (job
  bounds raised by measurement); ADR-199 (the first driver).
