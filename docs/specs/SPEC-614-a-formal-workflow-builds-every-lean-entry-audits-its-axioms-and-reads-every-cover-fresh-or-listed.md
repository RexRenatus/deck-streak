# SPEC-614: a formal workflow builds every Lean entry, audits its axioms, and reads every cover fresh or listed

- **Issue:** #806. **Context(s):** the formal entries (`formal/`), the Kani harness modules
  (`crates/*/src/kani_proofs.rs`), CI (`.github/workflows`), `scripts/` and `docs/`.
- **Decided by:** ADR-636 (the formal entries are checked in a workflow of their own, and the TLA+
  models stay hand-run); ADR-400, whose confirmation names the Lean entry `lean/ReplayHistory`.
- **Schematic:** `docs/schematics/formal-ci.md`, new.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-614.md`. **Mutation band:** S61400-S61499. **Models:** none: this delivery
  reads the formal entries and changes none of them.

## 1. The problem, measured

Read at dev `b065fcee`, by `git show <dev>:<path>`, `git grep` and `git ls-tree`:

- No workflow checks a formal entry: `git grep -n -i -E 'lean|formal|lake |elan|tlc|tla2tools'
  <dev> -- .github/workflows` prints 7 lines, two `cargo clean --workspace`, one `type: boolean`,
  two `- name: clean` and two `ios_lane.py clean`, none a formal check. The required Lean entry
  `lean/ReplayHistory` (ADR-400) is held only by hand runs of the formal checker.
- The entries cover 332 spans: `git grep -h -c '@phx covers' <dev> -- 'formal/lean/Formal/*.lean'
  'formal/tla/*/*.tla' 'crates/*/src/kani_proofs.rs'`, summed, reads 103 in Lean entries, 218 in
  TLA+ entries and 11 in the two Kani harness modules, over 98 distinct covered paths. Each line is
  `covers <path> anchor=<item> digest=sha256:<64 hex>`, and an edit to a covered item leaves its
  entry stale with no red anywhere.
- One cover is stale on dev: `tla/HabitXpFollowsItsLog` covers
  `crates/coordination/src/level_up.rs` `announce_level_up`, whose span reads another digest at
  dev than the one recorded at its stamp commit `7ee0b568`, where the recorded digest reproduces.
- `formal/lean` holds 32 modules whose names do not end `Vectors.lean`, and 195 `@phx theorem` or
  `@phx witness` lines in 31 modules (`git grep -l -E '@phx (theorem|witness)'`). The lakefile's one
  library builds every module by its glob `Formal.*`, but the root `Formal.lean` imports 10 modules
  (`grep -c -E '^import Formal\.'`), so 21 declaring modules are unreachable from `import Formal`.
- `config/formal.json` allows the axioms `propext`, `Classical.choice` and `Quot.sound`;
  `formal/lean/lean-toolchain` reads `leanprover/lean4:v4.34.0`, and the lakefile requires no
  package. The release's Linux archive `lean-4.34.0-linux.tar.zst` is 580367391 bytes, sha256
  `caaa98356098c85dc0fcbbd28e1ec66f39eb6551829972b752ff20e1286b646b` (the release API).
- The Kani entries are the two harness modules `crates/media/src/kani_proofs.rs` and
  `crates/sync/src/kani_proofs.rs`, with 8 `@phx harness` lines; no workflow names Kani.

## 2. Requirements

R1. `scripts/formal_ci.py covers --root <dir>` reads every `@phx covers` line of every
    `formal/lean/Formal/*.lean`, `formal/tla/*/*.tla` and `crates/*/src/kani_proofs.rs` file (entry
    `lean/<module>`, `tla/<folder>` or `kani/<crate>`), finds each item's span by the rule below and
    hashes it. It prints `examined <n> covers` first and refuses 0 (exit 2), then one line per
    finding, `STALE <entry> <path>::<anchor> is sha256 <now>, the entry recorded <then>` or
    `UNRESOLVED <entry> <path>::<anchor> <why>`, then a tally, and exits 1 on any finding R3 does not
    list exactly, else 0. The rule: a cover's digest is the sha256 of its item's span, every line
    from the definition to the item's end, each followed by a newline; the anchor names one item (a
    Rust item by its keyword, a shell function, or a Python `def` or `class`); a name defined twice
    in one file resolves to nothing; a cover path is a `.rs`, `.py` or `.sh` file.
R2. The readers reproduce every committed cover at its own stamp commit.
R3. `config/formal-stale.json` (`{"schema": "formal-stale.v1", "covers": [...]}`, each row `entry`,
    `path`, `anchor`, `recorded`, `now` and a one-line `cure`) lists exactly the covers stale at the
    base. A listed cover that reads fresh, that no entry declares, or whose current digest is not its
    `now` is itself a finding, so the list hides no further move and a re-stamp must delete its row.
R4. `audit-source --root <dir> --out <file>` writes one Lean file holding one `import
    Formal.<Module>` line for every module that declares a `@phx theorem` or `@phx witness`, then
    `#print axioms Formal.<Module>.<name>` for each such declaration, each once, and no other line,
    and prints `examined <n> declarations` (refusing 0). `audit --root <dir> --output <file>` reads
    Lean's output for that file: every declaration printed once, each axiom set inside
    `config/formal.json` `axioms`. `sorryAx`, any other axiom, a declaration missing from the output
    and an output line it cannot place are each a finding (exit 1).
R5. `.github/workflows/formal.yml`, workflow `formal`, runs one job `formal` on `ubuntu-24.04` with
    a timeout, a read-only token, the `ci.yml` concurrency form under its own group prefix, on every
    pull request into dev and main (opened, synchronize, reopened) and every push to them, with no
    cache and no secret. Its steps, in order: the checkout; the covers check; the Lean install named
    by `lean-toolchain`, checksum verified; `lake build`; the axiom audit.
R6. The TLA+ models and the Kani harnesses are not run in CI: they stay with the hand-run formal
    checker, at every delivery that changes a TLA+ entry, a Kani harness module or a span either
    covers (ADR-636 D3). R1 makes such a change visible, because a moved span reads stale.
R7. The check is a workflow of its own, not a `ci.yml` job, so it is a check of its own; whether it
    becomes required is the owner's ruleset decision.
R8. The vectors files and their headers are not read by this check.

## 3. Acceptance criteria of SPEC-614

| id | criterion | decided by |
|---|---|---|
| A1 | a moved span reads `STALE` by entry, path and anchor, with both digests (R1) | `test_formal_ci.py` |
| A2 | an anchor absent or defined twice reads `UNRESOLVED` (R1) | `test_formal_ci.py` |
| A3 | the Rust, Python and shell readers each reproduce a digest computed from the rule (R1, R2) | `test_formal_ci.py` |
| A4 | every cover at the head is fresh or listed exactly, over every `@phx covers` line of the R1 files (R1, R3) | `test_formal_ci.py` |
| A5 | a listed row that reads fresh, that no entry declares, or whose digest moved again is a finding (R3) | `test_formal_ci.py` |
| A6 | an axiom outside the allowed set, `sorryAx`, a missing declaration or an unplaced line is refused (R4) | `test_formal_ci.py` |
| A7 | the audit source names every declared theorem and witness once and imports each declaring module (R4) | `test_formal_ci.py` |
| A8 | the workflow runs its checks in order on every pull request and push, read-only (R5) | `test_formal_ci.py` |
| A9 | the Lean install is the toolchain pin, checksum verified (R5) | `test_formal_ci.py` |

```acceptance
A1: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_a_moved_span_reads_stale_by_entry_path_and_anchor
A2: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_an_anchor_absent_or_defined_twice_is_unresolved
A3: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_each_reader_reproduces_a_digest_written_by_the_rule
A4: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_every_cover_at_head_is_fresh_or_listed_exactly
A5: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_a_listed_row_fresh_undeclared_or_moved_again_is_a_finding
A6: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_an_axiom_outside_the_allowed_set_or_sorry_is_refused
A7: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_the_audit_source_names_every_declared_theorem_and_witness_once
A8: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_the_formal_workflow_runs_its_checks_in_order_on_every_pull_request_and_push
A9: python3 -B -m unittest discover -s scripts/tests -p test_formal_ci.py -k test_the_lean_install_is_the_toolchain_pin_checksum_verified
```

A1, A2, A3, A5 and A6 build their own trees in a temporary folder. A4 and A7 read the live tree,
each counting its population from the entry files and refusing zero; A8 and A9 read the workflow
as text, each beside a planted copy it refuses.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-614-a-formal-workflow-builds-every-lean-entry-audits-its-axioms-and-reads-every-cover-fresh-or-listed.md` | docs | new: this file |
| `docs/decisions/ADR-636-formal-entries-are-checked-in-a-workflow-of-their-own-and-the-tla-models-stay-hand-run.md` | docs | new |
| `docs/schematics/formal-ci.md` | docs | new |
| `changelog.d/formal-ci-806.md` | changelog | new |
| `docs/red-first/SPEC-614.md` | records | new |
| `scripts/tests/test_formal_ci.py` | tests | new: A1 to A9 |
| `scripts/formal_ci.py` | checker | new: the `covers`, `audit-source` and `audit` verbs (R1, R3, R4) |
| `scripts/mutation-python.json` | rows | changed: one row mapping `scripts/formal_ci.py` to `test_formal_ci` |
| `config/formal-stale.json` | config | new: the covers stale at the base (R3) |
| `scripts/mutation-rows.d/S61400-S61499.json` | rows | new: S61401 to S61403 |
| `.github/workflows/formal.yml` | ci | new: the `formal` workflow (R5) |

## 5. What this does NOT cover

- TLC over the TLA+ models and the Kani verifier over the harnesses in CI: they stay hand-run, as
  ADR-636 D3 records, which is the other arm of the close condition of #806.
- Making `formal` a required check: a ruleset change, the owner's (#806).
- The re-stamp of the stale cover the list carries: owed by the delivery that re-reads its model,
  which then deletes the row (#806).
- The vectors files and their headers (R8), which the formal checker compares by hand (#806).
- The lakefile and the Kani budget in `config/formal.json`, which open pull requests change (#874,
  #1131); this delivery changes neither.

## 6. Risks

- **A reader that differs from the formal checker's.** A port that reads one span otherwise would
  call a fresh cover stale or a stale one fresh. Detected by the reader proof over every cover whose
  path the hand-back may read, each at its own stamp commit, and by A3's fixtures, each digest
  computed from the rule in the test.
- **A Lean release that moves.** The archive is fetched by its exact version and refused unless its
  sha256 matches; a toolchain pin that names another version fails the step before any download.
- **A stale list that grows quietly.** A row must name both digests, so a cover that moves again
  reads as a finding (A5); a fresh row is a finding too, so a re-stamp must delete it.
- **Lean output this reader cannot place.** Every line it cannot place is a finding (A6), never
  skipped.
