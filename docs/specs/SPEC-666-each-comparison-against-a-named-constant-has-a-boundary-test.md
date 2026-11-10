# SPEC-666: each comparison against a named constant has a boundary test

Path: `docs/specs/SPEC-666-each-comparison-against-a-named-constant-has-a-boundary-test.md`.

- **Issue:** #1384. This SPEC closes it.
- **Context(s):** a new census module under `scripts/tests`, its planted trees, its register, the
  rows band and the documents. No product source, existing test, workflow, manifest lint table or
  mutation tool configuration changes.
- **Decided by:** ADR-688 (boundary tests are named in a register and checked by a text census
  that starts at warn). It cites SPEC-436 and ADR-450 (the shape of a census at warn, its listing
  and its promotion rule) and amends neither.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-666.md`.

## 1. The problem, measured

A comparison of a value with a named constant changes its answer at the constant. Only a test that
holds the compared value at the constant and one step to each side tells `<` from `<=`, or `>`
from `>=`. No check names that test today:

- **The mutation tools do not force it.** cargo-mutants replaces `<` by `==` and by `>`, `>` by
  `==` and by `<`, `<=` by `>` and `>=` by `<`, and never `<` by `<=` or `>` by `>=`; the
  repository's Python mutation runner negates each comparison (`<` to `>=`). A suite that never
  holds the value at the constant kills each of those mutants. StrykerJS replaces `<` by `<=`, so
  only the web code's mutation run can notice a missing test at the constant.
- **No SPEC or ADR states the rule.** The words on boundary tests in the tree sit in records and
  prose, and no check reads them.
- **The comparisons, by a reading of the design round.** A text reading of the shipped sources it
  opened found, as a lower bound, 286 comparisons against a named constant: Rust 183, Python 75,
  TypeScript 25, Svelte 1, Swift 2 and shell 0. It found none whose tests a register names, because
  no register exists. A few files that reading did not open stay in the census's population; this
  SPEC claims no exact count over the whole tree. The CI run of the push that lands the census
  measures every site, and the next push lists each one with that run's job id.

## 2. Requirements

The census (ADR-688 D1, D5):

R1. The census is `scripts/tests/test_constant_boundaries.py`, a `unittest` module the hygiene
    job's Python stage runs with every other `scripts/tests` module. It reads text and never
    imports, compiles or runs what it reads. Its class `RepositoryCensus` reads the whole tree and
    runs in CI; its class `PlantedTrees` reads the planted trees only.
R2. Its population is every tracked shipped source file in six languages: Rust (`.rs`), Python
    (`.py`), TypeScript and JavaScript (`.ts`, `.js`, `.mjs`, `.cjs`), Svelte (`.svelte`), Swift
    (`.swift`) and shell (`.sh`), whether or not a file holds a comparison today. It prints
    `examined N shipped source file(s)` and the count for each language.
R3. Test code is read apart, never as a site: a Rust file under a `tests`, `benches` or `examples`
    directory, an item under `#[test]`, `#[cfg(test)]` or `#[cfg(kani)]`, and an out-of-line module
    such an attribute declares; a Python file under a `tests` directory or named `test_*.py`; a web
    file the test runner's pattern names (`*.test.*`, `*.spec.*`), a `*.test.support.ts` module and
    the files under `web/app/tests` and `web/app/bench`; a Swift file under a directory whose name
    ends in `Tests`.
    An item under `#[cfg(kani)]` is compiled only when the Kani verifier sets that flag, so no
    shipped build holds it, and a `*.test.support.ts` module is imported only by `*.test.ts` files.

The reading (ADR-688 D2):

R4. A comparison is one of `<`, `<=`, `>`, `>=`, `==` and `!=` (and `===`, `!==` read as `==`,
    `!=`) in code: Python by its syntax tree; Rust, TypeScript, Svelte and Swift by the formatters'
    spaced operator, with comments and the contents of strings blanked, and a Svelte file's script
    blocks and markup expressions read; shell by the operators of `[`, `[[`, `test` and `((`.
R5. An operand is a named constant by its language's forms: a Rust `const` or `static` item or
    associated constant that a shipped file declares, by name or path, and a primitive's associated
    constant (`u32::MAX`); a Python capitals name assigned at module level or annotated `Final`,
    a capitals name imported by `from`, and a capitals attribute of a name; a TypeScript or Svelte
    capitals name that a shipped `const` declares or the file imports, alone or as a member; a
    Swift `Type.name` that a `static let` declares, and a bare name that a `static let` or a
    file-scope `let` of the same file declares; a shell name set by `readonly` or `declare -r`, or
    assigned exactly once at the start of a line in its file.
