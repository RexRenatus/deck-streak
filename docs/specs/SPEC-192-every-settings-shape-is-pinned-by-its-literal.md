# SPEC-192: every setting's shape is pinned by its literal, so a rewritten shape fails a test

- **Wave:** W4. **Issue:** #336. **Context(s):** `deck-streak-agent`, `-analytics`, `-api`, `-bot`,
  `-daemon`, `-identity`, `-ingest`, `-kernel`, `-readings`, `-vault` (tests only) and `repo`
  (`scripts/`).
- **Decided by:** ADR-192 (this SPEC's own: where the rows live and what the guard reads, and what
  each was chosen against), ADR-057 (the mutation rows) and SPEC-039 (the row runner and census).
- **Status:** delivered. It holds `docs/red-first/SPEC-192.md`.

## 1. The problem, measured

- **A setting's refusal names its shape.** Every `impl Setting for` carries `const SHAPE`, and a
  malformed value is refused as `the setting <NAME> is malformed: it must be <SHAPE>` (the kernel's
  `SettingsError::Malformed`). The words are what an operator reads.
- **A mutant of the words passed for most of them.** The audit of #336 planted each `SHAPE` twice
  (the middle letter to `Q`, and `XX<shape>XX`) and ran each crate's tests whole. Of 22
  implementations on dev, 14 survived both plants: the tests refused the value with `is_err()` or
  `matches!`, or compared the error against `Hour::SHAPE`, which reads the code under test back to
  itself. #354 had already pinned five others.
- **Dev now holds 24 implementations** (`git grep -n 'impl .*Setting for' -- crates | grep /src/`):
  the audit's 22, `AiRoute` (agent `route.rs`, pinned by the row S04302) and `RequestFile` (daemon
  `sync_request.rs`, never measured).
- **Nothing stops the next one.** A new `impl Setting for` with a shape no test spells passes every
  gate.

## 2. Requirements

R1. The population is every `impl Setting for`, generic (`impl<T> Setting for`), path-qualified
    (`crate::settings::Setting`) or written by a macro (`impl Setting for $t`), under `crates/*/src/`.
    It is 24 on the base of this delivery and it is enumerated by walking the tree (`pathlib`),
    because the git pathspec `'crates/*/src'` matches nothing.
R2. Each implementation's `SHAPE` literal is spelled, whole and quoted, in a test of its crate (its
    `tests/`, or the `#[cfg(test)]` module of the implementation's own source file; a comment does
    not count), and the assertion compares the whole refusal against it (the setting's name and the
    literal), never against the constant. A literal that two or more implementations of one crate
    share is pinned only by a mutation row on each implementation's own file.
R3. The 14 implementations that survived (`LeechThreshold`, `MiniAppUrl`, `ApiUrl`, `SyncEndpoint`,
    ingest `StateDirectory`, `IncludeDecks`, `LawDeckRoot`, `CoursesPath`, `Hour` with `UtcOffset`,
    `OffloadWorkers`, `CredentialsDirectory`, `TaxonomyPath` and `FolderName`) and `RequestFile` gain
    such an assertion, and each also gets a mutation row in this delivery's band (S19200-S19299),
    in one band file with the one `MUTATIONS` table the tree already uses for Rust targets.
R4. `Freshness` (identity) and `VaultRoot` (vault) are pinned by named tests that already spell the
    literal; they take no row. `AiRoute` is pinned by S04302, `RosterPath` by S05766 and the five
    #354 implementations by their rows; none of the seven is changed.
R5. A guard under `scripts/tests` enumerates the population, reads each literal, and refuses an
    implementation whose literal is spelled by no test of its crate (its `tests/` or the
    `#[cfg(test)]` module of its own file, comments not counting) and is the `find` of no row that
    targets the implementation's own file, and one whose literal another implementation of the
    crate shares and that lacks such a row. It prints `examined <N> Setting impl(s)` and refuses zero.
