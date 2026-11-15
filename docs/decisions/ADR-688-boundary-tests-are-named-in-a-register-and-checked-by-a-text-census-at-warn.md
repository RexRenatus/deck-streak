---
status: "accepted"
---

# ADR-688: boundary tests are named in a register and checked by a text census that starts at warn

Decides SPEC-666. Path:
`docs/decisions/ADR-688-boundary-tests-are-named-in-a-register-and-checked-by-a-text-census-at-warn.md`.
Amends nothing. Cites SPEC-436 and ADR-450 (a census at warn, its listing, its `STALE` refusal and
D6's promotion rule).

## Context

- **A comparison against a named constant turns at the constant.** `count <= MAX_ITEMS` and
  `count < MAX_ITEMS` differ only when `count` is `MAX_ITEMS`, so only a test at the constant tells
  them apart, and only tests one step to each side pin the side the comparison turns on.
- **The mutation tools do not force that test.** cargo-mutants replaces `<` by `==` and `>`, `>`
  by `==` and `<`, `<=` by `>` and `>=` by `<`: each mutant differs from the original on a whole
  range of values, so a suite with no test at the constant kills them all. The repository's Python
  mutation runner only negates a comparison. StrykerJS replaces `<` by `<=`, which only a test at
  the constant kills, and the web run breaks on a survivor; Rust, Python, Swift and shell have no
  such run.
- **The comparisons span six languages.** A text reading of the shipped sources found comparisons
  against named constants in Rust, Python, TypeScript, Svelte and Swift, and the shell scripts hold
  constant forms of their own.
- **No rule names the test.** No SPEC or ADR states it, and no check reads it.

## Decision Drivers

- No rule in force is lowered, and no existing test or production file changes.
- A new rule starts at warn and is promoted to fail by ADR-450 D6's rule; demoting a fail rule is a
  signed ruling.
- Every check prints what it examined and fails on an empty population, which is VOID.
- Code is read as the language reads it: a comparison in a comment or a string is not one.
- Today's sites are listed with their reasons, not cured.

## Decisions

**D1. One census in Python, for six languages (SPEC-666 R1 to R3).** The census is a `unittest`
module under `scripts/tests`, which the hygiene job's Python stage runs with every other module of
that directory. ADR-450 D4 put its censuses in Rust because they read Rust alone; this one reads
Rust, Python, TypeScript, Svelte, Swift and shell, so one Python reader serves all six. Test code
is read apart by path and by Rust's test and proof attributes.

**D2. The constant forms, and what stays undecided (SPEC-666 R4, R5, R8).** Each language's own
forms name a constant: Rust `const` and `static` items and associated constants, Python capitals
module names and `Final`, TypeScript `const` bindings and imported capitals, Swift `static let` and
file-scope `let`, shell `readonly` names and names assigned once. An operand that is an expression
of constants, or a capitals path that no shipped item declares (an enum variant, a dependency's
constant), prints `UNDECIDED` and is never a site.

- **False positives:** a capitals name that is a constant by form but holds a value the code
  treats as a flag; a comparison of a constant with a value that cannot reach it.
- **False negatives:** an alias of a constant, a constant behind a function call, a comparison a
  formatter does not space or that spans lines, and a shell script without the `.sh` suffix.

**D3. The link is a register (SPEC-666 R6, R7).** `scripts/constant-boundaries.json` names, for
each comparison's key, a test below, at and above the constant, and the census resolves each name
to test code the tree defines. It proves that a named test stands at each of the three points of
each comparison, in all six languages. It cannot see whether the test reaches that comparison or
holds those values; review reads the three names against the test bodies.

**D4. One step (SPEC-666 R6).** For an integer constant `C`, the three points are `C - 1`, `C` and
`C + 1`; for a float, the adjacent representable values; for a duration, one unit of the
constant's own unit; for a size, one byte or one element. For equality against an ordered
constant the points are its two neighbours and the constant; against an unordered one (a string,
a set member) the constant and one other value, named as `at` and `below`, with `above` naming the
same other-value test.

**D5. A key names no line (SPEC-666 R7).** `<file> <operator> <constant> #<ordinal>` survives an
edit above the comparison; the ordinal moves only when the same comparison is added earlier in
the same file.

**D6. The level, the listing and the kill criteria (SPEC-666 R9 to R12).** `LEVEL` is `warn`. The
push that lands the census holds an empty listing, and its CI job reports every site over the whole
tree; the next push lists each one with its reason and that job's id. A listed key that names no
current site fails at both levels, so the listing shrinks as sites gain tests. Promotion to fail
follows ADR-450 D6: three iterations of the rule review at precision of at least 0.8 with at least
five hits, by its own delivery; if 2 of the first 5 hits are false, a demotion is proposed, and a
demotion is a signed ruling. `examined 0` is VOID.

**D7. Not in this delivery.** No boundary test for a listed site, no production change, no
mutation tool configuration change and no new operator in any mutation runner.

## What it was chosen against

### D1

- **A Rust test under a crate's `tests`.** It would read five other languages from a crate that
  ships none of them, and run only in the Rust job.

### D3

- **A hand-written mutation row per replacement** (`<` to `<=` differs only at the constant). It
  proves behaviour, but the web code is proved by StrykerJS and never by hand rows, Swift rows are
  costly, and the census would see only that a row exists.
- **A test that names the constant beside its two neighbours.** It proves only that some test text
  spells them, misses a test that spells the values as literals, and moves with the constant.
- **StrykerJS's report.** It covers TypeScript alone and is a CI artifact, not tree text.

### D6

- **Fail from the start.** It would refuse every pull request until each site has its tests; a new
  rule starts at warn.
