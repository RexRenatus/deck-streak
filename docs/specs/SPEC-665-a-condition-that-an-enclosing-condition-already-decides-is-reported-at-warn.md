# SPEC-665: a condition that an enclosing condition already decides is reported at warn

Path: `docs/specs/SPEC-665-a-condition-that-an-enclosing-condition-already-decides-is-reported-at-warn.md`.

- **Issue:** #1385.
- **Context(s):** one new `scripts/tests` census module, its planted fixtures, the rows band and the
  documents. No product source, existing test, workflow, manifest lint table, `clippy.toml`,
  `ruff.toml` or other tool configuration changes.
- **Decided by:** ADR-687 (redundant guards are a text census at warn over every shipped language).
  It cites ADR-450 D6 (the kill criteria and promotion rule of a census that starts at warn) and
  SPEC-030 (the shape of a `scripts/tests` census that reads more than one language), and amends
  neither.
- **Schematic:** none added: the census is one module that reads text, beside its siblings.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-665.md`. Read at dev `b9a08e60`.

## 1. The problem, measured

A guard that sits under another condition and can never change the outcome, because the enclosing
condition has already decided it, is dead logic. A reader takes it for a live branch, and an edit to
one copy of the condition leaves the other stale.

What the tree holds at dev `b9a08e60`, by a design reading that re-implemented this census's
matching over copies of the shipped sources (a lower bound, read before this module existed):

- 692 shipped source files were read: Rust 408 (`crates/*/src`, two build scripts and four files
  under `tools/`), Python 73 (`scripts/*.py`, `deploy/` and the non-test files under `tools/`),
  TypeScript, JavaScript and Svelte 143 (under `web/`, outside its test files), Swift 53 (under
  `ios/`, outside its test targets) and shell 15.
- 1,884 conditions sit under an enclosing condition, in 228 of those files. 523 hold a call and are
  never decided. The other 1,361 are decided by no enclosing condition in the three forms of
  section 2. No site was found.
- Some shipped files were outside that reading. The census reads them in CI like every other file,
  so this SPEC claims no exact count over the whole population: the delivery's own first CI job
  prints it, with each site found.
- Test code (856 files) is outside the rule, because a test repeats a check on purpose to pin it,
  and a test's guard decides no shipped outcome.
- The lints in force already read some forms in Rust: clippy's correctness and style groups, which
  the workspace sets to warn under the lint stage's denial of warnings, read a repeated condition in
  an `if`-`else if` chain, a comparison of an expression with itself, and two comparisons of one
  value in one boolean expression. The census reads those forms too, with the nested and
  early-exit contexts beside them. The tree runs no lint of this kind over Python, TypeScript,
  Svelte, Swift or shell; whether an existing rule should also report a form is a separate
  decision (section 5).

## 2. Requirements

R1. The census is `scripts/tests/test_redundant_guards.py`, a `unittest` module that the python
    stage collects by its name. It reads each file as text; it never runs, compiles or imports one.
R2. Its population is every shipped source file of five languages, whether or not it holds a
    condition today: Rust under each crate's `src/`, each crate's `build.rs` and Rust under
    `tools/`; Python in `scripts/*.py`, under `deploy/` and under `tools/`; TypeScript, JavaScript
    and Svelte under `web/`; Swift under `ios/`; and every `*.sh` file. A walk of the checkout skips
    the generated directories a checkout may hold (`node_modules`, `target` and their kind).
R3. Test code is outside the rule and is not read: a `test_*.py` file under `tools/`, the web test
    runner's files (`*.test.*` and `*.spec.*` scripts, `web/app/tests/` and `__tests__/`), the
    Swift test targets (`ios/*Tests/` and a package's `Tests/`), and in Rust an item under
    `#[cfg(test)]` or `#[test]`, a file under `#![cfg(test)]` and an out-of-line module a
    `#[cfg(test)] mod <name>;` declares.
R4. It prints `examined <n> file(s)` and `examined <n> condition(s) under an enclosing condition`,
    and refuses either at zero as VOID; over the repository every language of R2 is present.
R5. Same expression: an inner condition whose normalised text equals an enclosing fact, or its
    negation, is a site.
R6. Range: a comparison of a name against a literal is a site when an enclosing comparison of the
    same name against a literal fixes its result (numbers by `<`, `<=`, `>`, `>=`, `==` and `!=`;
    strings by `==` and `!=` under an enclosing `==`).
R7. Presence: an absence or presence check (`is_none`, `is_some`, `is_err`, `is_ok`, `=== undefined`,
    `!== null`, `== nil`, `is None`, `is not None` and their kind) is a site when an enclosing
    binding (`if let Some(..)`, `if let`, `guard let`) or presence check on the same value proves it.
R8. An enclosing condition is: the condition of an `if`, `elif` or `while` whose body holds the
    inner one; an earlier conjunct of the same `&&` or `and` chain; an `if` whose body exits
    (`return`, `continue`, `break`, `throw`, `raise`, `panic!`, `unreachable!`, `fatalError`),
    negated, for the rest of its block; a Swift `guard`, for the rest of its scope; the negation of
    a single condition, for its `else` body; and an enclosing Svelte `{#if}` block.
R9. A fact holds only inside one function body, and only while nothing between the two checks
    assigns the value, shadows it, borrows it mutably or hands it to a call. The same holds for
    anything anywhere in a loop body that encloses the inner check and begins after the enclosing
    condition (a `while`, `for`, `loop`, `do` or `repeat` body; a Swift `do` scope is read as one
    too, which can only drop a fact), since a later pass runs the inner check again after it. An
    enclosing Svelte `{#each}` block binds its item, index and key afresh on every pass, so it
    drops each fact on them. A nested function, closure or arrow body drops every fact.
R10. Not a site: a condition that holds a call outside a closed pure set (`is_some`, `is_none`,
    `is_ok`, `is_err`, `is_empty`, `len`, `matches!`, `isinstance`, `Array.isArray`), since its
    result may differ; an inner pattern binding, which Rust and Swift need to reach the value; and
    any assertion, which branches no outcome. A decided check on an attribute path in Python,
    TypeScript, Svelte or Swift, where a property may run code, is counted as `undecided`, never
    a site.
R11. Shell: an `if` or `elif` test in brackets (`[`, `[[` or `test`), nested or chained by `&&`, in
    the same-expression form. Any other command between two tests clears every fact, since a
    function may set any variable; a loop head (`while`, `until`, `for`) is such a command, so no
    fact enters a loop body; and a test that substitutes a command is never a fact.
R12. A site's key is `<file> <function> <form> #<ordinal>`, the ordinal counting the sites of one
    file, function and form in line order.
R13. `LEVEL` is `"warn"`, and `LISTED` holds today's sites, each key with the reason it is kept.
R14. At warn an unlisted site prints `WARN redundant-guard <key>` with its line, language, context
    and enclosing line, and passes; at fail it fails naming the same. A listed entry no site matches
    fails with `STALE redundant-guard <key>` at both levels.
R15. Moving `LEVEL` to fail is its own delivery, under ADR-687 D5.
R16. The planted fixtures sit in `scripts/tests/fixtures/redundant-guards/`, each named with a
    `.fixture` suffix so that no runner collects it; the census reads them as text, and its
    population never holds them.

## 3. Acceptance criteria of SPEC-665

| id | criterion | decided by |
|---|---|---|
| A1 | R5 to R10, R12: in the planted Rust file each planted line is found with its form (a nested repeat, a chain that repeats its first conjunct negated, a fixed range, a binding that proves presence, a repeat after an early return), and the reassigned, loop, call and test-module lines are not | `test_redundant_guards.PlantedSitesAreFound.test_rust_planted_sites_are_found` |
| A2 | R5 to R10: the same in the planted Python file, with the attribute repeat counted as one undecided | `test_redundant_guards.PlantedSitesAreFound.test_python_planted_sites_are_found` |
| A3 | R5 to R10: the same in the planted TypeScript file, where a loop after the inner check that assigns the name leaves the site found | `test_redundant_guards.PlantedSitesAreFound.test_typescript_planted_sites_are_found` |
| A4 | R8, R9: the planted Svelte file's three `{#if}` sites are found, and its closure, reassigned, loop and call lines, in the scripts and the `{#each}` block, are not | `test_redundant_guards.PlantedSitesAreFound.test_svelte_planted_sites_are_found` |
| A5 | R5 to R10: the same in the planted Swift file, with a `guard` and a `guard let` as enclosing facts, and a `while` condition that decides a repeat in its own body however the body later assigns | `test_redundant_guards.PlantedSitesAreFound.test_swift_planted_sites_are_found` |
| A6 | R11: the planted shell file's nested and chained repeats are found, and its reassigned, command-substitution and loop lines are not | `test_redundant_guards.PlantedSitesAreFound.test_shell_planted_sites_are_found` |
| A7 | R2, R3: each planted shipped path reads as its language, each planted test path and fixture reads as none, and the out-of-line and file-level Rust test modules are named | `test_redundant_guards.PlantedSitesAreFound.test_the_population_reads_each_language_and_no_test_code` |
| A8 | R12, R14: a planted unlisted site passes with its exact `WARN` line at warn and fails with its `FAIL` line at fail, a planted stale entry fails with its `STALE` line at both, an unknown level fails, and two sites of one stem take ordinals 1 and 2 | `test_redundant_guards.PlantedSitesAreFound.test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does` |
| A9 | R4: an empty file list fails naming `examined 0 file(s)`, and a reading with no condition fails naming `examined 0 condition(s) under an enclosing condition` | `test_redundant_guards.PlantedSitesAreFound.test_an_empty_population_is_refused` |
| A10 | R2, R4, R13, R14: over the repository both counts print above zero, every language is present, no file is unread, `LEVEL` is warn and no listed entry is stale | `test_redundant_guards.TheRepositoryListsTodaysRedundantGuards.test_every_shipped_source_is_read_and_each_redundant_guard_is_listed` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_rust_planted_sites_are_found
A2: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_python_planted_sites_are_found
A3: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_typescript_planted_sites_are_found
A4: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_svelte_planted_sites_are_found
A5: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_swift_planted_sites_are_found
A6: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_shell_planted_sites_are_found
A7: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_the_population_reads_each_language_and_no_test_code
A8: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does
A9: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_an_empty_population_is_refused
A10: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_every_shipped_source_is_read_and_each_redundant_guard_is_listed
```

Every planted line is marked in its fixture by a `planted: <form>` or `accepted: <reason>` comment
that the test reads, and every expected key, line and count in the level test is a literal the test
writes, never a value the census computes.

## 4. File manifest

| path | change |
|---|---|
| `scripts/tests/test_redundant_guards.py` | added: the census and its tests |
| `scripts/tests/fixtures/redundant-guards/*.fixture` | added: six planted files, one per reader |
| `scripts/mutation-rows.d/S66500-S66599.json` | added: the rows of section 9 |
| `docs/specs/SPEC-665-...md`, `docs/decisions/ADR-687-...md` | added |
| `docs/red-first/SPEC-665.md` | added |
| `changelog.d/redundant-guards.md` | added |

The module needs no entry in `scripts/tests/test_ci_workflows.py`: it imports nothing dynamically,
starts no process and reads no attribute by a computed name.

## 5. What this does NOT cover

- No site is cured, and no production file or existing test changes: a site is listed with its
  reason, and its cure is its own change.
- No lint rule is enabled and no tool is added: an existing rule that reads a form is a separate
  decision.
- The census does not read a `||` chain, an `else if` under the negation of the conditions before
  it, a match or `case` arm's guard (a Swift `case ... where` included), a Rust `let ... else`, a
  conditional expression, or a shell range or presence test (shell reads the same-expression form
  only), and it follows no value across a function call. Each count is a lower bound.

## 6. Risks

- A false site prints at warn only, and is listed with its reason or cured by its own change; ADR-687
  D5's precision rule decides whether the census may move to fail.
- A generated directory outside R2's skip list, present in a CI checkout, would be read as source
  and could print a site at warn; its path names it in the `WARN` line.

## 7. What only CI proves

The repository test reads the whole tree, so on the box only the planted tests run, by full id,
over the planted files. The repository test's red and green, and today's sites over the whole
population, are read in CI's `hygiene` job.

## 8. Formal model

None: the census is a pure reader of committed text, closed by its planted fixtures and rows. It
adds no state, actor or write path.

## 9. Mutation rows (S66500-S66599)

`scripts/mutation-rows.d/S66500-S66599.json` holds 40 `SCRIPT_MUTATIONS` rows over
`scripts/tests/test_redundant_guards.py`, one per arm: the three forms (R5 to R7), the five
enclosing contexts (R8), the assignment, store and closure stops (R9), the loop-body scan with its
five heads, its two openings, its two bounds, the `{#each}` binding and the Python and shell loop
stops (R9, R11), the call refusal (R10), the Rust test-code skip (R3), the two shell stops (R11),
six population rules (R2, R3), the three
verdict arms (R14), the ordinal (R12) and the two empty refusals (R4). Each is killed by one
planted test, named by its full id.
