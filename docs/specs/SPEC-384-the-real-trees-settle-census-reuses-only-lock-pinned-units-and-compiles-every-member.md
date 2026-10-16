# SPEC-384: the real tree's settle census reuses only lock-pinned units, and compiles every member itself

- **Issue:** #692. **Context(s):** progression's census test (`crates/progression/tests/`), the
  mutation rows (`scripts/mutation-rows.d/`), the formal models (`formal/tla/`) and their docs.
- **Decided by:** ADR-398 (the real tree's census keeps a store of lock-pinned units, and every
  member unit it reads is one it compiled).
- **Amends:** SPEC-072 section 14's "An empty target for each census", by an insert-only section
  appended to SPEC-072 (section 18 below names its text). It lands only after the owner's signed ruling
  `docs/rulings/OWNER-RULING-2026-10-16-census-store.md`, which admits that amendment as a
  bounded weakening, is on `dev`.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-384.md`. **Mutation band:** `S38400-S38499`.

## 1. The problem, measured

Every figure below was read at `e7ecf10d`, or from a CI job named by its id.

- **Which test.** `only_progression_writes_xp_settlement_and_only_coordination_settles`
  (`crates/progression/tests/xp_census.rs`:1315-1445) is the one census of the real tree: its
  `census_in(&root(), …"settle-census")` call (:1318-1321) is the only call on `root()`. Every
  other census in the file judges a planted tree. `crates/progression/tests/ledger_census.rs` reads
  text and runs no cargo.
- **It runs for every progression mutant.** SPEC-327 R7 (:79-83) keeps both census tests in every
  mutant's run; `.cargo/mutants.toml`:17 and :23 run them under nextest's `mutants` profile, and
  `.config/nextest.toml`:8-9 stops a run at its first failure.
- **Its time.** SPEC-362 section 1 (:43-48) records it as the tree's slowest test: 1144.5 s on run
  37410215011, 1378.452 s on push run 37392351782 and 1441.022 s on push run 37445796358. On the
  push run at `e7ecf10d` (run 37930837063, `rust` job 113820846590) the whole `test` stage took
  2220 s. The per-test split at `e7ecf10d` is in that job's stage-log artifact, which this SPEC
  does not read (section 8 reads it after the merge).
- **Its shape.** The census makes one empty target (`census_in`, :1191-1203) and runs 16
  `cargo check --workspace` passes in it (`compiled_uses`, :914-930): the selections
  `--all-targets` and the plain selection, each under both combinations of the one declared feature
  (`deck-streak-ffi`'s `bindgen`), each under four (debug-assertions, panic) pairs. `pass()`
  (:496-511) sets `debug-assertions` for every package of the graph by name, so the four pairs are
  four distinct sets of units.
- **What it compiles.** `Cargo.lock` holds 646 packages (`grep -c '^\[\[package\]\]'`): 608 from
  the registry and 7 from git (`grep -c '^source = "registry+'`, `grep -c '^source = "git+'`), and
  31 workspace members. The 615 that are not members are pinned by the lock: they are the same for
  every mutant. The census reads the messages of member units only (:940-944).
- **Where the time goes.** Each census compiles the 615 pinned units cold under each of the four
  pairs, and again where a selection changes their features. The members alone are a small share:
  `clippy` over every member and target, with the dependencies already built, took 45 s in job 113820846590.
  The split between pinned units and members inside the census is inferred from
  that, not measured; section 8 measures the result.

## 2. Requirements

R1. The real tree's census compiles in a **store**: a directory outside `target/`, under the test
    process's temporary directory, named by the key of R3. Between censuses the store holds only
    units of packages whose source is `registry+` or `git+` in `Cargo.lock`.
R2. Before its first pass, the census removes from the store every unit of every path package of
    the graph (`cargo clean`, one `-p` per member, `--target-dir` the store, under the census's own
    environment). Each member unit a pass reads was compiled by this census: a member
    `compiler-artifact` message with `"fresh": true` whose files no earlier pass of the same census
    compiled (`"fresh": false`) is refused by name, and the census's verdict carries the refusal.
R3. The store's key is the SHA-256 of: the bytes of `Cargo.lock`, the bytes of
    `rust-toolchain.toml`, the flags and the probe's arming that `cargo()` sets, the cargo program,
    each name in `INHERITED` with its value or its absence, and the canonical tree root. Equal
    inputs give one key; a change to any one gives another.
R4. A census holds an exclusive lock on its store's lock file from before the removal of R2 to after
    its last pass. A second census of the same store waits for it.
R5. Every planted tree's census is unchanged: `census_in` still compiles in an empty target of its
    own, and row S07295's find text stays byte-identical at its one site.
R6. The census still runs whole for every progression mutant. No test in `xp_census.rs` gains
    `#[ignore]`, a skip, a filter, a sampled or shrunk population, or a weaker assertion.
    `.cargo/mutants.toml`, `.config/nextest.toml` and every workflow are unchanged, and so are the
    per-mutant `--timeout` and the sizer's `CENSUS_SECONDS` and `BASELINE_SECONDS`.
