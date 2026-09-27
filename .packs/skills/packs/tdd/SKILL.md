---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: cac3ede067874a89b831ed4e544829291364ddd9
body_status: seeded
---

# packs/tdd

Test-driven development held by evidence rather than by intention: a test is seen red for the
reason its criterion states before the code exists, its assertions observe something present, a
test that enumerates says how much it examined, and a mutant proves the test can fail. Its probe
judges any repository root (SPEC-V2-2186, ADR-V2-2186). Which seats consume this pack is its
catalog row's `consumes` (ADR-V2-1990), so this body names none.

```
phxd pack probe --pack tdd --root PATH --format json
python3 scripts/tdd-probe.py --root PATH check all
```

`pack probe` keeps only each row's exit; run the probe itself to read the findings.

## Red first

1. Write the test the acceptance criterion names, under the name the SPEC's fence gives it.
2. Run it at the base, before the implementation, and read WHY it fails. The failure must be the
   one the criterion states: an assertion on the missing behaviour. A test that fails because it
   does not compile against the base API, because a fixture is missing, or because the runner
   selected nothing is not red for the criterion's reason.
3. Implement until it passes. Run the whole suite, not only the new test.
4. Record both facts, `docs/red-first/<SPEC id>.md` (ADR-V2-2186 D4):

       ```red-first
       A1: red at 3f2a91c: assertion `left == right` failed: left: [2, 1], right: [1, 2]
       A1: green at 8d07e4b
       A2: not red: pins the ordering the base already had; it guards the refactor, it proves no new behaviour
       ```

   One line per fact. `red at <sha>: <the failure observed>` names the commit the test ran
   against and quotes the failure. `green at <sha>` is a later commit. `not red: <why>` discloses a
   criterion that could not be red first, with the reason. Red and green at one commit are
   refused: the point of the record is that the test preceded the code.

A test green at the base is not red-first evidence. A test written after the code passes for
reasons nobody checked.

## Assertions that observe something

- **Assert a positive artifact.** `assert_eq!(deck.due(), vec![a, b])`, not
  `assert!(deck.due().is_empty())` on a fixture that should have produced cards. A mutant that
  deletes the behaviour leaves an absence true, so an absence-only test cannot kill it.
- **Pair every absence census with a positive control.** A guard asserting "no violations" over a
  population is absence-shaped by design; beside it, a planted fixture the guard must refuse by
  name, so a guard that went blind fails.
- **Assert a value, never a shape.** A 64-character hex digest is a shape; the digest of the known
  input is a value.
- **Never derive the expected value from the code under test.** Who built each side of the
  comparison? If the test built both, it compares the code with itself.

The probe flags a test whose every assertion is an absence: in Rust `assert!(!x)`,
`x.is_none()`, `x.is_empty()`, `assert_ne!`, `assert_eq!` against `None`, `false`, `0`, `""`,
`vec![]`; in Python `assertNotIn`, `assertIsNone`, `assertFalse`, `assertEqual(x, [])`, `assert
not x`, `assert x is None`; in TypeScript `.not.`, `.toBeUndefined()`, `.toBeNull()`,
`.toBeFalsy()`, `.toHaveLength(0)`. It reads shapes, not behaviour, and it cannot see a positive
control in another test, so it is a lint: a repository can declare it advisory.

## The examined count, refused at zero

A test that derives its subjects by enumeration (a glob, a directory walk, a `git ls-files`) can
derive none of them: a directory is renamed, a glob stops matching, and every assertion then
passes over the empty set. So every enumerating test reports how many it examined, and refuses
zero, through one helper:

    // Rust
    fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
        println!("examined {} {what}", items.len());
        assert!(!items.is_empty(), "examined 0 {what}: the population is empty, so nothing was judged");
        items
    }

    # Python
    def examined(what, items):
        items = list(items)
        print(f"examined {len(items)} {what}")
        if not items:
            raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
        return items

A population that may legitimately be empty takes a second door, `examined_may_be_empty`, that
prints the count and does not refuse, and the call site says why. Every probe in these packs
holds itself to the rule: each verdict carries `examined N`, and a class that examined nothing
exits VOID, never OK.

## Mutation testing

A test proves something only when it fails on a program that is wrong. Mutation testing makes the
program wrong on purpose:

- **One row per behaviour**: a file, an exact `find` that occurs once, its `replace`, and the one
  test that must fail. The row names the behaviour its mutant removes.
