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
    (`crate::settings::Setting`) or written by a macro (`impl Setting for $t`, also through
    `$crate::settings::Setting`), under `crates/*/src/`.
    It is 24 on the base of this delivery and it is enumerated by walking the tree (`pathlib`),
    because the git pathspec `'crates/*/src'` matches nothing.
R2. Each implementation's `SHAPE` literal is spelled, whole and quoted, in a test of its crate (its
    `tests/`, or the `#[cfg(test)]` module of the implementation's own source file; a comment of
    either form, `//` or `/* */`, does not count, and only that module of the own file counts, not a
    line after it), and the assertion compares the whole refusal against it (the setting's name and the
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
    `#[cfg(test)]` module of its own file; comments of both forms not counting) and is the `find`
    of no row that
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
| A11 | the guard examines a generic impl, admits a shape a test of the crate spells or a row on the impl's own file finds, and refuses a shape that only its own constant, a read-back of the constant, a production line or a comment spells, that only another file's row pins, or that two impls of one crate share without a row on each file | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardJudgesAPlantedTree` |
| A12 | the guard examines a macro impl, plain or through `$crate`; admits a shape its own file's `#[cfg(test)]` module spells, and one a test spells after a `//` inside a string; and refuses a shape that only a block comment spells, that only a production line after the own file's test module spells, or that only a production line after a `#[cfg(test)]` opening no module spells | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsRustSource` |

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
A12: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsRustSource
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

- **The guard reads text with a small lexer, not the compiler.** It blanks `//` and nested `/* */`
  comments, keeps strings and character literals, and counts only a `#[cfg(test)]` module of the
  implementation's own file. A construct the lexer does not know (a `#[cfg(test)]` module written
  by a macro) is not counted, so the guard errs toward refusing.
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

## 8. Amendment, 2026-09-29: every lexer arm of the guard is killed by an assertion

Issue #406. The guard reads Rust with a small lexer, and a review measured eight rewrites of the
guard that left it green. Six are rewrites of the lexer: nested block comments untracked, raw strings
unrecognised, character literals unrecognised, block comments kept in the skeleton, strings kept in
the skeleton, and escapes inside strings ignored. Two are rewrites of the selection of spellings: the
implementation's own `SHAPE` constant not excluded, and every other source file read whole instead of
only its test module. This amendment states the rule that closes them and the two readings that
refused a correct tree.

R7. Each of the eight rewrites turns `test_setting_shapes.py` red by an assertion inside a test that
already existed, so no test is renamed or removed and A11 and A12 keep their meaning. So does each
of three further arm rewrites: a `//` comment kept in the skeleton, the `#[cfg(test)]` requirement
deleted from the out-of-line reading, and a raw string closed without its hashes. The unmodified
guard stays green and prints the same `examined 24 Setting impl(s)`.

A second review measured 72 rewrites of the guard's reader, in the lexer, in the out-of-line reading
and in the selection of spellings, and 27 of them left the tests green while changing the guard's
verdict on a tree of valid Rust. Assertions added to existing tests, and one line for the depth check
of R8, turn 66 of the 73 rewrites red (the 72 and the depth check), and the unmodified guard still
prints `examined 24`. Each of the seven that stay green is equivalent on valid Rust:

- deleting the look-behind of the raw-string token (X11) differs only on a C raw string
  `cr#"..."#`, which the rewrite reads more correctly, and #434 tracks that misreading;
- swapping the two comment alternatives of the token pattern (T8) is inert, because `//` and `/*`
  cannot both match at one place;
- closing a raw string without its hashes (R3) leaves the closing hashes visible, and no valid Rust
  puts a `[` after them;
- ending an unterminated raw string at the failed search (R4) concerns a string that valid Rust does
  not hold;
- reading a backslash as a plain character (C3) is never reached, because the escape alternative
  comes first;
- allowing a raw newline in a character literal (C4) concerns a literal that valid Rust does not
  hold;
- leaving a `//` comment at the end of a file with no newline unhandled (L1) leaves at most the
  file's last character unblanked, and one character spells no shape.
R8. An out-of-line `#[cfg(test)] mod name;`, with or without `#[path = "..."]`, is read as the
implementation file's own test module. The guard resolves the file the way rustc does for a
declaration at the top level of the file: the `#[path]` value relative to the implementation file's
directory, otherwise `name.rs` or `name/mod.rs` beside the file (below the file's own directory for a
file that is not `lib.rs`, `main.rs` or `mod.rs`). It does not follow a `mod tests;` declared inside
an inline `mod inner { }`: it reads no file for it, so a shape only that module spells stays
refused. That limit fails closed, and #433 tracks it.
R9. `impl Setting for` is read from comment-free source, so one inside a block comment is not
examined.
R10. Each of the eight rewrites and each of the three further arm rewrites has one row in the band
S19200-S19299 (S19216 to S19228: thirteen rows, because the two new readings have a row each and a
test may serve several). So does each of the 28 arms of the reader that a second review found
unpinned: nine of the lexer, seven of the out-of-line reading, eleven of the selection of spellings,
and the depth check of R8 (S19229 to S19256). That is forty-one rows, S19216 to S19256, each proved
by its full id.

Insertions into the criteria of section 3 (the section is not edited, which keeps this file's earlier
bytes as they were; the criteria are defined below, by insertion of new A-numbers, as SPEC-038 §8
ruling (i) allows):

- A13 (out-of-line test modules) and A14 (an impl in a comment) are new tests, red first.
- A15 is the set of assertions added inside the existing tests of A11 and A12, recorded `not red`
  because the base guard already passed them; the forty-one rows prove each kills its rewrite.

## 9. Amendment acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A13 | an out-of-line `#[cfg(test)] mod`, beside the file, in a `mod.rs` directory, through `#[path]`, or below a `lib.rs`, is the implementation's own test module | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsOutOfLineTestModules` |
| A14 | an `impl Setting for` inside a block comment is not examined | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardIgnoresAnImplementationInAComment` |
| A15 | the strengthened assertions of A11 and A12 refuse each of the six lexer rewrites and the two selection rewrites | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardJudgesAPlantedTree -k TheGuardReadsRustSource` |

```acceptance
A13: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsOutOfLineTestModules
A14: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardIgnoresAnImplementationInAComment
A15: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardJudgesAPlantedTree -k TheGuardReadsRustSource
```

## 10. What this amendment does NOT do

- It does not change any production line, `SHAPE` literal or setting (#406).
- It does not make the lexer a full Rust parser: a `#[cfg(test)]` module written by a macro is
  still not counted (#406).
- It does not follow a `mod tests;` declared inside an inline module, so a shape only that module
  spells stays refused (#433).
- It does not read a C raw string `cr#"..."#`: the guard misreads it, and the misreading fails
  closed (#434).