R6. The delivery changes no production line: `SHAPE` literals, setting names and behaviour are as
    they were.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every `impl Setting for` has a literal that a test or a row pins | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py` |
| A2 | a malformed leech threshold is refused naming its whole shape | `cargo test -p deck-streak-analytics --test rollup_metrics -- --exact a_malformed_leech_threshold_is_refused_naming_its_whole_shape` |
| A3 | a mini app URL that is not https is refused naming its whole shape | `cargo test -p deck-streak-bot --test commands -- --exact the_mini_app_url_is_https` |
| A4 | an API URL that is not https or loopback http is refused naming its whole shape | `cargo test -p deck-streak-bot --test transport -- --exact the_api_url_is_https_or_loopback_http` |
| A5 | each malformed ingest setting is refused naming its whole shape | `cargo test -p deck-streak-ingest --test settings -- --exact each_malformed_setting_is_refused_naming_its_whole_shape` |
| A6 | a malformed courses file path is refused naming its whole shape | `cargo test -p deck-streak-kernel --test courses_config -- --exact the_courses_file_refuses_a_duplicate_or_overlapping_course` |
| A7 | the kernel's hour, offset, worker and credentials settings state their whole shape | `cargo test -p deck-streak-kernel --test settings -- --exact a_malformed_setting_is_refused_by_name_without_its_value` |
| A8 | a malformed taxonomy path is refused naming its whole shape | `cargo test -p deck-streak-readings --test topics -- --exact a_taxonomy_file_that_names_a_deck_badly_is_refused_whole_and_never_quoted` |
| A9 | the vault's folder settings state their whole shape | `cargo test -p deck-streak-vault --test confinement -- --exact the_vault_settings_refuse_by_name_and_never_by_value` |
| A10 | a relative sync request path is refused naming its whole shape | `cargo test -p deck-streak-daemon --test sync_request -- --exact a_relative_request_path_is_refused_naming_its_whole_shape` |
| A11 | the guard examines a generic and a macro impl, and refuses a shape that only its own constant, a production line, a comment, or a read-back of the constant spells, or that two impls of one crate share without a row on each file, or that only another file's row pins | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardJudgesAPlantedTree` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k test_every_setting_impl_has_a_shape_literal_that_a_test_or_a_row_pins
A2: cargo test -p deck-streak-analytics --test rollup_metrics -- --exact a_malformed_leech_threshold_is_refused_naming_its_whole_shape
A3: cargo test -p deck-streak-bot --test commands -- --exact the_mini_app_url_is_https
A4: cargo test -p deck-streak-bot --test transport -- --exact the_api_url_is_https_or_loopback_http
A5: cargo test -p deck-streak-ingest --test settings -- --exact each_malformed_setting_is_refused_naming_its_whole_shape
A6: cargo test -p deck-streak-kernel --test courses_config -- --exact the_courses_file_refuses_a_duplicate_or_overlapping_course
A7: cargo test -p deck-streak-kernel --test settings -- --exact a_malformed_setting_is_refused_by_name_without_its_value
A8: cargo test -p deck-streak-readings --test topics -- --exact a_taxonomy_file_that_names_a_deck_badly_is_refused_whole_and_never_quoted
A9: cargo test -p deck-streak-vault --test confinement -- --exact the_vault_settings_refuse_by_name_and_never_by_value
A10: cargo test -p deck-streak-daemon --test sync_request -- --exact a_relative_request_path_is_refused_naming_its_whole_shape
A11: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardJudgesAPlantedTree
```

A2 to A10 pin a literal the code already carries, so each is recorded `not red`; the proof that
each kills the mutant is its row (R3), run with `python3 scripts/mutation_rows.py prove`, with
`census` and `retired --base <merge base>` green.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-192-every-settings-shape-is-pinned-by-its-literal.md` | repo | added |
| `docs/decisions/ADR-192-a-settings-shape-is-pinned-by-its-literal-and-the-rows-live-in-one-band.md` | repo | added |
| `docs/red-first/SPEC-192.md` | repo | added |
| `scripts/tests/test_setting_shapes.py` | repo | added (the guard) |
| `scripts/mutation-rows.d/S19200-S19299.json` | repo | added (15 rows) |
| `crates/analytics/tests/rollup_metrics.rs` | `deck-streak-analytics` | test changed |
| `crates/bot/tests/commands.rs`, `crates/bot/tests/transport.rs` | `deck-streak-bot` | test changed |
| `crates/daemon/tests/sync_request.rs` | `deck-streak-daemon` | test changed |
| `crates/ingest/tests/settings.rs` | `deck-streak-ingest` | test changed |
| `crates/kernel/tests/courses_config.rs`, `crates/kernel/tests/settings.rs` | `deck-streak-kernel` | test changed |
| `crates/readings/tests/topics.rs` | `deck-streak-readings` | test changed |
| `crates/vault/tests/confinement.rs` | `deck-streak-vault` | test changed |
| `changelog.d/test-setting-shapes-192.md` | repo | added |

## 5. What this does NOT do

- It does not pin constants other than `SHAPE` (a default, a setting's name, a bound): those have
  their own rows and tests, and the audit of #336 measured `SHAPE` only (#336).
- It does not re-price or rename any setting, and it does not change a `SHAPE` literal or a
  refusal's wording (#336).
- It does not add the `#[cfg(test)]` or `tests/` implementations of `Setting` to the guard: none
  exists, and a test-only implementation has no operator to read it (#336).
- It does not give `VaultSettings::from_env` a production caller, although `FolderName` has none
  today; the row and the test pin the shape for the day it does (#336).

## 6. Risks

- **A literal shared by implementations** ("an absolute file path" is `RosterPath`'s in agent,
  `RequestFile`'s in daemon and `CoursesPath`'s in kernel: one crate each today). Two or more
  implementations of one crate that share a literal are not pinned by a spelling in a test: the guard
  asks a mutation row on each implementation's own file, and each row still has to kill its own
  mutant.
- **Two open pull requests edit files this delivery edits** (`crates/bot/tests/commands.rs`,
  `crates/analytics/tests/rollup_metrics.rs`); a conflict is resolved by keeping both sides'
  assertions, and each row's `find` is re-checked to occur once after a merge.

## 7. References

- Issue #336; ADR-192; ADR-057; SPEC-039; SPEC-057 R20; SPEC-020 (the kernel's settings).
