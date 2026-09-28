# The parity oracle

DeckStreak's game math is proved against its predecessor's own functions, never re-derived
(CHARTER 8, ADR-012). The generator calls those functions over seeded synthetic inputs and writes
one golden per registration. The Rust tests read the goldens through one reader, and
`test_goldens.py` keeps every committed golden current in public CI (ADR-029, SPEC-029).

| file | what it is | where it runs |
|---|---|---|
| `generate.py` | the generator: loads the registry, calls the predecessor, writes the goldens | the owner's machine, beside a private checkout of the predecessor |
| `registry/spec_NNN.py` | one SPEC's registrations | loaded by the generator |
| `goldens/<name>.json` | the goldens, in the schema `phx.parity-golden.v1` | committed, and read by the tests |
| `golden.rs` | the one Rust reader, included by path | every proving crate's integration tests |
| `test_generate.py` | the generator's tests, over a synthetic stand-in | the gate's python stage |
| `test_goldens.py` | every committed golden is current, synthetic and free of date strings; no registry module reads a file; the day token's example below round-trips | the gate's python stage |

## Register a golden

A SPEC that ports a rule adds its own module, `registry/spec_NNN.py` under its own number, and
never edits another SPEC's module. The module exports `FUNCTIONS`: a golden's name mapped to one
registration, written to `goldens/<name>.json`. A name registered by two modules is refused,
naming both. Every path names an object of the predecessor relative to its package, such as
`analytics.study_day`, so no committed file names the package.

A case builder, `cases(rng)`, receives only a `random.Random` seeded with `SEED`, and returns
`(class or None, input)` pairs. It covers the classes a port gets wrong: a `tie` that Python
rounds to even, a `negative` operand for floor division and modulo, and the study day's
`rollover`. The generator draws the cases twice and refuses a builder whose second draw differs,
because it drew from something other than `rng`.

### A function

When every argument is JSON, the generator calls the function with each case's input as keyword
arguments. For example:

```python
def cases(rng):
    return [("tie", {"ease": 0.5, "maturity": 1.3})] + [
        (None, {"ease": rng.choice([0.5, 1.0, 1.2]), "maturity": 2.0}) for _ in range(8)
    ]


FUNCTIONS = {"review_xp": {"kind": "function", "function": "xp.review_xp", "cases": cases}}
```

### An adapter

When an argument is not JSON (an object, a store, a sleep to patch), glue in the registry module
builds it and calls the function. The glue receives the function already resolved, then
`predecessor`, which resolves any other object of the predecessor, then the case's input:

```python
def with_collection_config(study_day, predecessor, *, instant_ms, rollover_hour, utc_offset_minutes):
    config = predecessor("types.CollectionConfig")(
        rollover_hour=rollover_hour, tz_offset_minutes=utc_offset_minutes
    )
    return study_day(instant_ms, config)


FUNCTIONS = {
    "study_day": {
        "kind": "adapter",
        "function": "analytics.study_day",
        "adapter": with_collection_config,
        "note": "Builds the predecessor's CollectionConfig from rollover_hour and ...",
        "cases": cases,
    },
}
```

An adapter CALLS the predecessor and never computes the rule: its `note` says what it builds or
patches, and review holds it to that (ADR-029). It may write synthetic data built from the case's
own input into a temporary directory, but a registry module never reads a file, a database or the
network: `test_goldens.py` refuses one that calls `open`, reads a `Path`, or imports `sqlite3`,
`socket`, `urllib` or `http`.

### Constants

A constant the port must use verbatim is read from the predecessor, never typed:

```python
FUNCTIONS = {
    "kernel.constants": {
        "kind": "constants",
        "names": ["constants.DEFAULT_ROLLOVER_HOUR", "constants.DEFAULT_DIGEST_HOUR"],
    },
}
```

Each case is `{"input": {"name": <path>}, "output": <the value>}`. A constants golden drives no
one function, so its `function` is its own name.

## What a golden holds

- `schema`, `kind` and `function`, and for an adapter its `adapter` and `note`. The note also
  records the interpreter that drew the cases, because a seeded sequence may change between
  Python versions.
- `source_commit`: the predecessor's commit.
- `generator` with `generator_sha256`, and `registry` with `registry_sha256`. A golden is stale
  when either digest differs from the committed file: editing one registry module stales only its
  own goldens, and editing `generate.py` stales them all.
- `inputs: synthetic` and `seed`.
- `cases`: each an `input`, an `output`, and an optional `class`.

No golden holds a calendar date or a time string. A returned `date` is written as its epoch day
number (whole days since the Unix epoch), an aware `datetime` as epoch milliseconds, and a
duration by an adapter as a number whose key names its unit.

### A day inside a text

A day the predecessor reads or writes inside a text, such as a note's frontmatter, cannot become a
number the way a returned `date` does, because the text itself is the case's input or output. Such
a golden writes each day in its text as the token `{day:N}`, where N is the day's epoch day number:
whole days since 1970-01-01, and negative before it. `registry/spec_042.py` does this for the
reading note's roll (SPEC-042).

The token is lossless. Each day has exactly one token and each token exactly one day, so nothing
is rounded or dropped in either direction. The adapter holds both directions: it refuses a case
whose text holds a raw date, so every day in a golden is a token, and it refuses a date the
predecessor returns that does not read back as itself, so a token always names exactly the date
the predecessor wrote.

A registry module emits the token through its adapter, which calls the predecessor and never
computes the rule (ADR-029):

1. `expand` writes every token of the case's text as the ISO date of its day, and the adapter
   refuses the case when writing those dates back does not give the case's text;
2. the adapter calls the predecessor's function on the expanded text, passing any other day the
   case names as a `date`;
3. `contract` writes every ISO date of the returned text back as its token.

`with_days_as_numbers`, with its `expand` and `contract`, in `registry/spec_042.py` is the pattern.
A module that needs the token writes its own in the same shape, because a SPEC never edits another
SPEC's module. For example, a synthetic note's text as its golden holds it:

```text
date: {day:11016}
last_rolled: {day:-1}
rolls: 0
```

and the same text as the predecessor reads it:

```text
date: 2000-02-29
last_rolled: 1969-12-31
rolls: 0
```

`test_goldens.py` round-trips this example through `registry/spec_042.py`'s own `expand` and
`contract`, and holds the golden's form free of dates while the golden check refuses the other.

## Regenerate

On the owner's machine, beside a clean private checkout of the predecessor:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/parity-oracle/generate.py \
  --source-checkout PATH/TO/PREDECESSOR --package PACKAGE --out tools/parity-oracle/goldens
python3 -m unittest discover -s tools/parity-oracle -p 'test_*.py'
```

The generator refuses a malformed registry with exit 2 before it calls anything, writes no golden
unless every golden could be written, and writes nothing into the checkout, not even bytecode.
Commit `generate.py`, the registry and the goldens together, and say in the pull request which
predecessor commit produced them. Public CI never regenerates a golden: the predecessor is
private (ADR-012).

## Read a golden in Rust

A proving crate includes the reader in an integration test, and adds `serde.workspace = true`
and `serde_json.workspace = true` to its `[dev-dependencies]`, and nothing else (ADR-029):

```rust
#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

#[test]
fn the_study_day_equals_the_predecessors() {
    golden::each_case("study_day", |case| {
        // Read the input, run the port, and compare with case.output, naming case.input.
    });
}
```

`each_case` runs the check over every case, prints `examined N case(s) of <function>`, and
panics on a refused golden (another schema, no case, a missing field) or on zero cases.
