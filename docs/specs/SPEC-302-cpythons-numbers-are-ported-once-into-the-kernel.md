# SPEC-302: the kernel holds CPython's numbers, ported once

- **Wave:** W4. **Issue:** #540 (epic #5). **Context(s):** `deck-streak-kernel` (the numeric
  module `pynum`); `deck-streak-analytics` (its compensated sum delegates to the kernel's); `repo`
  (the parity oracle's registrations and goldens).
- **Decided by:** ADR-012 (the parity oracle proves the math) and ADR-090 (CPython's numeric
  semantics are ported once, into the kernel). No new ADR: ADR-090 decides the placement, and this
  SPEC builds the slice of it that nothing else waits on.
- **Carries:** SPEC-090 R1 and SPEC-095 R1, as this SPEC's R1 and R2. Both SPECs stay in
  `docs/specs/planned/` and each replaces its own lines with one line that names this SPEC.
- **Prerequisites:** SPEC-029 (the parity oracle) and SPEC-071 (the compensated sum this SPEC
  moves). The slice needs nothing from SPEC-077. **Mutation band:** `S30200-S30299`.
- **Status:** built. It holds `docs/red-first/SPEC-302.md` (ADR-016).

## 1. The problem, measured

- **The numbers the W4 ports read are CPython's, and each differs from the obvious Rust at an
  edge.** ADR-090 lists them. Four edges decide most of the difference, each measured on the
  predecessor's interpreter (CPython 3.12):
  - `round(2.675, 2)` is `2.67`: it rounds the float's exact binary value, `2.67499999999999982236431605997495353221893310546875`,
    and not its shortest decimal, which is a tie.
  - The nearest-rank percentile of `gamification/adaptive.py:percentile` is
    `ordered[max(1, ceil(pct * n)) - 1]`, and `0.9 * 70` is `63.00000000000001` as a float, so the
    rank is 64 where the exact product would give 63.
  - `statistics.mean` sums its data exactly and rounds once; a running float sum lands on another
    float for lists with cancellation.
  - `random.Random(n)` seeds the Mersenne Twister from the 32-bit words of `abs(n)` through
    `init_by_array`, so a seed past 32 bits is two words and `0` is the one-word key `[0]`.
- **A second copy would drift.** At the base, `crates/analytics/src/metrics.rs` holds the one
  compensated `sum` (`python_sum`, private to analytics). Curriculum, insights and analytics may
  not depend on one another (ADR-002), so each would copy it. ADR-090 puts one copy in the kernel.

## 2. Requirements

R1. `crates/kernel/src/pynum.rs` holds CPython's float semantics the W4 ports read: the
    compensated `sum`, `statistics.median`, `statistics.mean` (the exact mean, rounded once, which
    a running float sum is not), `round(x, n)` and the predecessor's nearest-rank percentile
    (`gamification/adaptive.py:percentile`). Each equals its golden (`goldens/pynum_basics.json`,
    `goldens/percentile.json`), and analytics' compensated sum (SPEC-071) delegates to the kernel's,
    keeping its own name. (SPEC-090 R1.)

R2. `pynum` also holds CPython's Mersenne Twister, seeded from an integer as `random.Random(n)`
    seeds it, its 53-bit `random()` and its unweighted `choices`, equal to
    `goldens/pynum_random.json`; and `lgamma` for a finite argument above zero, equal to
    `goldens/pynum_lgamma.json` (CPython's `math.lgamma`). (SPEC-095 R1.)

R3. Every function is a pure function of its arguments, or of a generator's own state: none reads a
    clock, the environment or a file, so a port that calls one reads the same number on every run.

R4. The two rows of §7 are each killed by the one test they name.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the kernel's sum, median, mean and round equal CPython's golden over float edge cases | `the_numeric_basics_match_cpythons_golden` |
| A2 | the nearest-rank percentile equals the golden of `adaptive.py:percentile`, the float edge included | `the_percentile_matches_the_predecessors_golden` |
| A3 | the generator and `choices` equal CPython's for every golden seed | `the_generator_matches_cpythons_golden` |
| A4 | `lgamma` equals CPython's for every golden argument | `lgamma_matches_cpythons_golden` |

```acceptance
A1: cargo test -p deck-streak-kernel --test pynum_goldens -- --exact the_numeric_basics_match_cpythons_golden
A2: cargo test -p deck-streak-kernel --test pynum_goldens -- --exact the_percentile_matches_the_predecessors_golden
A3: cargo test -p deck-streak-kernel --test pynum_random -- --exact the_generator_matches_cpythons_golden
A4: cargo test -p deck-streak-kernel --test pynum_random -- --exact lgamma_matches_cpythons_golden
```

Analytics' existing tests stay green after its sum delegates to the kernel's (R1).

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/src/pynum.rs` | `deck-streak-kernel` | added: CPython's sum, median, mean, round, the nearest-rank percentile, the Mersenne Twister, `choices` and `lgamma` (ADR-090) |
| `crates/kernel/src/lib.rs` | `deck-streak-kernel` | changed: the numeric module |
| `crates/kernel/Cargo.toml` | `deck-streak-kernel` | changed: the test reader reads floats exactly (`float_roundtrip` on the dev-dependency) |
| `crates/kernel/tests/pynum_goldens.rs` | `deck-streak-kernel` | added: A1, A2 |
| `crates/kernel/tests/pynum_random.rs` | `deck-streak-kernel` | added: A3, A4 |
| `crates/kernel/tests/pynum_edges.rs` | `deck-streak-kernel` | added: the round, lgamma, mean and sum edge rows against CPython (mutation coverage) |
| `changelog.d/pynum-540.md` | repo | added: the changelog fragment |
| `crates/analytics/src/metrics.rs` | `deck-streak-analytics` | changed: its compensated sum delegates to the kernel's |
| `tools/parity-oracle/registry/spec_302.py` | repo | added: the four registrations |
| `tools/parity-oracle/goldens/pynum_basics.json` | repo | added: CPython's `sum`, `statistics.median`, `statistics.mean` and `round` (adapter) |
| `tools/parity-oracle/goldens/percentile.json` | repo | added: the golden of `gamification/adaptive.py:percentile` (function) |
| `tools/parity-oracle/goldens/pynum_random.json` | repo | added: CPython's `random.Random`, `random` and `choices` (adapter; seeds 0, 1, `20260803` and one past 32 bits) |
| `tools/parity-oracle/goldens/pynum_lgamma.json` | repo | added: CPython's `math.lgamma` (adapter; small, integral and large arguments) |
| `scripts/mutation-rows.d/S30200-S30299.json` | repo | added: the two rows of §7 |
| `docs/specs/planned/SPEC-090-*.md` | repo | changed: R1, A1, A2, their manifest, parity and mutation lines each become one line naming this SPEC |
| `docs/specs/planned/SPEC-095-*.md` | repo | changed: the same, for R1, A1, A2 and their lines |
| `docs/schematics/pynum.md` | repo | added: the module's components and the golden flow |
| `docs/red-first/SPEC-302.md` | repo | added |

## 5. What this does NOT do

- It ports no `shuffle` and no `math.erfc`; SPEC-097 owns both (#540).
- It ports `lgamma` for a finite argument above zero only. CPython's reflection for a negative
  argument has no caller in the W4 ports, so it is not built (#540).
- It builds nothing else of SPEC-090 or SPEC-095: the forecast, the goal, the balance, the
  syllabus tool and the weekly reads each keep their own delivery (#540).
- It weights no `choices`: the instruments read only the unweighted form (#540).
- It seeds from an integer only, never from a string or bytes, which nothing reads (#540).

## 6. Risks

- **A float edge moves a percentile rank, a median or a rounded value.** Detected by A1 and A2,
  whose goldens hold the `0.9`-of-70 rank, an even count and `round(2.675, 2)`.
- **A seed past 32 bits seeds one word.** Detected by A3, whose fourth seed is two words.
- **The kernel's `ln` differs from the interpreter's libm.** `lgamma` is compared bit for bit, so
  A4 would fail on such a platform rather than pass a near value.
- **The golden reader parses a float one bit away.** The kernel's test reader enables
  `float_roundtrip`, as the other float-proving crates do.

## 7. The mutation rows

Band `S30200-S30299`. The two rows moved here from SPEC-090 and SPEC-095, which numbered them
`S09006-NEAREST-RANK` and `S09501-CHOICES-FLOOR`.

| row | target | what it guards | killer |
|---|---|---|---|
| `S30201-NEAREST-RANK` | `crates/kernel/src/pynum.rs` | the rank's ceiling and its floor of 1 | `pynum_goldens::the_percentile_matches_the_predecessors_golden` |
| `S30202-CHOICES-FLOOR` | `crates/kernel/src/pynum.rs` | `choices` takes the floor of `random() * n` | `pynum_random::the_generator_matches_cpythons_golden` |

## 8. Parity goldens

Registered in `tools/parity-oracle/registry/spec_302.py` and generated on the owner's checkout of
the predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `pynum_basics` | CPython's `sum`, `statistics.median`, `statistics.mean`, `round` | adapter | float lists with cancellation, even and odd counts, lists whose running sum differs from the exact mean, halves at each digit, and `round(2.675, 2)`, whose shortest decimal is a tie but whose binary value is not (expected 2.67) |
| `percentile` | `gamification/adaptive.py:percentile` | function | none: empty, one value, `0.9` of 70 values, `0.2` and `1.0`, `0.9` of 7 distinct values (rank 6.3, which the ceiling takes to 7) and `0.0` of 70 values (rank 0, which the floor takes to 1) |
| `pynum_random` | CPython's `random.Random`, `random` and `choices` | adapter | sequences of draws, then `choices` over short lists from the same generator, from four seeds |
| `pynum_lgamma` | CPython's `math.lgamma` | adapter | arguments from 1 to `10**6`, integral and not |

## 9. References

Issue #540; ADR-002; ADR-012; ADR-016; ADR-090; SPEC-029; SPEC-071; SPEC-090 R1; SPEC-095 R1.
