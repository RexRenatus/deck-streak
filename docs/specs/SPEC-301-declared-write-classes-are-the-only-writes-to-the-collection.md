# SPEC-301: CHARTER constraint 4, ADR-089 and ADR-037 name declared write classes as the only writes to the collection

- **Wave:** none (an owner amendment of the charter, outside the wave plan). **Issue:** #514.
  **Context(s):** `repo` (`CHARTER.md`, `docs/` and `scripts/tests/`).
- **Decided by:** ADR-301 (this SPEC's own: the rule, its parts (a) to (g), and what it was chosen
  against).
- **Status:** built. It holds `docs/red-first/SPEC-301.md`.

## 1. The problem, measured

- **Three documents limit every write to the skip day.** At the base, this command prints 6 lines
  in 4 documents:
  `git grep -n -E 'ONLY write back|The only writes ever made|no upload path, proven' -- CHARTER.md docs/decisions`.
  CHARTER constraint 4 says "The skip day is the ONLY write back to Anki"
  (`CHARTER.md` line 34). ADR-089's guardrail (i) allows only the skip day's reschedule and its
  exact inverse (lines 55 to 57), and its guardrail (iii) runs that write only on the owner's
  explicit skip declaration (lines 60 and 61). ADR-037's condition (a) allows no upload path
  (line 14), and its condition (b) allows one scheduled sync per study day plus the owner's
  explicit triggers (lines 15 to 17). ADR-083 restates the limit, and ADR-089 is the only document
  that relaxes it, for the skip day alone.
- **No document says what a write class is.** `git grep -n -i 'write class' -- docs CHARTER.md`
  prints 0 lines at the base. Nothing names the writes that may never be made, the backup a write
  takes first, how a write earns the right to run without the owner, or what a write is scored on.
