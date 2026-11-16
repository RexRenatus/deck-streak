# SPEC-701: the ruling count index

- **Context(s):** `docs` (the documentation index `docs/README.md`, the agent instructions
  `AGENTS.md`, the signed rulings folder `docs/rulings/` and the decision records) and its census
  `scripts/tests/test_ruling_index.py`.
- **Decided by:** this SPEC for the two count lines and their census; ADR-719 and ADR-720 for the
  two records this change adds.
- **Status:** built. It holds `docs/red-first/SPEC-701.md`. **Mutation band:** `S70100-S70199`.

## 1. The problem, measured

- **No index states how many signed rulings there are.** `docs/rulings/` holds 28 files, each an
  `OWNER-RULING-*.md` signed ruling, and nothing else. `docs/README.md` links the decisions folder
  but not the rulings folder, and the list in `AGENTS.md` that says where decisions live names no
  rulings folder. No test reads the folder, so a ruling can land without any index moving.
- **Two decisions are not recorded.** No record says that Anki's engine leaves DeckStreak, or which
  decisions built on it end or change at which step, and no record states the iPhone and iPad
  app's position on Anki's AGPL code in the App Store. ADR-151, which made the package the
  engine's own export, was never accepted and rests on the engine.

## 2. Requirements

- **R1.** `docs/README.md` links `rulings/` once and `AGENTS.md` links `docs/rulings/` once, and
  each of those lines states the number of `OWNER-RULING-*.md` files in `docs/rulings/`, written
  as `<n> signed rulings`.
- **R2.** The census is `scripts/tests/test_ruling_index.py`. It counts the `OWNER-RULING-*.md`
  files in `docs/rulings/` by name and fails when there are none; for each index it reads every
  count stated beside a link to the rulings folder and asserts that those counts are exactly the
  one held count.
- **R3.** ADR-719 records the engine's staged removal and ADR-720 the iPhone and iPad app's
  position, both `proposed`. ADR-151's status line reads `superseded by ADR-719`, and no other
  record's status line changes.
- **R4.** Nothing else changes: no production code, workflow, script or signed ruling.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | R1, R2: each index states, beside its one link to the rulings folder, the count of signed rulings the folder holds | `scripts/tests/test_ruling_index.py` `test_each_index_states_the_count_of_signed_rulings` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ruling_index.py -k test_each_index_states_the_count_of_signed_rulings
```

The test runs one subtest per index. At the red commit neither index states a count, so each
subtest fails its equality assertion with an empty list against the held count; the commit that
adds the two count lines turns both green with the test unchanged.

## 4. Read from CI by row id

The row S70100 in `scripts/mutation-rows.d/S70100-S70199.json` drops the word `signed` from the
documentation index's count line, so that line states no count. The pull request's
`mutation-rows` and `mutation-verdict` jobs read it as KILLED by the census test, by row id, never
on a local run.

## 5. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-701-the-ruling-count-index.md` | docs | added |
| `scripts/tests/test_ruling_index.py` | tests | added: the census |
| `docs/decisions/ADR-719-ankis-engine-leaves-deckstreak-in-stages-and-this-record-supersedes-or-amends-each-decision-built-on-it.md` | docs | added |
| `docs/decisions/ADR-720-the-iphone-and-ipad-app-ships-no-anki-code-and-no-app-store-build-carries-the-engine.md` | docs | added |
| `docs/decisions/ADR-151-the-package-is-the-engines-own-export-of-a-throwaway-collection-with-a-frozen-note-type-and-the-decisions-time.md` | docs | changed: its status line |
| `docs/README.md` | docs | changed: one count line |
| `AGENTS.md` | docs | changed: one count line |
| `docs/red-first/SPEC-701.md` | docs | added |
| `changelog.d/ruling-count-index-701.md` | docs | added |
| `scripts/mutation-rows.d/S70100-S70199.json` | rows | added: one row on the documentation index's count line |

## 6. What this does NOT do

- No other record's status line changes. The pull request of each later step sets the status line
  of each record ADR-719 supersedes at that step.
- Neither record is accepted here; each is accepted later by the owner's signature or by a new
  signed ruling.
- The census reads the rulings folder's file names and the two index files only, and no ruling's
  text.

## 7. Risks

- **A new ruling moves the count.** The pull request that adds a signed ruling moves both count
  lines with it; until it does, the census reads red, which is its purpose.
- **An index line is reworded.** A count written in another form, or a second link to the folder,
  makes the census read that index red, so a rewording keeps the `<n> signed rulings` form beside
  the one link.

## 8. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani. The change is two decision records, one status line, two
documentation lines and a test that reads committed text; no protocol, state machine,
interleaving, engine path or crate source gains or changes.