R6. The register is `scripts/constant-boundaries.json`. Its `boundary_tests` maps a comparison's
    key to a test `below`, `at` and `above` the constant, each `<path>::<name>`: a Rust `fn`, a
    Python `def`, a Swift `func`, or a web `test(` or `it(` title, in test code the tree defines. A
    comparison with all three points resolved is clean; any other is a site, whose reason is
    `no boundary test` when the register names it not at all and `no test <points>` naming each
    missing point otherwise.
R7. A key is `<file> <operator> <constant> #<ordinal>`, the ordinal counting the same operator and
    constant in the file in reading order; a key names no line.
R8. A comparison against a bare literal, a comparison of two constants and a comparison inside test
    code are counted apart and are outside the rule. An operand the census cannot decide (an
    expression of constants, a Rust capitals path that no shipped `const` or `static` declares, a
    Python capitals name with no binding in its file, a shell capitals name with no assignment in
    its file) prints `UNDECIDED constant-boundary <key>` and is never a site.

Levels and listings (ADR-688 D6):

R9. The verdict prints `WARN constant-boundary <key>: <reason>` for each site the register's
    `listed` table does not name, which passes at `warn` and fails at `fail`, and prints
    `STALE constant-boundary <key>` for each `listed` key that names no current site and each
    `boundary_tests` key that names no comparison, which fails at both levels.
R10. An empty shipped population fails naming `examined 0 shipped source file(s)`: VOID, never a
    pass.
R11. The planted trees sit under `scripts/tests/fixtures/constant-boundaries/`, each file with a
    `.fixture` suffix so no runner collects it, read as text by its path without the suffix. The
    planted tree holds, in each of the six languages, a comparison with no boundary test, one whose
    tests hold the value below, at and above the constant, one whose test holds the constant alone,
    and one against a bare literal; the void tree holds test code only.
R12. `LEVEL` is `warn`. The register's `listed` table is empty in the push that lands the census
    and lists each site that push's CI job reports, with its reason, in the next. Moving `LEVEL` to
    `fail` is its own delivery, under ADR-450 D6's promotion rule.

## 3. Acceptance criteria of SPEC-666