R7. The real-tree test prints, beside its existing examined counts (unchanged by name), the passes
    the census ran, the member units it compiled, the member units it read and the store's units it
    reused. It asserts that every pass read member units, that every member unit read was compiled
    by this census, and that no refusal of R2 occurred.
R8. Every census-only kill is still a kill: rows S07274, S07275 and S07283, killed only by the
    real-tree test, read KILLED at the delivery's head, and S07295 reads KILLED by its planted
    killer.
R9. SPEC-072, ADR-197 and ADR-328 gain insert-only text naming this exception (section 4), and the
    mutation schematic gains its section 12.
R10. A TLA+ entry, `formal/tla/CensusStore`, models two censuses over one store and holds that an
    accepted verdict reads only member units its own census compiled; each of its two witnesses is
    caught.

## 3. Acceptance criteria of SPEC-384

| id | criterion | decided by |
|---|---|---|
| A1 | a planted tree with a member and a git dependency, judged twice on one store: the second census reuses at least one unit of the git package, compiles every member unit it reads, and both verdicts equal the verdict `census_in` reaches in an empty target (R1, R2, R5) | `xp_census.rs::a_second_census_on_one_store_reuses_only_pinned_units_and_compiles_every_member` |
| A2 | a store that already holds a member unit, with the removal of R2 withheld, makes the census refuse that unit by name (R2) | `xp_census.rs::a_member_unit_the_census_did_not_compile_is_refused_by_name` |
| A3 | after a planted census on a store and the removal of R2, no file under the store's fingerprint, deps or build directories belongs to a path package, and the git package's units remain (R1, R2) | `xp_census.rs::the_removal_leaves_no_unit_of_a_path_package` |
| A4 | the key changes when any one of its inputs changes alone, and is equal for equal inputs (R3) | `xp_census.rs::a_store_is_keyed_by_the_lock_the_toolchain_the_environment_and_the_root` |
| A5 | while a census holds its store, another handle's `try_lock` on the store's lock file reads `WouldBlock`, and succeeds once the census has ended (R4) | `xp_census.rs::a_census_holds_its_store_locked_until_its_last_pass` |
| A6 | the real-tree census runs on its store, prints the counts of R7 and asserts them (R7) | `xp_census.rs::only_progression_writes_xp_settlement_and_only_coordination_settles` |

```acceptance
A1: cargo test -p deck-streak-progression --test xp_census -- --exact a_second_census_on_one_store_reuses_only_pinned_units_and_compiles_every_member
A2: cargo test -p deck-streak-progression --test xp_census -- --exact a_member_unit_the_census_did_not_compile_is_refused_by_name
A3: cargo test -p deck-streak-progression --test xp_census -- --exact the_removal_leaves_no_unit_of_a_path_package
A4: cargo test -p deck-streak-progression --test xp_census -- --exact a_store_is_keyed_by_the_lock_the_toolchain_the_environment_and_the_root
A5: cargo test -p deck-streak-progression --test xp_census -- --exact a_census_holds_its_store_locked_until_its_last_pass
A6: cargo test -p deck-streak-progression --test xp_census -- --exact only_progression_writes_xp_settlement_and_only_coordination_settles
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/progression/tests/xp_census.rs` | progression | the store, its key, its lock and the removal of R2; the refusal of R2 in its own small function called from `compiled_uses`; the counts of R7; A1 to A5; A6's assertions; the module doc's census sentence |
| `formal/tla/CensusStore/CensusStore.tla` | formal | added: the model, its covers of the store's functions |
| `formal/tla/CensusStore/MCCensusStore.cfg` | formal | added: two censuses, two trees, one store |
| `formal/tla/CensusStore/witness/AcceptedVerdictReadsItsOwnMembers.cfg` | formal | added: the lock off |
| `formal/tla/CensusStore/witness/NoMemberSurvivesTheRemoval.cfg` | formal | added: the removal off |
| `scripts/mutation-rows.d/S38400-S38499.json` | scripts | added: this delivery's rows (section 9) |
| `docs/red-first/SPEC-384.md` | docs | added: each criterion's red, then its green |
| `docs/specs/SPEC-384-the-real-trees-settle-census-reuses-only-lock-pinned-units-and-compiles-every-member.md` | docs | added |
| `docs/decisions/ADR-398-the-real-trees-census-keeps-a-store-of-lock-pinned-units-and-compiles-every-member.md` | docs | added |
| `docs/specs/SPEC-072-xp-is-earned-per-review-and-per-day-settled-once-per-source-with-levels-and-titles.md` | docs | insert-only: section 18 appended |
| `docs/decisions/ADR-197-the-settle-census-reads-progressions-own-reexports.md` | docs | insert-only: a closing pointer to ADR-398 |
| `docs/decisions/ADR-328-the-per-mutant-budget-is-1200-seconds-and-the-sizer-charges-the-census.md` | docs | insert-only: a closing pointer to ADR-398 |
| `docs/schematics/mutation-testing.md` | docs | insert-only: section 12 appended |
| `docs/schematics/ci-jobs-and-caches.md` | docs | insert-only: one paragraph after the census's diagram |
| `changelog.d/census-speed-384.md` | docs | added |

