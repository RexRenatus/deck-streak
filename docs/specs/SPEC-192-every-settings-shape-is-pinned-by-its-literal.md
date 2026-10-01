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
R8. A module that the implementation's own file declares, inline (`mod name { }`) or out-of-line
(`mod name;`), in any visibility and as `name` or `r#name`, is its test module only when rustc
compiles the declaration under `--cfg test` and not without it, whatever else is configured. The
sentence judges the declaration, and two shapes lie outside it, disclosed and not claimed: a file that
a second declaration compiles without `test` is still read through its test declaration (#458), and
an item that a `cfg` removes inside a compiled test module is still read as test code (#449). The
guard reads Rust source with rustc's lexer, as the Rust Reference's chapter on lexical structure
gives it: every whitespace character the Reference lists, line comments, nested block comments,
outer and inner doc comments (which rustc reads as `doc` attributes), identifiers and raw
identifiers, lifetimes, punctuation, the three delimiters, and every literal form (character, byte,
string, byte string, C string, raw string, raw byte string and raw C string, with any hash count
and suffix, and numbers), after a byte order mark and a shebang. It reads attributes and
declarations as tokens: an attribute is `#`, a `!` when it is inner, and a bracketed token tree,
with any whitespace or comment between them, and a declaration spelled inside a literal, a comment
or a macro's token tree declares nothing. A source that rustc's lexer refuses (a reserved prefix,
a literal or comment never closed, an unpaired delimiter, a character that starts no token) is
never read as a test.

The guard reads the declaration's whole attribute run: the declaring file's inner attributes, the
outer attributes, which must follow the end of an item or the file's inner attributes, and the
inner attributes that open the module's body or file. It evaluates the run in three-valued logic
in which `test` is the one known option and `true` and `false` hold their values. `all`, `any` and
`not` combine true, false and unknown; any other option or key-value (`unix`, `debug_assertions`,
`feature = "slow"`), and `test` spelled as a raw identifier, is unknown; a `cfg` keeps the module
when its predicate holds; a `cfg_attr` keeps it when its predicate fails or when every attribute it
applies keeps it; and any other attribute keeps it, unless it names `cfg` or `cfg_attr` in another
spelling (a path, a raw identifier, inside `unsafe(...)`), which the guard does not read. The
module is a test module when the run is true with `test` and false without it. A run the guard
cannot read whole, or a predicate it cannot parse, makes no test module.

The guard reads the own file's test modules only when rustc compiles the own file under
`--cfg test` whatever else is configured: the file is a crate root (`src/lib.rs`, `src/main.rs`,
`src/bin/*.rs` or `src/bin/*/main.rs`), or a crate root reaches it through out-of-line
declarations, each at its file's top level, each read from the one file rustc could read for it,
and each kept by its whole run with `test` on. A file that only an inline module, a `#[path]`, an
undecided run or a source the lexer refuses reaches is not reached, so a shape it implements stays
refused.

The zero-file case is refused. When the run removes the declaration under `--cfg test`
(`#[cfg(test)] #[cfg(any())] mod tests;`, or `#[cfg(test)] #[cfg(feature = "slow")] mod tests;`
with the feature off), rustc reads no module file, and a stale `tests.rs` spelling the shape pins
nothing. A module that may also compile without `test` is refused as well.

An out-of-line test module is read only when rustc's choice of its file is not in doubt. The guard
lists every file rustc could read for it: `name.rs` and `name/mod.rs`, below the implementation
file's own directory and beside the file, since a crate root, a `src/bin` file, a `mod.rs` and a
file loaded through `#[path]` all read their modules beside themselves. It reads the one of them
that exists, when rustc's lexer reads it. When two exist, or none, or an attribute of the
declaration carries `path` in any spelling (`#[path = "..."]`, a raw string, spaces inside the
brackets, or `cfg_attr` under any predicate), it reads no file. A test module declared inside
another module, inline or out-of-line, is not followed.

Each limit fails closed, so a shape that only such a module spells stays refused. Measured against
rustc's own reading of a generated population of 133,267 members (whitespace and comments at every
place between an attribute's tokens, every literal prefix and hash count, doc comments, macro token
trees, raw identifiers, CRLF, a byte order mark and a shebang, crossed with the populations of the
two reviews before), of which rustc accepts 117,616, and in which rustc compiles each member once
with `--cfg test` and once without, the rule reads no module that rustc compiles out under
`--cfg test` or compiles without it, outside the two limits of section 10 (#449 and
#458). It refuses valid members for five reasons: 14,874 where an attribute chooses the
file, 12,408 where only a `#[path]` reaches the own file, 6,796 where the run names an option other
than `test`, 4,720 where the module is declared inside another module or by a macro (#433, #441),
and 2,360 where two candidate files exist: 41,158 in all. With `debug_assertions` off and two
further options set in both runs, it refuses 44,656: 14,874, 12,702, 9,376, 4,720 and 2,984.
R9. `impl Setting for` is read from comment-free source, so one inside a block comment is not
examined.
R10. Each of the eight rewrites and each of the three further arm rewrites has one row in the band
S19200-S19299 (S19216 to S19228: thirteen rows, because the two new readings have a row each and a
test may serve several). So does each of the 28 arms of the reader that a second review found
unpinned: nine of the lexer, seven of the out-of-line reading, eleven of the selection of spellings,
and the depth check of R8 (S19229 to S19256). A third review moved R8 to the union rule, which
removes the reading of a `#[path]` value: the five rows of that reading (S19239, S19240, S19242,
S19243 and S19244) lost their finds and are deleted, and six rows pin the arms of the union rule
(S19257 to S19262: the `path` word, the place beside the file, the module directory, the
one-file requirement, the `name/mod.rs` leaf and the `name.rs` leaf). A fourth review moved R8 to
the three-valued reading of the attribute run, which removes the substring test for `#[cfg(test)]`
and the two patterns of a declaration. Four rows are re-anchored on the new reading (S19227,
S19238, S19246 and S19247), and S19256's killer moves to the generated test of A13. S19241 is
deleted as equivalent, on the reasoning that a declaration spelled inside a string cannot
follow the end of an item; a fifth review measured that false for a string after a `;` among a
macro's arguments (`m!(a; "#[cfg(test)] mod tests;")`), which that reading refused. Fifteen
rows pin the arms of the new reading (S19263 to S19277: the three operators, the removal without
`test`, the whole outer run, the inner run, a partly read
inner run, a key-value option, another option, both halves of `cfg_attr`, a test module inside
another module, a span opened only by `{`, a file read only for `;`, and `r#name`). That was
fifty-six rows, S19216 to S19277 less the six deleted ids, each proved by its full id. A fifth
review moved R8 to rustc's lexer and to the walk from the crate roots, which removes the skeleton,
its patterns and the raw-string reader. Nineteen rows are re-anchored on the token reader (S19217,
S19218, S19221, S19223, S19225, S19227, S19229, S19230, S19238, S19245, S19246, S19247, S19256,
S19257, S19268, S19270, S19275, S19276 and S19277). Eight are deleted: the three rows of the
skeleton (S19219, S19220 and S19226) and the partly read inner run (S19269) lost their arms, since
a token reader cannot stop inside an attribute; three became another row's (S19231 is S19229's
byte prefix, S19232 is S19230's hash count, and S19274 is S19256's walk of the top level); and
deleting the item-end check of an outer run (S19267) is equivalent on source rustc accepts, since
every item ends at `;` or `}`. Twenty-two rows pin the new arms (S19278 to S19299): the byte order
mark, the shebang, a `#!` that opens an attribute, a comment after `#!`, inner and outer doc
comments, the Reference's whitespace, raw identifiers, C strings, raw C strings, byte characters,
spaced inner and outer attributes, an inline module's inner run, the walk's reach, the declarations
it follows, its judging under `test`, a `cfg` in another spelling, an inner run read to its end, a
key-value's value, numbers and lifetimes. Six further arms are killed by the tests but have no row,
because the band is full and each only adds refusals: finding an inline declaration, the `true`
and `false` literals, the `main.rs` and `src/bin` roots, and the own file's inner attributes on an
inline and on an out-of-line module. Deleting the walk's record of the files it has seen is
equivalent on source rustc accepts, which declares no module twice. That is seventy rows, S19216 to
S19299 less the fourteen deleted ids, each proved by its full id.

Insertions into the criteria of section 3 (the section is not edited, which keeps this file's earlier
bytes as they were; the criteria are defined below, by insertion of new A-numbers, as SPEC-038 §8
ruling (i) allows):

- A13 (out-of-line test modules) and A14 (an impl in a comment) are new tests, red first.
- A15 is the set of assertions added inside the existing tests of A11 and A12, recorded `not red`
  because the base guard already passed them; the seventy rows prove each kills its rewrite.

## 9. Amendment acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A13 | a module whose attributes keep it under `--cfg test` and remove it without, with every other option unknown and `true` and `false` their values, read from rustc's tokens (whitespace and comments between an attribute's `#`, `!` and `[`, doc comments, every literal prefix and hash count, macro token trees, raw identifiers), is the implementation's own test module, inline or out-of-line, when a crate root reaches the own file through declarations kept under `--cfg test`; an out-of-line one, beside the file, in a `mod.rs` directory, or below a `lib.rs`, is read when it is the one file rustc could read; an ambiguous choice, any `path` attribute, a module rustc compiles out, or a source rustc's lexer refuses is read as none; every generated member is read from the source rustc compiles only under `test`, or refused, judged by rustc runs whose exit status, error count and root probes are checked | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsOutOfLineTestModules` |
| A14 | an `impl Setting for` inside a block comment is not examined | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardIgnoresAnImplementationInAComment` |
| A15 | the strengthened assertions of A11 and A12 refuse each of the six lexer rewrites and the two selection rewrites | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardJudgesAPlantedTree -k TheGuardReadsRustSource` |

```acceptance
A13: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsOutOfLineTestModules
A14: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardIgnoresAnImplementationInAComment
A15: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardJudgesAPlantedTree -k TheGuardReadsRustSource
```

## 10. What this amendment does NOT do

- It does not change any production line, `SHAPE` literal or setting (#336).
- It does not make the lexer a full Rust parser: a `#[cfg(test)]` module written by a macro is
  still not counted (#441).
- It does not follow a test module declared inside another module, inline or out-of-line, so a
  shape only that module spells stays refused (#433).
- It does not read the items of a compiled test module one by one: an item that a `cfg` removes
  inside it is still read as test code (#449).
- It judges a declaration, not a source file: a file that a second declaration compiles without
  `test` (`#[cfg(not(test))] mod tests;` beside `#[cfg(test)] mod tests;`, or a `#[path]` to the
  same file) is still read through its test declaration (#458).
- It does not examine an implementation whose line does not open with `impl Setting for`: a raw
  identifier (`impl r#Setting`), the trait under a `use ... as` alias, a one-line `macro_rules!`
  body, and an implementation after an attribute or a closing brace on its line are not found, so
  their shapes are not judged (#436).

## 11. Amendment, 2026-09-30: a module written by a macro is read

Section 6 says a `#[cfg(test)]` module written by a macro is not counted, so the guard errs toward
refusing. That is wrong for one shape, and it is corrected here. A `macro_rules!` that declares a
`#[cfg(not(test))]` `#[path = ...]` module, invoked beside a `#[cfg(test)] mod tests;`, gives rustc
two declarations of one file: the macro's, which compiles it without `test`, and the test module's.
The guard's reader does not look inside a macro's token tree, so it sees only the test declaration
and reads that file. Such a module is READ, not refused, and so the shape is a miss toward reading,
not toward refusing: section 6's "errs toward refusing" does not hold for it. It is the sibling of
the shape recorded under #458, a file that a second declaration compiles without `test`, and it is
filed with that issue (#458). A run of the guard against a crate whose `lib.rs` declares the macro,
invokes it and declares the test module reads the file the macro's module also compiles.

The band continues at S19300 to S19309, because S19200 to S19299 is full, and a pin's row still
lives in the delivering SPEC's band (ADR-192). The five rows S19300 to S19304 are in
`scripts/mutation-rows.d/S19300-S19399.json`, a file whose name spans S19300 to S19399 and that holds
only ids from S19300 to S19309, so the file's name and the ids of this SPEC's rows differ by design. They
pin five arms of the reader: a shebang after a byte order mark, the walk's refusal of an undecided
declaration, a `path` spelled as a raw identifier, a block comment of three stars, and a `cfg_attr`
that lists two attributes. The five rows share one killer by design, because the five members that
kill them were added to one test,
`test_every_module_file_choice_is_read_from_rustcs_file_or_refused`: each row's proof still runs
that one test green without the mutant and red with it.

## 12. Amendment, 2026-10-01: a nested module is resolved, a macro module is refused and a rival declaration is refused

Section 10 recorded three residuals of the guard as open, and section 11 recorded a fourth. This
amendment closes them, and each is closed by an arm that a test plants and a row kills. Nothing above
is rewritten: the old bullets stay as the record of what the guard did when they were written.

- **#433, a module declared inside an inline module.** Section 10's third bullet said the guard does
  not follow a `mod tests;` declared inside an inline module, so a shape only that file spells stayed
  refused. The guard now resolves it to the file rustc reads. The file is below the inline module
  names, in the module directory of the declaring file (`src/` for a crate root, a `mod.rs` and a
  file an attribute loaded, and `src/<stem>/` for any other file, so `src/depth/a/tests.rs` for a
  declaration in `depth.rs` inside `mod a`). A `#[path = "..."]` on the declaration is read relative
  to that directory, and a `#[path]` on an enclosing inline module moves the directory in a way the
  guard does not read, so a declaration below one stays refused. Tests plant the declaration in
  `lib.rs` and in a non-root module, at one to three inline levels, with the shape and without it,
  and with the `#[path]` form. The lexer's reading of C string literals stays out of scope.
- **#441, a module written by a macro.** Section 10's first bullet and section 11's correction said
  a `#[cfg(test)]` module written by a `macro_rules!` body is not counted. The guard now FAILS
  CLOSED: a crate file whose `macro_rules!` body declares a module under a `cfg(test)` or
  `cfg_attr(test, ...)` attribute is refused, and the refusal names the file. Macro expansion was
  rejected, because the guard would then have to be a macro expander. Over the repository at the
  base the arm examined 194 crate files and found 0 hits, so it refuses no file that is read today.
- **#458, a file that a second declaration compiles without `test`.** Section 10's fourth bullet and
  section 11 said the guard judges a declaration and not a file. It now judges the file: a module
  file that any visible declaration compiles without `test` is refused, and is not read through its
  test declaration. The shapes are `#[cfg(test)] mod tests;` beside `#[cfg(not(test))] mod tests;`,
  and `#[path = "tests.rs"] mod prod;` beside `#[cfg(test)] mod tests;`, in either attribute order,
  with an inner `cfg_attr`, and with a `#[path]` that names the same file. A rival whose attributes
  the guard cannot decide counts as compiled, so it errs toward refusing. A file compiled only under
  test is still read.

Five rows pin the new arms, S19305 to S19309, in `scripts/mutation-rows.d/S19300-S19399.json`: the
inline names that join the module path, the nested `#[path]` that is read below the module
directory, the refusal of a macro module, the refusal of a rival declaration, and the rival whose
attributes are undecided. Each killer is the pinning test, and each row is proved KILLED by full id.
What stays open is not this SPEC's: the trait read by token and the aliases (#436), and an item a
`cfg` removes inside a compiled test module (#449).

## 13. Acceptance criteria of the 2026-10-01 amendment

| id | criterion | decided by |
|---|---|---|
| A16 | a `mod tests;` declared inside inline modules is resolved to the file rustc reads, or to the `#[path]` it names, from `lib.rs` and from a non-root module | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardResolvesAModuleDeclaredInsideAnInlineModule` |
| A17 | a crate file whose `macro_rules!` body declares a `cfg(test)` module is refused by name, and no file of the repository is | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardRefusesAMacroThatDeclaresATestModule` |
| A18 | a module file that any visible declaration compiles without `test` is refused, and a file compiled only under test is still read | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardRefusesAModuleFileThatAnotherDeclarationCompilesWithoutTest` |

```acceptance
A16: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardResolvesAModuleDeclaredInsideAnInlineModule
A17: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardRefusesAMacroThatDeclaresATestModule
A18: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardRefusesAModuleFileThatAnotherDeclarationCompilesWithoutTest
```

## 14. Amendments to the file manifest, 2026-10-01

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-192-every-settings-shape-is-pinned-by-its-literal.md` | repo | changed (sections 12 to 14 appended) |
| `docs/decisions/ADR-304-the-guard-resolves-refuses-and-judges-the-file.md` | repo | added |
| `docs/red-first/SPEC-192.md` | repo | changed (an addendum) |
| `scripts/tests/test_setting_shapes.py` | repo | changed (A16 to A18 and the arms) |
| `scripts/mutation-rows.d/S19300-S19399.json` | repo | changed (rows S19305 to S19309) |
| `changelog.d/guard-setting-shapes-433-441-458.md` | repo | added |