- **The owner decided to widen the limit under guard rails (#514).** The owner signed the change
  "with this never-list", named DeckStreak as the one writer, chose efficiency over any exam-date
  target, kept derived card data under data-rights, sent card text to a model only through the
  proxy, and gave the writes "Autonomous within guard rails" as their ceiling.

## 2. Requirements

R1. ADR-301 exists with `status: accepted` and states the rule word for word: "DeckStreak writes to
    the collection only through declared write classes, each with its own ADR in ADR-089's form."
R2. ADR-301 amends CHARTER constraint 4, ADR-037's conditions (a) and (b), and ADR-089's
    guardrails (i) to (iv), and says what each now reads:
    - ADR-037 (a) is superseded for each declared write class's path only. Every other path keeps
      the zero-upload proof against the recording fake sync server, and each class's ADR extends
      that proof to its exact changes and their inverse.
    - ADR-037 (b): an autonomous write class rides the study day's one scheduled sync and adds no
      scheduled sync. An approval-rung batch rides an owner trigger, as today.
    - ADR-089 (i): the only writes are each declared write class's exact changes and their exact
      inverses, and every other path records zero uploads.
    - ADR-089 (ii) binds every class: incremental sync only, and on any full or one-way sync demand
      the write aborts and writes nothing.
    - ADR-089 (iii): the owner's explicit declaration becomes the approval rung. The autonomous
      rung replaces it with promotion by trial, the guard metric's own undo and the kill switch.
    - ADR-089 (iv) generalises: record the prior state, preview the batch, and undo only cards
      whose current state still equals what the write wrote.
R3. ADR-301 states the parts (a) to (g) that #514 binds, each under its own lettered heading: (a)
    the never-list; (b) the backup and the batch; (c) the promotion ladder; (d) the single writer;
    (e) the objective; (f) card text, derived data and persona review; (g) what it was chosen
    against.
R4. The never-list holds exactly the eight entries #514 states, numbered 1 to 8, and each names
    what it protects.
R5. ADR-301's options name the chosen option and four rejected ones, each with its reason on the
    bullet's first line: advisory-only, approval-only, a second scheduled sync for writes, and
    superseding condition (a) for every path.
R6. CHARTER.md, ADR-089 and ADR-037 keep the clauses ADR-301 amends byte for byte. Each gains ONE
    dated amendment note, after those clauses and in its last section, naming ADR-301, the clauses
    it amends and the rule. Nothing earlier in the three files is edited: the delivery only appends
    to each of them.
R7. ADR-301 says that each future write class needs its own ADR in ADR-089's form, with its own
    formal-methods decision under the repository's formal check (ADR-295) and its own SPEC whose
    criteria are proven red first against the recording fake sync server. This delivery builds no
    write path.
R8. The charter change is recorded by the owner's ruling,
    `docs/rulings/OWNER-RULING-2026-10-01-deck-writes.md`. Its first line names ADR-301 and quotes
    the owner's answer, its commit is the delivery's last and holds that one file, and the owner
    signs it with a key `config/owner-allowed-signers` holds.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | ADR-301 is accepted, states the rule word for word, and names CHARTER constraint 4, ADR-089, ADR-037, its own ADR in ADR-089's form and the formal-methods decision; a copy whose rule is reworded no longer states it | `python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k the_adr_states_the_rule_and_names_what_it_amends` |
| A2 | the decision outcome holds the parts (a) to (g) once each, and each part states the terms #514 binds it to; a part with a term removed is caught | `python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k the_adr_states_each_part_a_to_g` |
| A3 | part (a)'s never-list holds exactly #514's eight entries, numbered 1 to 8, each naming what it protects in three words or more; a planted blank protection and a planted foreign entry are refused | `python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k each_never_list_entry_names_what_it_protects` |
| A4 | the options hold the chosen option and the four rejected ones once each, with their verdict, and every option bullet carries its reason on its first line | `python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k the_options_say_why_advisory_only_and_approval_only_lost` |
| A5 | CHARTER.md keeps constraint 4's two lines byte for byte and carries one dated note, after them and in its last section, naming ADR-301, constraint 4 and the rule; the same note over a charter that lost those lines is refused | `python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k the_charter_keeps_constraint_4_and_its_note_names_adr_301` |
| A6 | ADR-089 keeps guardrails (i) to (iv) byte for byte and carries one dated note, after them and in its last section, naming ADR-301, (i) to (iv) and the rule; the same note over a copy that lost them is refused | `python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k adr_089_keeps_guardrails_i_to_iv_and_its_note_names_adr_301` |
| A7 | ADR-037 keeps conditions (a) and (b) byte for byte and carries one dated note, after them and in its last section, naming ADR-301, condition (a), condition (b) and the rule; the same note over a copy that lost them is refused | `python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k adr_037_keeps_conditions_a_and_b_and_its_note_names_adr_301` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k the_adr_states_the_rule_and_names_what_it_amends
A2: python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k the_adr_states_each_part_a_to_g
A3: python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k each_never_list_entry_names_what_it_protects
A4: python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k the_options_say_why_advisory_only_and_approval_only_lost
A5: python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k the_charter_keeps_constraint_4_and_its_note_names_adr_301
A6: python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k adr_089_keeps_guardrails_i_to_iv_and_its_note_names_adr_301
A7: python3 -m unittest discover -s scripts/tests -p test_declared_write_classes.py -k adr_037_keeps_conditions_a_and_b_and_its_note_names_adr_301
```

R8 is decided by the owner's signature, not by a test (§5).

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-301-declared-write-classes-are-the-only-writes-to-the-collection.md` | `repo` | added |
| `scripts/tests/test_declared_write_classes.py` | `repo` | added: reads documents only |
| `docs/decisions/ADR-301-deckstreak-writes-to-the-collection-only-through-declared-write-classes.md` | `repo` | added |
| `CHARTER.md` | `repo` | changed: one note appended under its amendments |
| `docs/decisions/ADR-089-the-skip-day-writes-its-reschedule-back-to-anki-and-every-other-path-never-uploads.md` | `repo` | changed: one note appended |
| `docs/decisions/ADR-037-sync-once-a-study-day-plus-owner-triggers-and-never-upload.md` | `repo` | changed: one note appended |
| `docs/red-first/SPEC-301.md` | `repo` | added |
| `changelog.d/docs-charter-deck-writes-514.md` | `repo` | added |
| `docs/rulings/OWNER-RULING-2026-10-01-deck-writes.md` | `repo` | added: the delivery's last commit, alone, signed by the owner |

## 5. What this does NOT do

- It builds no write path: no code, no configuration and no test of a write. Each write class is
  its own delivery, with its own ADR in ADR-089's form and its own SPEC (#514).
- It declares no new write class. ADR-089's skip day stays the only class with an ADR, and its
  planned SPEC takes part (b)'s backup and counts through its own amendment before its write is
  built (#108).
- It edits no other document that restates the old limit. ADR-083, ADR-151, SPEC-001's gate-6
  amendment and the planned SPEC-083 and SPEC-151 are read under ADR-301, which names them (#514).
- It writes no test of the ruling's signature. The owner signs the delivery's last commit, after
  every commit a test could name, and the signature is checked against
  `config/owner-allowed-signers` when the delivery lands (#514).
- It sets no kill switch, rung, trial or guard metric in configuration: each belongs to the first
  write class that needs it (#514).
- It decides no model and no proof: it touches no interleaving and no invariant surface, and each
  write class's ADR states its own formal-methods decision (#514).

## 6. Risks

- A later edit could reword an amended clause or drop its note, and a write class could then be
  built against the earlier wording. A5 to A7 pin each kept clause and each note.
- An edit inside the earlier text of an amended file would read as a kept clause while it changed
  the text around it. The delivery's diff of each of the three files only appends:
  `cmp -n <base size> <base file> <head file>` exits 0 for each, and the pull request records it.
- A write class could read ADR-301 as permission and skip its own ADR. R7 and part (c) put every
  class at the advisory rung until its own ADR and SPEC are accepted.
- The ruling is valid only under the owner's signature. A commit made after it would leave the
  head unsigned, so the ruling's commit is the delivery's last.

## 7. The mutation rows

It adds no mutation-row band: the change is documents and a test that reads them, with no
production code to mutate (#514).

## 8. References

Issue #514; ADR-301; ADR-089; ADR-037; ADR-083; ADR-151; ADR-295; CHARTER constraints 4, 11, 13,
16 and 18; SPEC-083; SPEC-001 §14.