- **The killer must die on its mutant, and only there.** Run it on the unmutated tree: green, with
  exactly one test selected. Install the mutant: red, for the reason the row names. Restore the
  file byte for byte. A killer that selects zero tests passes every mutant.
- **Anchor on code that will stay.** A `find` that a reformat re-indents stops matching; add new
  behaviour between statements rather than wrapping them.

phoenix-v2's mutation-rows pack holds the rest, measured on 7,000 rows.

## Names and time

- **A test is named for the behaviour it pins**, as a sentence: `due_cards_come_oldest_first`,
  `a_red_with_no_green_is_refused`. The name is what a failing run prints, so it must say what
  broke without opening the file.
- **No test depends on time passing.** A `sleep`, a wall-clock deadline or a race against another
  thread flakes on a loaded machine and trains everyone to re-run reds. Inject the clock, wait on
  the event, or poll with a bound the test states. A flaky test is a red nobody believes.

## Check table

| id | scope | green when | on phoenix-v2 |
|---|---|---|---|
| `acceptance-has-a-test` | tree | every fenced acceptance command names a test the tree holds | advisory: `scripts/spec_contracts.py` proves selection with `phxd idea selects` |
| `red-first-recorded` | tree | every judged SPEC with fenced commands has a record: red then green per criterion, or disclosed | advisory: `scripts/red-first.py`, the gate's red-first stage, re-runs new tests at the merge-base |
| `absence-only-assertions` | tree | no test asserts only absences | advisory: the mutation stage kills what an absence cannot |
| `examined-counts` | tree | every enumerating test calls the examined contract | advisory: `scripts/tests/test_guards_report_examined.py` holds the same rule |

`acceptance-has-a-test` resolves, without running anything: `cargo test` (`-p`, `--test`,
`--lib`, filters and `--exact`, every filter selecting at least one `#[test]`-attributed
function), `cargo nextest run` (the same, and `-E 'test(name)'`), `python3 -m unittest`
(`discover -s -p -k`, or dotted names), `pytest` (paths, `::` node ids, `-k` expressions),
`vitest` and `jest` (path filters and `-t`, through `npx`, `pnpm` or `npm test --`) and
`node --test`. A command in any other shape is refused by name: a criterion must be decided by a
command someone can show selects a test.

## Configuration

| key (`tdd`) | default |
|---|---|
| `red_first` | `docs/red-first` |
| `test_globs` | Python `**/test_*.py`, `**/*_test.py`; TypeScript and JavaScript `**/*.test.ts` and its kin; Rust `**/*.rs`, of which only files holding a `#[test]` function are tests |
| `examined_calls` | `examined`, `examined_may_be_empty`, `examinedMayBeEmpty` |
| `advisory` | `{}` |

The SPEC population, and the `adopted_from` below which a SPEC owes no record, are the `sdd`
section's.

## Adopt in another repository

1. Copy `scripts/methodology_probe.py`, `scripts/sdd-probe.py`, `scripts/ddd-probe.py` and
   `scripts/tdd-probe.py` from a phoenix-v2 checkout into your `scripts/`, side by side.
2. In `methodology.json`, set `vendored_from`. A Rust workspace needs nothing else: the default
   globs find `#[test]` functions in `crates/*/src` and `crates/*/tests`, and SPEC fences name
   `cargo test -p <crate> --test <target> -- --exact <name>` first. A TypeScript front end's
   `*.test.ts` files are found by the same defaults.
3. Add `python3 scripts/tdd-probe.py --root . check all` to CI, and write the first SPEC's red-first
   record in the same change as its first test.

## The reference implementation

phoenix-v2 runs these rules through stronger machinery: its red-first stage re-runs a delivery's
new tests at the merge-base and classifies the delivery (the red-first pack), its mutation stage
proves every row's killer (the mutation-rows pack), and the gate-traps pack records every way a
gate reported green having examined nothing. Its `methodology.json` declares all four classes
advisory and names those gates, so the probe runs here and gates nothing twice.

## What this pack does not do

- It runs no test. It resolves commands to tests statically; running them is the gate's.
- It does not re-run a red-first record's commits to check that the record is true. The record is
  evidence a reviewer reads; phoenix-v2's red-first stage is what re-measures one.
- It does not generate or run mutants. It teaches the row; the repository's mutation runner runs
  it.
