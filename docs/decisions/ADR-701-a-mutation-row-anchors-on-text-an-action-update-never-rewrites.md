---
status: accepted
decision-makers: "the DeckStreak architect"
---

# A mutation row anchors on text an action update never rewrites, and a test refuses a find that holds a pin or its version comment

## Context and Problem Statement

A hand mutation row finds its anchor as text that occurs once in its target. Five rows on the
workflows held an action pin or its version comment in that text: S42802 and S42803 in
`.github/workflows/ci.yml` (the checkout and install pins of the `engine` and `release` jobs),
S59109 in `.github/workflows/xcframework.yml` (the sibling's checkout pin) and S61834 and S61835 in
`ci.yml` (the benchmark upload's version comment). An update of a pinned action rewrites the pin
and the version in its comment on every `uses:` line that names it, so each update of one of those
actions left its rows' anchors at 0 occurrences, and the census and the mutation verdict read red
on the bot's pull request until a hand commit re-anchored them, as on #1308 and #1310 (SPEC-678
section 1).

## Decision Drivers

- No update of a pinned action moves a row's anchor, so a bot's pull request reads the rows green
  without a hand commit.
- Each row keeps its id, band, killer and mutant: the mutant edits the same characters of the same
  line, and its killer observes it as before.
- No step's behaviour, key, value, pin or version comment changes to make a find hold.
- A later row that anchors on a pin is refused before it lands, by a test that names it.

## Decision Outcome, and what each decision was chosen against

### D1. The two upload rows anchor forward, on the step's own inputs

S61834 and S61835 find the upload step's `if:` line with the `with:` line and the `mode:` input
under it, text an update never rewrites, which occurs once in `ci.yml`. The replacement edits the
same `'true'` to `'false'`.

- Chosen against: keeping the version comment in the find and re-anchoring on each update, because each update of the upload action then needs a hand commit on the bot's branch, as #1310 did.

### D2. The install and checkout rows anchor on a comment line placed above the edited line

S42802, S42803 and S59109 each gain one comment line directly above the line their mutant edits,
inside the step's `with:` mapping, naming the row; the find runs from that comment through the
edited line. A comment is no part of the step: GitHub's parser and the house workflow reader drop
it, so every test of the workflows reads the same steps.

- Chosen against: a find running forward from the install step's `tool:` line to the first line that differs between the `engine` and `release` jobs, because that window spans ten lines of the next step, its checksum and its download command, so a hand update of that tool moves the row, and its text is far wider than the mutant.
- Chosen against: a step `name:` with the step's keys reordered so `with:` precedes `uses:`, because it changes how the step reads in the run's log and its key order, where a comment changes neither.
- Chosen against: no change for S59109, because no window around its edited line avoids both pins and occurs once: the sibling repeats `harness` byte for byte between them.

### D3. A test refuses a find that holds a pin or a version comment

`scripts/tests/test_mutation_rows.py` reads every committed row through the runner's reader and
asserts that no find holds `@` and forty lowercase hex digits or `# v<n>.<n>`, naming each row
that does; a second test plants a pin, a comment and both, and three clean finds, and asserts the
check names exactly the first three. Five rows of the new band pin each arm and its boundary.

- Chosen against: a refusal inside the runner's `census`, because it changes the runner, which every rows job runs, for a rule a test states as well, and SPEC-039's census holds what a row needs to be proved, not where it may anchor.
- Chosen against: a target-aware check that refuses any find overlapping a `uses:` line's ref, because it reads every target for a rule the two text forms already decide at the base, and the forms are the ones the delivery's done condition names.

## Consequences

- Good, because an update of the install, checkout or upload action now leaves every row's
  anchor in place, and the next row that anchors on a pin is refused by name.
- Good, because each mutant is unchanged: each row's mutated workflow, with the comment line
  taken out, is byte for byte the base row's.
- Bad, because three workflow comment lines exist for a row's anchor, and a hand edit of one moves
  its row, as an edit of any anchored line does.
- Bad, because a pin spelt as a tag or a short commit is not refused, since no workflow at the
  base pins that way.

## Confirmation

The first test reads red at the commit that adds it, naming the five rows, on the pull request's
`hygiene` job, and green once they are re-anchored; the census test reads each re-anchored find
once; the rows leg and `mutation-verdict` name each of the five rows and the new band's rows
KILLED by id at the head.

## What would make this wrong

- An update tool that rewrites comment lines inside a step's `with:` mapping, or a `with:` input
  other than a version.
- A workflow reader in the tree that reads comment lines as part of a step.

## More Information

SPEC-678; SPEC-039 (the rows, the census and the runner); SPEC-428, SPEC-591 and SPEC-618, whose
bands hold the five rows; #1308 and #1310.
