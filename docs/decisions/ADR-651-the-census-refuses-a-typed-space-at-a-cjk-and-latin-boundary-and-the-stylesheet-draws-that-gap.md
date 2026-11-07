---
status: "accepted"
decision-makers: "the DeckStreak architect"
---

# The census refuses a space typed at a CJK and Latin boundary, and the stylesheet draws that gap

## Context and Problem Statement

The web app's stylesheet sets `text-autospace: normal` for Japanese and Chinese
(`web/app/src/lib/styles/cjk.css:58-69`), which draws the narrow space CLREQ 6.3.3 and JLREQ 3.2
ask for between CJK and Latin letters or numerals, "so no hand-typed spaces are needed". SPEC-571
R10 held that rule for the values SPEC-571 wrote, and its census judges no spacing. At the dev
SPEC-629 was cut from, by the rule of SPEC-629 R2 to R4:

- `web/app/messages/ja.json` holds 59 typed-space runs at a CJK and Latin boundary, in 30 values;
  `zh-Hans.json` 72, in 40 values; `zh-Hant.json` 72, in 40 values; `ko.json` 0.
- A planted control of three values counts 2 under the same rule.
- So one boundary renders as a full typed space in some messages and as the stylesheet's narrow
  gap in others, and nothing refuses a new typed space.

This ADR decides SPEC-629: the census refuses such a space, and the three catalogs drop every one.

## Decision Drivers

- The rule is already the house's, in the stylesheet: a check must hold it where a value is
  written, by the census CI's Python stage already runs.
- No wording changes: only the typed spaces at those boundaries go, so the fluent review of each
  locale (#1053) still reads the same words.
- No new package or tool, and no change to the stylesheet, English, Spanish, French or Korean.

## D1. The census refuses the typed space

`scripts/locale_census.py` judges each value of `ja`, `zh-Hans` and `zh-Hant` (SPEC-629 R1) and
refuses one holding a run of whitespace between a CJK letter (R2: kana, the iteration marks and
the Han ranges, never punctuation or Hangul) and an ASCII letter, a digit, a percent sign or a
placeholder brace (R3), counted as runs with a character on each side (R4). The finding is
`spaced <locale> <key> <n>`, after the value's `not-nfc` and `placeholder` findings, and exits 1
as every finding does (R5).

## D1. Chosen against

- Leaving the typed spaces to the fluent review (#1053): the review reads meaning, and a typed
  space is shape; the next message would bring one back unseen.
- Judging Korean: Korean spaces its words, and a space beside Hangul is its word space, not a gap
  the stylesheet draws.
- A web build lint in place of the census: it adds a tool and a stage, where the census already
  reads every catalog in CI's Python stage.

## D1. Consequences

- A later change that types such a space is refused by name, with the key and the count.

## D2. The three catalogs drop every run now

Every run at a boundary in the three catalogs is deleted, both neighbours kept, by a program that
asserts the line count, the keys and their order, every placeholder multiset and NFC unchanged
(SPEC-629 R6). English, Spanish, French and Korean keep their bytes.

## D2. Chosen against

- Replacing each typed space with a thin space: the stylesheet already draws the gap, so a thin
  space would add a second gap beside it, and it is a typed space the census would have to admit.
- Stripping only the values a later change touches: the mixed rendering stays on every other
  screen, and the census could not be green over the tree.

## D2. Consequences

- The same boundary renders the same narrow gap on every Japanese and Chinese screen.
- A browser without `text-autospace` renders those boundaries with no gap, which JLREQ and CLREQ
  accept.

## D3. Tests red first; rows pin the literals; formal not applicable by surface

The refusal's tests are committed first, over the census that judges no spacing, and read red by
an assertion on the missing finding; the census over the repository reads red again once the
refusal lands, until the catalogs are stripped. Rows in `scripts/mutation-rows.d/S62900-S62999.json`
pin the census's new literals, each killed by a named test, and the Python mutation lane runs the
census against its test module. Formal: not applicable by surface (no protocol, no interleaving,
no state machine).

## D3. Chosen against

- One commit for the refusal and the strip: the census's red over the tree would never be seen.

## D3. Consequences

- The record holds each red and each green at the commit it was measured at.

## Consequences

- No space is typed at a CJK and Latin boundary in the Japanese and Chinese web catalogs, and a
  new one is refused where it is made.

## What would make this wrong

- A value whose typed space at a boundary is wanted, such as a code token: the census refuses it,
  and a later SPEC must teach the exception.
- A CJK letter outside R2's ranges, such as CJK Extension J of Unicode 17: a space beside it is
  not refused until the ranges grow.

## More Information

- SPEC-629 holds this decision's requirements, acceptance criteria and file manifest.
- SPEC-571 and ADR-590 hold the census this decision extends; neither is changed.
