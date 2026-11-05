---
status: "accepted"
---

# ADR-632: a report helper takes a temporary directory per run, removed with its test

Decides SPEC-610.

## Context and Problem Statement

Two test helpers run the Python mutation runner and read the report it writes: `Tree.report` in
`scripts/tests/test_mutation_python.py` and `TheRunPrintsAndWritesItsReport.report_of` in
`scripts/tests/test_mutation_python_cli_kills.py`. Each wrote the report into the parent of the
scenario tree, which is the shared system temporary directory, and removed only the report once it
had read it. The runner opens a `costs.jsonl` beside every report and appends to it (SPEC-418 R6),
so every run of either helper appended to one shared file that outlived the run, and runs at the
same time appended to the same file. Issue #833 asks that each helper give each run a directory of
its own, removed with the test.

## Decision Drivers

- Nothing a test writes outlives the test, and no two runs share a file.
- The runner's contract stays as SPEC-418 R6 states it: the costs record goes beside `--report`.
- Each helper returns what it returned before, in the same order, so no caller changes.
- The scenario tree holds exactly the fixture's files, since tests compare `Tree.files` whole.

## Considered Options (the alternatives each was chosen against)

- Chosen: a `tempfile.TemporaryDirectory()` per helper call, registered with the test's
  `addCleanup`, with `Tree` keeping the test that made it, because each run then gets a report
  and a costs record of its own that go when the test ends, and neither the runner nor any caller
  changes.
- One directory per `Tree`, made when the tree is built: rejected because a tree's runs would
  append to one `costs.jsonl`, the same mixing in a smaller place.
- The report inside the scenario tree: rejected because it would show in the fixture's git status
  and in `Tree.files`, which tests compare whole.
- Removing `costs.jsonl` after each run: rejected because the shared directory stays where runs at
  the same time write, so two runs still race on one file.
- A runner switch that skips `costs.jsonl` under tests: rejected because it changes SPEC-418 R6 to
  fix a test.

## Decision Outcome

Chosen option: a temporary directory per helper call. `Tree.__init__` keeps the test that made the
tree as `self.test`. `Tree.report` makes a `tempfile.TemporaryDirectory()`, registers its cleanup
with `self.test.addCleanup` and writes the report there as `report.json`. `report_of` makes its own
directory the same way, registered with its own test's `addCleanup`, and writes `cli.json` there.
Each helper still removes its report once it has read it; the directory, and the `costs.jsonl` the
runner put beside the report, go at the test's cleanup.

### Consequences

- Good: no run of either helper leaves a file in the shared system temporary directory, and no two
  runs append to one costs record.
- Good: the runner, every caller and every existing assertion stay as they are.
- Neutral: each helper call makes one more temporary directory, removed at its test's cleanup.
- Neutral: a `Tree` now holds the test that made it, which its constructor already required.

### Confirmation

SPEC-610 A1 and A2 fail against the old helpers by their assertion on the report's directory and
pass after the change, with the tests unchanged. Rows S61000 and S61001 each put one helper's
report back into the parent of the scenario tree; A1 and A2 kill them in the `mutation-rows` job.

## What would make this wrong

A runner that writes a record anywhere but beside `--report`. Such a record would not go with the
helper's directory, and these tests would not see it.

## More Information

SPEC-610 states the requirements and the criteria. SPEC-418 R6 is the costs record beside the
report. Issue #833 reports the shared file.
