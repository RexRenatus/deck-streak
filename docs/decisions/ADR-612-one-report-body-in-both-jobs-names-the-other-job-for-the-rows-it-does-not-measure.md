---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-612: one report body in both jobs of the split, which names the other job for each row it does not measure

Decides SPEC-591 (R6 and R9).

## Context and Problem Statement

After the split (ADR-610), `harness` and `release-and-app` each hold some of the result bundles and each must write
a report. At the base the report reads every row's source in one job, and a source it cannot read reads "not
measured". The census reads the report's text inside the workflow. Should the two jobs share a checked-in report
script, or carry one inline body, and what does a row of the other job read?

## Decision Drivers

- The census reads the report where it reads it today, at equal strength.
- Two copies must never drift apart silently.
- A mutation row's find must occur once in its target.
- A full run's report holds no "not measured" row, so a row of the other job must say where it is measured.

## Considered Options (the alternatives it was chosen against)

- **Chosen: the same inline body in both jobs, held equal by the census; each source is owned by one job, a row of
  the other job reads "in `<that job>`", and the body names its job from the runner's job id, failing on any id
  that is not a job of the split.**
- One checked-in report script that both jobs run: rejected because it moves the census's reading of the report
  out of the workflow and re-points every report assertion; no mutation row anchors inside the report body at the
  base (the band files were read for it), so the copy costs no row.
- The base body unchanged in both jobs: rejected because each report would then read "not measured" for every row
  of the other job, which a full run's reading refuses.
- Two different bodies, each covering its own job: rejected because the census would need two sets of report
  assertions, and drift between them would go unseen.
- An environment entry naming the job: rejected because the two environments would then differ, and the census
  holds them equal (SPEC-591 R3).

## Decision Outcome

Chosen: one inline body, because every report assertion keeps reading the workflow text where it reads it today,
drift becomes a census failure, and each report says where every row it does not hold is measured:

- the owners are `harness` for the Debug rows and `release-and-app` for the Release, app, review and bury and flag
  rows; a row of its own job reads its measurement or "not measured", so a missing measurement still shows;
- each job uploads under its own name, `harness` and `release-and-app`, since two uploads of one name in one run
  conflict;
- a change to the body is made in both copies in one commit.

### Consequences

- Good: the report's census and its rows keep their meaning, and a full run's two reports hold no "not
  measured" row.
- Bad: the report body is written twice, and the file grows by its length.

### Confirmation

SPEC-591 A7 and A9, red at the base, with rows S59107 and S59110.

## More Information

- SPEC-339 R14 and R16 (the report's rows), SPEC-591 R6 and R9, ADR-610.
- What would make this wrong: a mutation row that must anchor inside the report body, which would make the
  checked-in script the better form; or a runner whose job id differs from the job's id in the file, which the
  report then fails on.
