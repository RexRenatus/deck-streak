# SPEC-678: no mutation row anchors on an action pin or its version comment

- **Context(s):** the hand-proved mutation rows (`scripts/mutation-rows.d`), their tests
  (`scripts/tests/test_mutation_rows.py`), two workflows (`.github/workflows/ci.yml` and
  `.github/workflows/xcframework.yml`, one comment line per re-anchored install or checkout step)
  and `docs`.
- **Decided by:** ADR-701
- **Status:** delivered with its tests and `docs/red-first/SPEC-678.md`.

## 1. The problem, measured

At the base of this change, five rows of 3690, in 199 band files and the header, hold a find with
a GitHub Actions pin (`@` and forty hex digits) or the version comment beside one (`# v<n>.<n>`):

| row | band file | target | what its find spans | what its mutant does |
|---|---|---|---|---|
| S42802 | `scripts/mutation-rows.d/S42800-S42899.json` | `.github/workflows/ci.yml` | the `engine` job's checkout and install steps, both pins and both version comments | drops the exact version of the engine set's `cargo-nextest` |
| S42803 | `scripts/mutation-rows.d/S42800-S42899.json` | `.github/workflows/ci.yml` | the `release` job's checkout and install steps, both pins and both version comments | drops the exact version of the release job's `cargo-nextest` |
| S59109 | `scripts/mutation-rows.d/S59100-S59199.json` | `.github/workflows/xcframework.yml` | the `release-and-app` job's env and its checkout, the pin and its version comment | the sibling's copied checkout persists its credentials |
| S61834 | `scripts/mutation-rows.d/S61800-S61899.json` | `.github/workflows/ci.yml` | the `bench-rust` upload step's version comment and its `if:` line | the upload runs when the build job says not to upload |
| S61835 | `scripts/mutation-rows.d/S61800-S61899.json` | `.github/workflows/ci.yml` | the `bench-web` upload step's version comment and its `if:` line | the same, for `bench-web` |

An update of a pinned action rewrites its pin and the version in its comment on each `uses:` line
that names it, and no other text of the step. So each such update left the rows on that action at
0 occurrences of their anchor. The install action's update, #1308, rewrote 23 `uses:` lines in six
workflows and left S42802 and S42803 at 0; the benchmark upload action's update, #1310, rewrote the
two `uses:` lines of `ci.yml` and left S61834 and S61835 at 0. On each bot commit the `hygiene`
census (jobs 128717670535 and 128717680454) and `mutation-verdict` (jobs 128717671605 and
128717681168) read red, and each pull request took a hand re-anchor commit before it merged. An
update of the checkout action would leave S42802, S42803 and S59109 at 0 together.

Which text of each step an update never rewrites, measured on the base by the runner's find rule
(a find occurs once in its target):

- S61834 and S61835: the upload step's own `if:` line with the `with:` line and the `mode:` input
  under it occurs once in `ci.yml`, so each re-anchors there with no workflow change.
- S42802 and S42803: the install step holds only its `uses:` line, `with:` and the `tool:` line,
  identical in the two jobs. The smallest window around the `tool:` line that holds no pin and
  occurs once runs ten lines into the next step, across its checksum and its download command.
- S59109: no window around the `persist-credentials` line holds no pin and occurs once. Between
  the checkout's pin and the next step's pin, the sibling repeats `harness` byte for byte, as its
  own test requires.

So each of those three steps gains one comment line directly above the line its mutant edits, and
the row's find runs from that comment through the edited line. A comment line is no part of the
step: the workflow reader in `scripts/tests/test_ci_workflows.py` (`read_workflow`) drops it, as
GitHub's parser does.

## 2. Requirements

R1. No committed row's find holds an action pin, `@` followed by forty lowercase hex digits, or a
    version comment, `# v` followed by a number, a dot and a number; a test reads every committed
    row through the runner's reader and names each row that does.

R2. S42802, S42803, S59109, S61834 and S61835 keep their ids, bands, targets, killers and
    descriptions. Only each row's find and replacement change, in lockstep, and each mutant edits
    the same characters of the same workflow line as at the base.

R3. Each re-anchored find occurs exactly once in its target, and every other row whose target is
    `.github/workflows/ci.yml`, `.github/workflows/xcframework.yml` or
    `scripts/tests/test_mutation_rows.py` still finds its anchor once.