No `Cargo.toml` changes: `sha2` and `tempfile` are already progression's dev-dependencies
(`crates/progression/Cargo.toml`:34 and :36).

## 5. What this does NOT cover

- It does not speed the planted censuses, the killer `the_census_refuses_every_caller_the_compiler_finds`
  included: each planted tree still compiles in an empty target of its own (#917).
- It does not speed the `rust` job's census: each CI job starts with an empty store, so the job's one
  census of the real tree is still cold (#508).
- It does not lower a price or the timeout. `CENSUS_SECONDS`, `BASELINE_SECONDS` and `--timeout` stay
  as they are until a run's reports measure the new census (#597).
- It does not close the disclosed class of build-time code in packages that cannot name `settle`;
  within one store that code's output is kept rather than re-run, and the class stays disclosed
  (#445).
- It does not touch the web mutation leg (#693) or the release's mutants (#691).

## 6. Risks

- **A kept unit moves a verdict.** A pinned package's build script or proc-macro whose output
  depends on host state outside the tree is not re-run within one store. Detection: none by a test;
  the class is #445's. Bound: a CI job's store is born empty with the job.
- **The removal misses a member's unit kind.** A3 checks the fingerprint, deps and build
  directories; R2's refusal catches any member artifact the census did not compile, so a miss reads
  as a refusal, not as a pass.
- **Two censuses of one checkout overlap.** R4's lock serialises them; A5 holds the lock, and the
  model's first witness shows what an unlocked store admits.
- **The store grows on a developer's machine.** A new key per lock file leaves the old store in
  place, outside `target/`, where `cargo clean` does not reach it. Detection: none by a test.
- **The store cannot be written.** The census refuses "the workspace does not compile", loudly.
- **The gain is smaller than the pinned units' share suggests.** Section 8 reads the time.

## 7. Proved by the builder at its head

| id | what | record |
|---|---|---|
| P1 | the real-tree test's existing examined counts are equal by name at the cut and at the head | the builder's two `--nocapture` runs, quoted in `docs/red-first/SPEC-384.md` |
| P2 | rows S07274, S07275 and S07283 read KILLED by the real-tree test, and S07295 by its planted killer, at the head, each beside a passing control | the builder's per-row runs, quoted in the same record |
| P3 | a warm-store census and a cold census of one tree report the same killed and survived set: the unmutated tree passes, and rows S07274, S07275 and S07283 read KILLED, each with the store emptied first and again with the store warm, every run named (the owner's ruling, before anything relies on the store) | the builder's paired runs, each named by its command, its head and its log, and the pull request's CI run by id, quoted in the same record |
| F1 | `formal/tla/CensusStore`'s properties hold at its configuration and each of its two witnesses is caught (R10) | the repository's formal check of that one entry, quoted in the same record |

## 8. Read after the merge

| id | criterion | read by |
|---|---|---|
| L1 | on the first CI leg after landing that runs two or more progression mutants, the census test's time on its second and later mutants is read by job id and recorded beside 1441.022 s | that leg's log, by job id |

## 9. The mutation rows

`scripts/mutation-rows.d/S38400-S38499.json`, `MUTATIONS`, package `deck-streak-progression`, path
`tests/xp_census.rs` (relative to the crate, as the S07200 band writes it). Each find text is taken
from the delivered code and occurs once, and each replacement keeps every item used.

| row | mutant | killer |
|---|---|---|
| S38400 | the removal of R2 is skipped | A3 |
| S38401 | the refusal of R2 never fires | A2 |
| S38402 | the key leaves out the bytes of `Cargo.lock` | A4 |
| S38403 | the key leaves out the values of `INHERITED` | A4 |
| S38404 | the key leaves out the tree root | A4 |
| S38405 | the store's lock is not taken | A5 |
| S38406 | the count of member units compiled is not incremented | A1 |

The real tree's census moved back to an empty target is an equivalent mutant (a slower census,
the same verdict) and has no row.

## 10. References

SPEC-072 section 14 and its rows S07290 to S07301; ADR-197's round 8; SPEC-327 R7; SPEC-362
section 1; ADR-328's option B; ADR-199 (the timeout is a signed act); `docs/rulings/OWNER-RULING-2026-10-16-census-store.md`; #692; #597; #917; #508; #445.
