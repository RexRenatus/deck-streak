# Regression records

A record is a JSON file in this folder that a pull request carries when it keeps a change that
makes a bench slower. CodSpeed's check on the pull request shows which bench is slower and by how
much; the record says why the change is kept and what follows it (SPEC-618, ADR-640).

## When a pull request carries one

A pull request carries a record when its CodSpeed check reports a bench slower and the change is
kept rather than fixed. A pull request that makes no bench slower carries none. One record per pull
request, named after its branch, for example `perf/regressions.d/feat-faster-queue.json`.

## What a record holds

The schema `deckstreak.perf-regression.v1` is closed: a key it does not name is refused.

```json
{
  "schema": "deckstreak.perf-regression.v1",
  "branch": "<the pull request's head branch>",
  "benchmarks": [
    {
      "name": "<a name from perf/benchmarks.txt>",
      "change_pct": 4.2,
      "base": "<the base commit, 7 to 40 lowercase hex>",
      "head": "<the head commit, 7 to 40 lowercase hex>",
      "instrument": "simulation"
    }
  ],
  "cause": "<what in the change makes the bench slower>",
  "why": "<why the change is kept>",
  "spec": "<the SPEC that decides the change>",
  "alternatives": ["<each faster form considered, and why it lost>"],
  "remediation": "requested"
}
```

`remediation` is `requested` or `not-needed: <reason>`. The record carries no verdict.

## How it is checked

On each pull request the Rust bench build job copies the records that pull request adds or changes
in this folder and runs:

```
scripts/perf-record.py check --dir <the copied records> --branch <the head branch> --benchmarks perf/benchmarks.txt
```

It refuses a record with an unknown or a missing key, a blank `cause`, `why` or `spec`, a
`change_pct` that is no number above 0, a benchmark name `perf/benchmarks.txt` does not hold, a
`branch` other than the pull request's own, or any other value outside the schema. A refusal fails
`ci`. Only `*.json` files are records, so this guide is never checked as one. The merge step reads
the CodSpeed check and the record together.