R4. The workflows change by three comment lines and nothing else: one directly above the `tool:`
    line of the `engine` job's install step, one above the `release` job's, and one above the
    `persist-credentials` line of the `release-and-app` job's checkout. No step, key, value, pin or
    version comment changes.

R5. The check names a planted pin alone, a planted version comment alone and both together, and
    names no clean find: a tool version after `@`, a seven-digit ref, or a comment that is not a
    version.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | no committed row's find holds an action pin or its version comment (R1, R2) | `scripts/tests/test_mutation_rows.py` `test_no_committed_find_holds_an_action_pin_or_its_version_comment` |
| A2 | the check names each planted pin and version comment form and no clean find (R1, R5) | `scripts/tests/test_mutation_rows.py` `test_the_pin_check_names_each_planted_form_and_no_clean_find` |
| A3 | every committed row, the five re-anchored ones included, finds its anchor once and names one test (R3) | `scripts/tests/test_mutation_rows.py` `test_every_committed_row_is_anchored_once_and_names_one_test` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_no_committed_find_holds_an_action_pin_or_its_version_comment
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_pin_check_names_each_planted_form_and_no_clean_find
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_every_committed_row_is_anchored_once_and_names_one_test
```

The first test reads every committed row through the runner's own reader and asserts that the
list of rows whose find holds a pin or a version comment is empty; at the red commit it names the
five rows above. The second runs the same check over six planted rows and asserts it names exactly
the pin row, the comment row and the row holding both. The census test is the base's own.

## 4. Read from CI by row id

Each re-anchored row's kill is read from the pull request's CI by job name, never on a local run:
the rows leg (`mutation-rows`) and `mutation-verdict` at the head name S42802, S42803, S59109,
S61834 and S61835 as KILLED by their own killers, and the five rows of `S67800-S67899` as KILLED
by the second test above. R2's same-characters claim is checked by the delivery's own measurement:
each row's mutant at the head, with the comment line taken out, is byte for byte the base row's
mutant. R4 is held by the diff, which changes the two workflows by three added lines.

## 5. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-678-no-mutation-row-anchors-on-an-action-pin-or-its-version-comment.md` | docs | added |
| `docs/decisions/ADR-701-a-mutation-row-anchors-on-text-an-action-update-never-rewrites.md` | docs | added |
| `docs/red-first/SPEC-678.md` | docs | added |
| `changelog.d/rows-anchor-off-action-pins.md` | docs | added |
| `scripts/tests/test_mutation_rows.py` | tests | changed: the pin check, its two tests and its planted rows |
| `scripts/mutation-rows.d/S42800-S42899.json` | rows | changed: S42802 and S42803's find and replacement |
| `scripts/mutation-rows.d/S59100-S59199.json` | rows | changed: S59109's find and replacement |
| `scripts/mutation-rows.d/S61800-S61899.json` | rows | changed: S61834 and S61835's find and replacement |
| `scripts/mutation-rows.d/S67800-S67899.json` | rows | added: five rows on the check |
| `.github/workflows/ci.yml` | ci | changed: two comment lines |
| `.github/workflows/xcframework.yml` | ci | changed: one comment line |

## 6. What this does NOT do

- A row on another ecosystem's dependency line keeps the hand re-anchor an update of that line
  needs: S59213's mutant edits the catalog version itself, so no anchor can leave it out (#1308).
- A pin spelt as a tag or a short commit is not refused: no workflow at the base pins an action
  that way, and the check names only the two forms the base holds (#1310).
- No row is added for the install, checkout or upload steps beyond the five re-anchored; their
  behaviour is unchanged and each keeps its one row (#1308).

## 7. Risks

- **A hand edit of a comment line moves its row.** The census names the row by id, as for any
  anchored line, and the comment says which row reads it.
- **An update rewrites a form the check does not name.** A tag or short ref in a find would read
  0 after its update; the census still names that row, and the check's patterns are the place to
  add the form.
- **The two upload rows now span the `mode:` input.** A hand change of that input re-anchors them,
  as any edit of an anchored line does.

## 8. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani. The change is a test, its planted rows, five re-anchored rows,
three workflow comment lines and documents; no protocol, state machine, interleaving, engine path
or crate source gains or changes.
