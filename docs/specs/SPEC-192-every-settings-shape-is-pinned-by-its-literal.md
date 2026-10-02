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

Section 10 recorded five residuals of the guard as open (#441, #433, #449, #458 and #436), and
section 11 corrected how one shape of #458 errs. This amendment closes three of them, #433, #441
and #458, and each is closed by arms that a test plants and a row kills. #449 and #436 stay open.
Nothing above is rewritten: the old bullets stay as the record of what the guard did when they were
written.

- **#433, a module declared inside an inline module.** Section 10's third bullet said the guard does
  not follow a `mod tests;` declared inside an inline module, so a shape only that file spells
  stayed refused. The guard now resolves it to the file rustc reads. The file is below the inline
  module names, in the module directory of the declaring file (`src/` for a crate root, a `mod.rs`
  and a file an attribute loaded, and `src/<stem>/` for any other file, so `src/depth/a/tests.rs`
  for a declaration in `depth.rs` inside `mod a`). A `#[path = "..."]` on the declaration is read
  relative to that directory, and a `#[path]` on an enclosing inline module moves the directory in a
  way the guard does not read, so a declaration below one stays refused. The inner attributes that
  open the file are the module's own, so a file that opens with `#![cfg(test)]` is a test module
  even when its declaration carries no attribute. Tests plant the declaration in `lib.rs` and in a
  non-root module, at one to three inline levels, with the shape and without it, with the `#[path]`
  form, and in a file that opens with `#![cfg(test)]`. The lexer's reading of C string literals
  stays out of scope.
- **#441, a module written by a macro.** Section 10's second bullet and section 11's correction said
  a `#[cfg(test)]` module written by a `macro_rules!` body is not counted. The guard now FAILS
  CLOSED: a crate file whose `macro_rules!` body declares a module under a `cfg(test)` or
  `cfg_attr(test, ...)` attribute is refused, and the refusal names the file. The attribute is read
  in each place that reaches the module once the macro is expanded: before the `mod` keyword, back
  to the previous item's end; before a `$( ... )` repetition that holds the module; and as an inner
  attribute that opens the module's braces. Macro expansion was rejected, because the guard would
  then have to be a macro expander. Over the repository at this delivery's head the arm examined 194
  crate files and refused none, so it refuses no file that is read today.
- **#458, a file that a second declaration compiles without `test`.** Section 10's fifth bullet and
  section 11 said the guard judges a declaration and not a file. It now judges the file: a module
  file that any visible declaration compiles without `test` is refused, and is not read through its
  test declaration. The shapes are `#[cfg(test)] mod tests;` beside `#[cfg(not(test))] mod tests;`,
  and `#[path = "tests.rs"] mod prod;` beside `#[cfg(test)] mod tests;`, in either attribute order,
  with an inner `cfg_attr`, with a `#[path]` that names the same file, and in a `src/bin` crate
  root. A declaration is judged by every attribute that reaches the file: the inner attributes that
  open the file count, so a file that opens with `#![cfg(test)]` is compiled only under test; and a
  declaration whose one attribute naming a path is `#[cfg_attr(P, path = "...")]` reaches the file
  it names only under P, and its default file only without P. A declaration whose attributes the
  guard cannot decide counts as compiled. One whose file the guard cannot name is taken to name
  every file: a path literal with an escape or a raw string, a declaration below an inline module
  whose attributes name `path`, and a declaration whose attributes the guard cannot read back to the
  previous item's end (`unsafe mod`, which rustc rejects wherever it compiles it). So every doubt
  errs toward refusing. A file compiled only under test is still read.

Ten rows pin the new arms in `scripts/mutation-rows.d/S19300-S19399.json`. S19305 to S19309 pin the
inline names that join the module path, the nested `#[path]` that is read below the module
directory, the refusal of a macro module, the refusal of a rival declaration, and the rival whose
attributes are undecided. S19310 to S19314 pin the inner attributes that open a reached file, the
declaration whose file the guard cannot name, the `cfg_attr` path that reaches its file only under
its predicate, the inner attribute of a macro-written module, and the attribute before a macro
repetition. Each killer is the pinning test, and each row is proved KILLED by full id. What stays
open is not this SPEC's: the trait read by token and the aliases (#436), and an item a `cfg`
removes inside a compiled test module (#449). Section 15 lists what this amendment does not cover.

## 13. Acceptance criteria of the 2026-10-01 amendment

| id | criterion | decided by |
|---|---|---|
| A16 | a `mod tests;` declared inside inline modules is resolved to the file rustc reads, or to the `#[path]` it names, from `lib.rs` and from a non-root module, and the inner attributes that open that file are the module's own | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardResolvesAModuleDeclaredInsideAnInlineModule` |
| A17 | a crate file whose `macro_rules!` body declares a `cfg(test)` module, by an attribute before the `mod`, before a `$( ... )` repetition or inside its braces, is refused by name, and no file of the repository is | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardRefusesAMacroThatDeclaresATestModule` (the planted trees) and `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k EverySettingShapeIsPinnedByItsLiteral` (the repository) |
| A18 | a module file that any visible declaration compiles without `test`, or that a declaration whose file the guard cannot name may compile, is refused, and a file compiled only under test, by its own inner attributes or a `cfg_attr` path, is still read | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardRefusesAModuleFileThatAnotherDeclarationCompilesWithoutTest` |

```acceptance
A16: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardResolvesAModuleDeclaredInsideAnInlineModule
A17: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardRefusesAMacroThatDeclaresATestModule
A17: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k EverySettingShapeIsPinnedByItsLiteral
A18: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardRefusesAModuleFileThatAnotherDeclarationCompilesWithoutTest
```

## 14. Amendments to the file manifest, 2026-10-01

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-192-every-settings-shape-is-pinned-by-its-literal.md` | repo | changed (sections 12 to 15 appended) |
| `docs/decisions/ADR-304-the-guard-resolves-refuses-and-judges-the-file.md` | repo | added |
| `docs/red-first/SPEC-192.md` | repo | changed (two addenda) |
| `scripts/tests/test_setting_shapes.py` | repo | changed (A16 to A18 and the arms) |
| `scripts/mutation-rows.d/S19300-S19399.json` | repo | changed (rows S19305 to S19314) |
| `changelog.d/guard-setting-shapes-433-441-458.md` | repo | added |

## 15. What this does NOT cover, the 2026-10-01 amendment

- It does not see every spelling rustc reads. Four still fail open, and none occurs in the tree
  today: an attribute spelled `#[$a]` inside a `macro_rules!` body, or a macro that passes its
  tokens to another invocation; a `#[path]` rival declared inside a block such as a function body; a
  `#[path]` rival declared by a macro; and `mod prod { include!("tests.rs"); }` (#535).
- It still refuses some trees that rustc reads plainly, each by failing closed: a `macro_rules!`
  body that declares a module under `cfg(not(test))` or `cfg(any(test, ...))`, a `mod tests;` inside
  an inline module of a non-`mod.rs` file beside a decoy declaration, and a declaration below an
  inline module whose file a `cfg_attr` path chooses (#536).
- `modules()` still returns a name that its only caller drops (#536).
- It does not read a trait implementation by token or through an alias, and it does not read the
  items of a compiled test module one by one (#436, #449).

## 16. Amendments, 2026-10-02: an implementation read by token, a test module read by its compiled items, four spellings refused and three over-refusals read, with their acceptance criteria

Sections 12 and 15 left #436 and #449 open, and #535 and #536 recorded what the 2026-10-01 arms
still miss and still over-refuse. This amendment closes the four. Nothing above is rewritten: the
old bullets stay as the record of what the guard did when they were written. ADR-310 records the
four decisions it rests on.

- **#436, an implementation is read by token.** The guard finds an implementation from rustc's
  tokens, not from a pattern anchored at a line start. An `impl` whose trait path, read up to the
  `for` after its generic parameters, ends in a name bound to `Setting` is a `Setting`
  implementation. Names compare after an `r#` prefix is removed. The names are `Setting` and every
  name a `use ... as` binds to one of them, in any file under `crates/*/src`, followed through a
  chain (`use self::A as B;`) until no name is added. So an attribute or a second item on the
  implementation's line, a raw identifier and an alias are read as rustc reads them, and a string
  literal that holds the words is not an implementation. The `SHAPE` literal is the first
  `const SHAPE` inside the implementation's own braces, read from comment-free text. An `impl`
  inside a `macro_rules!` body or inside a macro invocation's token tree, whose trait path ends in
  such a name or in a metavariable (`$tr`), or holds a repetition (`impl $($p)::+ for $t`,
  `impl $($p)* for Wide`), is refused by its file whether or not its shape is pinned, because the
  guard does not expand a macro. So is an `impl` whose head the tree leaves open with no trait, a
  metavariable or such a name in it (`make!(impl)`), one with no `for` whose head holds a
  repetition or a metavariable beside another word (`impl Setting $f Wide`), and a `for` whose
  head holds a metavariable and no `impl`, since the `impl` is passed in (`$k Setting for Wide`).
  A loop's `for` and a bound's `for<'a>` make none.
- **#449, a pin counts only in an item rustc compiles under test.** In each test module the guard
  reads (an inline test module of the implementation's own file, an out-of-line test module's file,
  and a file of the crate's `tests/` that cargo and rustc compile), it walks the items. An item ends at its first `;` at depth
  0, or at a `{ }` group at depth 0 that the next token does not continue: an open delimiter, a
  punctuation other than `#` and `$`, and the words `else`, `as` and `where` continue it. An item is
  read when one evaluator, `rustc_keeps`, reads its outer attributes as true under `test`.
  `rustc_keeps` evaluates `cfg` over `test`, `any`, `all`, `not`, `true` and `false` exactly. Any
  other option (a `feature = ...`, a target key) is unknown, every `cfg_attr` is unknown because it
  may make a `cfg`, and so is an attribute it cannot read. An item it does not read as true counts
  as not compiled, so the guard never reads a pin from an item rustc strips, and at worst refuses a
  pin rustc compiles: such a false refusal is disclosed. A kept inline module is walked again with
  its own inner attributes. Any other kept item that holds an attribute `rustc_keeps` does not read
  as true (on a statement, a field or an associated item) is not read at all. A file under `tests/`
  is read as cargo builds it. A crate root is `tests/<name>.rs` or `tests/<dir>/main.rs`, the
  targets cargo discovers, and its inner attributes are read by `rustc_keeps` too. Any other file
  under `tests/` is a module: it is read only when a file the walk reads declares it with `mod`
  (below inline modules too) under attributes `rustc_keeps` reads as true under `test`, from the
  module directory that declaration gives it, so a file no such declaration reaches is never read.
  The walk reads a file once for each module directory it is given, so a module cycle ends it, and a
  root rustc cannot lex is listed but declares nothing the walk follows. A test module's own
  attributes are still judged as R8 judges them.
- **#535, four spellings are refused by name.** A module in a `macro_rules!` body or in a macro
  invocation's token tree is refused by its file when an attribute that reaches it holds a
  metavariable (`#[$a]`, `$(#[$m])*`), or names `path`, or (#441) names `cfg` or `cfg_attr` with
  `test` unless `rustc_keeps` proves it is no test-only module. A macro invocation is read as a body
  is, so a macro that passes its tokens through is refused for what it passes. An out-of-line module
  that a macro declares, and that `rustc_keeps` does not prove removed without `test`, may compile a
  test module's file without `test`; the guard cannot name that file, so, by #458's rule, it may
  name every file and refuses every out-of-line test module file of its crate. An out-of-line module
  declared inside a block (a function body, a `const` block, an `impl`) that `rustc_keeps` does not
  prove removed without `test` is refused by its file. An `include!` in a crate file is refused by
  its file, because it compiles another file's text where it stands. So is a crate file with a
  `use` declaration that may import `include` under any name or in any group
  (`use core::include as pull;`, `use std::{include as x};`) or whose path a macro passes in
  (`use core::$m as pull;`), and one where a macro name is passed in by a macro (`$m!(...)`,
  `$($m)*!(...)`), because each may name `include`.
- **#536, three over-refusals are read.** A module a macro declares is no longer refused under #441
  when `rustc_keeps` proves it is no test-only module: one removed under `test` (`cfg(not(test))`)
  or one compiled without it (`cfg(any(test, true))`). A reached file's module directory is known
  from how a crate root reaches it: a crate root, a `mod.rs` and a file a `#[path]` loaded read
  beside themselves, and any other file reads below its stem, so a decoy file in the other directory
  no longer refuses the module; a file reached both ways keeps the hedge of section 12. A
  declaration below an inline module whose one attribute naming a path is
  `cfg_attr(P, path = "...")` is read from the file P chooses under `test`: the named file when P
  holds, the default file when it fails. `modules()` returns each declaration's attributes and
  index, without the name its only caller dropped.

What these amendments leave out of reach, each by design and each failing closed except where a
limit says otherwise:

- A block-scoped out-of-line module is refused, not resolved: the guard does not model rustc's
  block-scope path rules, which a text reader could only approximate and could get wrong toward
  reading. `test_a_block_scoped_declaration_is_refused_by_its_file` pins it (#535).
- `mod prod { include!("tests.rs"); }` is refused, not followed: the guard does not read the file an
  `include!` names. `test_an_include_is_refused_by_its_file` pins it (#535).
- A `use` that imports `include` refuses its file even when the name is never invoked, and so does
  a `use` that binds another item to the name `include` (`use std::fmt::Write as include;`): the
  guard reads the word, not what it names. Each is a false refusal, never a false pass (#535).
- An out-of-line module that a `macro_rules!` body declares, unless `rustc_keeps` proves it removed
  without `test`, leaves every out-of-line test module file of its crate unread by #458's rule,
  even when no invocation expands the macro, so a shape pinned only in such a file is refused: a
  false refusal, never a false pass (#535).
- A `cfg(any(test, P))` module a macro declares, whose P `rustc_keeps` cannot decide, stays refused,
  because it is a test-only module when P fails (#536).
- A name bound to `Setting` in one module and to another trait in another is read as `Setting` in
  both, so an implementation of the other trait with no pinned shape is refused (#436).
- A macro invocation that passes a lone `impl` (`make!(impl)`) refuses its file even when the macro
  makes an implementation of another trait, because the guard does not expand it: a false refusal,
  never a false pass (#436).
- An item under an unknown predicate or a `cfg_attr`, or a kept item that holds one, is not read
  even when rustc compiles it (#449).
- A module a `tests/` file declares through a `#[path]` attribute is not read, and a `tests/` file
  that is both a crate root and another file's module has two module directories, so a module it
  declares is followed from neither when its file is in both. A pin in such a file is not read:
  each is a false refusal, never a false pass (#449).
- A manifest's `[[test]]` tables and `autotests` key are not read: `tests/` is read by cargo's
  default discovery, and no manifest of the repository holds either. A target a `[[test]]` table
  adds elsewhere is not read, which can only refuse; a crate that set `autotests = false` would
  have its `tests/` roots read although cargo builds none of them, which fails open (#449).
- A procedural macro's output is not read; no crate of the workspace is a procedural macro crate
  (#436).

The rows that pin the new arms are in `scripts/mutation-rows.d/S19300-S19399.json`, from S19315,
and in `scripts/mutation-rows.d/S19400-S19499.json`, each killed by the test that pins its arm and
proved KILLED by full id.

The lines this delivery adds to the guard hold 126 decision arms. 121 are pinned by rows: 14 by
the rows S19315 to S19328, and 107 by the 108 rows S19329 to S19436 (one arm has two). The other 5
are equivalent: the generated population reads the same with and without each mutant, and a probe
rustc compiles shows that no input it accepts reads differently. One arm was removed instead: a
pass in `start_of` over a `$(` refused an attribute inside a macro repetition
(`$(#[cfg(any())] mod $n;)*`) that rustc compiles with the pin under test, and without it the
guard reads that shape and refuses the one with the attribute before the `$(`, as it did. The band
files now hold 100 rows (S19300 to S19399) and 37 rows (S19400 to S19436). S19324's killer is
`test_a_pin_counts_only_in_a_compiled_item_of_every_file_kind`, a MUTATION COVERAGE test in A20's
class: its 12 members put a pin in each kind of file that holds one (the implementation's own test
module, a `tests/` file and an out-of-line test module's file) under four attributes, judged
against rustc. In `scripts/mutation-rows.d/S19200-S19299.json`, S19252 and S19225's killer point
at the token reader (#436), and S19254 finds the roots of the walk of `tests/` that replaced the
line it found (#449). The arms left dev's rows S19300, S19301 and S19303 surviving: the R8
members that killed them at the base read the same with and without each mutant. R8 gains 75
members over the base: 11 that carry a pin through each declaration chain and behind each opening
the lexer skips, and 64 whose inner run is `#![doc = "a"]` then `#![cfg(p)]` (examined 6384 R8
members judged against rustc, and 1486 members rustc refuses judged for Width), and the three are
KILLED again. The module runs 76 tests. One over-refusal is kept on purpose and disclosed by count
(#536): A22 asserts 1 macro module shape, `cfg(any(test, feature = "..."))`, refused, and the
#441 arm refuses 0 of the repository's 217 crate files.

Files of these amendments:

- `docs/specs/SPEC-192-every-settings-shape-is-pinned-by-its-literal.md`: changed (this section
  appended).
- `docs/decisions/ADR-310-one-cfg-evaluator-and-every-unexpanded-spelling-refused-by-name.md`:
  added.
- `docs/red-first/SPEC-192.md`: changed (two addenda).
- `scripts/tests/test_setting_shapes.py`: changed (A19 to A22, the arms and their killers).
- `scripts/mutation-rows.d/S19300-S19399.json`: changed (rows from S19315).
- `scripts/mutation-rows.d/S19400-S19499.json`: added (rows S19400 to S19436).
- `scripts/mutation-rows.d/S19200-S19299.json`: changed (S19252 and S19225's killer point at the
  token reader, and S19254 finds the roots of the walk of `tests/`).
- `changelog.d/guard-setting-shapes-436-449-535-536.md`: added.
- `docs/decisions/ADR-192-a-settings-shape-is-pinned-by-its-literal-and-the-rows-live-in-one-band.md`:
  unchanged.
- `docs/decisions/ADR-304-the-guard-resolves-refuses-and-judges-the-file.md`: unchanged.
- `crates/analytics/tests/rollup_metrics.rs`: unchanged.
- `crates/bot/tests/commands.rs`: unchanged.
- `crates/bot/tests/transport.rs`: unchanged.
- `crates/daemon/tests/sync_request.rs`: unchanged.
- `crates/ingest/tests/settings.rs`: unchanged.
- `crates/kernel/tests/courses_config.rs`: unchanged.
- `crates/kernel/tests/settings.rs`: unchanged.
- `crates/readings/tests/topics.rs`: unchanged.
- `crates/vault/tests/confinement.rs`: unchanged.
- `changelog.d/test-setting-shapes-192.md`: unchanged.
- `changelog.d/guard-setting-shapes-433-441-458.md`: unchanged.

| id | criterion | decided by |
|---|---|---|
| A19 | every spelling of an implementation (a raw identifier, an alias, an alias chain and a shadowing alias, an attribute or a second item on its line, a string that holds the words, and a macro-made one, its trait path a repetition or its `impl` passed in among them), with its shape pinned and unpinned, is read as rustc reads it or refused by its file, against expectations written without the guard | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsAnImplementationByToken` |
| A20 | over a generated population of item-level `cfg` shapes inside compiled test modules, with rustc's own evaluation as the oracle, the guard reads no pin from an item rustc strips, and reads every pin rustc compiles where `rustc_keeps` decides; a file under `tests/` counts only where cargo and rustc compile it | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsOnlyTheItemsRustcCompilesUnderTest` |
| A21 | each of #535's spellings, an `include!` a `use` imports under another name among them, is refused by its file, each limit fails closed, the same trees without the plant read 0 refusals, and no file of the repository is refused | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardRefusesASpellingItDoesNotExpand` (the planted trees) and `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k EverySettingShapeIsPinnedByItsLiteral` (the repository) |
| A22 | each of #536's over-refusals is read as rustc reads it, each #441 and #535 arm still refuses beside it, and `modules()` returns no name | `python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsATreeItOverRefused` |

```acceptance
A19: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsAnImplementationByToken
A20: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsOnlyTheItemsRustcCompilesUnderTest
A21: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardRefusesASpellingItDoesNotExpand
A21: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k EverySettingShapeIsPinnedByItsLiteral
A22: python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py -k TheGuardReadsATreeItOverRefused
```
