---
status: "accepted"
---

# ADR-687: redundant guards are a text census at warn over every shipped language

Decides SPEC-665. Path:
`docs/decisions/ADR-687-redundant-guards-are-a-text-census-at-warn-over-every-shipped-language.md`.
Amends nothing. Cites ADR-450 D6 (the kill criteria and promotion rule of a census that starts at
warn) and SPEC-030 (a `scripts/tests` census that reads more than one language). Read at dev
`b9a08e60`.

## Context

A condition under another condition that the enclosing one already decides can never change the
outcome. It reads as a live branch, and an edit to one copy leaves the other stale. SPEC-665
section 1 measures the tree: 1,884 conditions under an enclosing condition in the shipped files a
design reading covered, none decided in the three forms, and some shipped files left to CI. Clippy's
groups in force read some forms in Rust; nothing reads them in the other languages.

## Decision Drivers

- Default-deny: a language or a form is left out only on a measurement that supports it.
- Every count is a lower bound, and the census says so: a fact it cannot follow is dropped, never
  guessed.
- A new rule starts at warn with today's sites listed, and lowers no rule in force.
- No dependency and no tool configuration change.

## Decisions

**D1. A text census in `scripts/tests`.** One `unittest` module reads every shipped file of the
five languages as text: Python through its own `ast`, the brace languages and Svelte by one token
reader, and shell line by line (SPEC-665 R1 to R3). It adds no tool, so it runs where the python
stage runs.

**D2. Three forms, decided only over unchanged values.** The same expression or its negation, a
comparison whose result the enclosing range fixes, and a presence check the enclosing binding
proves (R5 to R7), under the contexts of R8. A fact holds inside one function body only while
nothing between the two checks, nor anything in a loop body that encloses the inner check and
begins after the outer one, assigns, shadows, mutably borrows or passes the value to a call
(R9); a condition with a call is never decided, and an attribute path is counted undecided (R10).
- **False positives:** a value changed through an alias or a global between the checks, which text
  cannot see; each is listed with its reason or cured by its own change.
- **False negatives:** the forms SPEC-665 section 5 names, and every fact the stops drop.

**D3. It starts at warn.** `LEVEL` is warn and `LISTED` holds today's sites with their reasons
(R13). An unlisted site prints its `WARN` line and passes; a stale entry fails at both levels (R14).

**D4. Test code is outside the rule.** A test repeats a check on purpose to pin it, and its guard
decides no shipped outcome (R3). The planted fixtures carry a `.fixture` suffix and sit outside the
population (R16).

**D5. Kill criteria and promotion.** ADR-450 D6's rule, applied to this census: `LEVEL` moves from
warn to fail after three iterations of the rule review at precision of at least 0.8 with at least
five hits, by its own delivery (R15). Live precision is fixes ÷ (fixes + new listing entries) over
the last 30 merged pull requests; below 0.8 with at least five hits, propose a demotion; for a
census with no hit today, if two or more of its first five hits are false, propose a demotion.
Demotion is a weakening, and goes to the maintainer as a signed ruling. `examined 0` is VOID, which
blocks; it is never a pass.

**D6. Tests and rows, red first.** One test per acceptance criterion. The red commit holds the
tests, the fixtures and stubs that keep every input but the missing behaviour (no form decides, the
verdict passes everything, no path reads as a language and no refusal refuses), so each red is an
assertion on its criterion. The planted reds are read on the box by full id; the repository test's
red is read in CI. Rows S66501 to S66540 sit in `scripts/mutation-rows.d/S66500-S66599.json`,
each killed by one planted test.

**D7. Not in this delivery.** No lint rule enabled, no tool added, no site cured, and no
production file, existing test, manifest lint table or tool configuration changed.

## What it was chosen against

- Enabling an existing rule in each language's linter: rejected here because it adds a tool or a
  configuration change, and the rules differ in what they read; it stays a separate decision.
- Clippy alone: rejected because it covers Rust only.
- A census that follows a value across calls: rejected because a call may change it, so a reading
  that follows it would report sites that are not decided.

## Consequences

- Each condition an enclosing condition decides in shipped code is printed at warn or listed with
  its reason.
- A new site in a later change prints a `WARN` line in CI's `hygiene` job.

## Confirmation

SPEC-665's acceptance criteria, each decided by one test of `scripts/tests/test_redundant_guards.py`,
and the rows of D6.

## What would make this wrong

- A false-site rate that D5's precision rule reads below 0.8.
- A form outside D2 that turns out common in the tree, which the reading did not measure.