| id | criterion | decided by |
|---|---|---|
| A1 | R2, R3: the planted tree's shipped files are exactly one per language, and its Rust, Python, TypeScript and Swift test code is counted apart | `test_constant_boundaries.PlantedTrees.test_each_file_is_classed_by_language_and_test_code_is_read_apart` |
| A2 | R4, R5, R7, R9: the comparison with no boundary test prints its exact `WARN` line in each of the six languages, and the level `warn` fails nothing | `test_constant_boundaries.PlantedTrees.test_a_comparison_with_no_boundary_test_is_a_site_in_each_language` |
| A3 | R6: each comparison whose three tests resolve prints no line, and the tree holds exactly its eighteen keys | `test_constant_boundaries.PlantedTrees.test_a_test_below_at_and_above_the_constant_makes_a_comparison_clean` |
| A4 | R6: each comparison whose test holds the constant alone prints `no test below, above` in each language | `test_constant_boundaries.PlantedTrees.test_a_test_at_the_constant_alone_leaves_a_site` |
| A5 | R6: a register point naming a test the tree does not define leaves that point missing | `test_constant_boundaries.PlantedTrees.test_a_test_the_tree_does_not_define_leaves_its_point_missing` |
| A6 | R4, R8: one bare-literal comparison is counted in each language, the three undecided keys print `UNDECIDED`, and twelve sites print | `test_constant_boundaries.PlantedTrees.test_a_bare_literal_test_code_and_an_undecided_operand_are_never_sites` |
| A7 | R9: at `fail` each site fails; a listed site is silent; a listed key that names no site fails at both levels | `test_constant_boundaries.PlantedTrees.test_the_level_decides_whether_a_site_fails_and_a_stale_entry_always_does` |
| A8 | R10: a tree of test code only fails naming `examined 0 shipped source file(s)` | `test_constant_boundaries.PlantedTrees.test_an_empty_population_is_void` |
| A9 | R1, R2, R12: over the repository, the register is present, each language's count is above zero, `LEVEL` is `warn`, and no entry is stale | `test_constant_boundaries.RepositoryCensus.test_the_repository_reads_every_language_and_holds_no_stale_entry` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_each_file_is_classed_by_language_and_test_code_is_read_apart
A2: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_a_comparison_with_no_boundary_test_is_a_site_in_each_language
A3: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_a_test_below_at_and_above_the_constant_makes_a_comparison_clean
A4: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_a_test_at_the_constant_alone_leaves_a_site
A5: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_a_test_the_tree_does_not_define_leaves_its_point_missing
A6: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_a_bare_literal_test_code_and_an_undecided_operand_are_never_sites
A7: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_the_level_decides_whether_a_site_fails_and_a_stale_entry_always_does
A8: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_an_empty_population_is_void
A9: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_the_repository_reads_every_language_and_holds_no_stale_entry
```

Every planted comparison is text a fixture spells, and every expected key, line and count is a
literal the test writes, never a value the census computes. The repository test reads the whole
tree, so it runs in CI; the eight planted tests read the planted trees only.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_constant_boundaries.py` | the census | added: R1 to R12 |
| `scripts/tests/fixtures/constant-boundaries/planted/` | the planted tree | added: eleven `.fixture` files (R11) |
| `scripts/tests/fixtures/constant-boundaries/void/` | the void tree | added: two `.fixture` files (R10, R11) |
| `scripts/constant-boundaries.json` | the register | added: R6, R12 |
| `scripts/mutation-rows.d/S66600-S66699.json` | the rows | added: section 9 |
| `docs/specs/SPEC-666-each-comparison-against-a-named-constant-has-a-boundary-test.md` | the spec | added |
| `docs/decisions/ADR-688-boundary-tests-are-named-in-a-register-and-checked-by-a-text-census-at-warn.md` | the decision | added |
| `docs/red-first/SPEC-666.md` | the record | added |
| `changelog.d/constant-boundaries-NEW.md` | the fragment | added |

## 5. What this does NOT cover

- It writes no boundary test for a listed site and changes no production file; each site's own
  test is later work.
- It does not check which values a named test holds: the register's three names are the claim,
  and review reads them.
- It does not follow a constant through an alias, a function's return value or an expression of
  constants; those operands print `UNDECIDED`.
- It does not read a comparison that a formatter would not space, a comparison split across lines
  in a brace language, or a shell script without the `.sh` suffix.
- It changes no mutation tool's configuration or operator set.

## 6. Risks

- **A site the design reading missed.** The census counts every shipped file in CI; the first
  run's listing is the record, never the design round's count.
- **A tree-scanning check reads the new files.** The planted files end in `.fixture`, so no
  pattern that names a language suffix collects them.
- **A sibling change adds or moves a comparison.** Any pull request that lands first can move a
  listed key; the census reports it as a new `WARN` line and a `STALE` entry at the merge.

## 7. What only CI proves

- The repository test reads the whole tree, so its red and green are read in CI's hygiene job by
  name.

## 8. Formal model

No state, actor or write path is added: the census is a pure reader of committed text, and its
classes are closed by the planted trees, whose expected keys are spelt literally, and by section
9's rows. No model checker runs.

## 9. Mutation rows (S66600-S66699)

Rows S66601 to S66619, in `scripts/mutation-rows.d/S66600-S66699.json`, each a
`SCRIPT_MUTATIONS` row on `scripts/tests/test_constant_boundaries.py`. Each find is taken from the
green commit's text and occurs exactly once in the file; each killer is one planted test by its
full id.

| rows | what each mutant removes | requirement |
|---|---|---|
| S66601-S66603 | a test-code path rule: Rust `tests`, the web test pattern, Swift `Tests` targets | R3 |
| S66604-S66605 | `<=` as a comparison; a line comment blanked | R4 |
| S66606 | an undeclared Rust capitals path read as undecided | R8 |
| S66607-S66611 | a constant form: a shipped TypeScript `const`, a Swift `static let`, a Python module constant, an imported Python capital, a shell name assigned once | R5 |
| S66612 | a Rust file's test items read apart | R3 |
| S66613-S66615 | each of the three points required; a Rust test resolved by name; the uncovered reason | R6 |
| S66616-S66618 | a listed site silent; `fail` failing a site; a stale listed entry | R9 |
| S66619 | the empty population refused | R10 |
