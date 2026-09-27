# SPEC-044: personas are public templates filled from a private roster, and each reads only its own subject's memory

- **Wave:** W1. **Issue:** #30 (epic #2). **Context(s):** `deck-streak-agent`; the public `agent/personas/` and `agent/golden/` directories.
- **Decided by:** ADR-013 (private material never enters the tree), ADR-019 (a language reading is
  its mentor's daily reading), and ADR-044 (the roster file, the topic binding, the band fallback,
  and memory read through a subject-scoped reader).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-044.md` (ADR-016).

## 1. The problem, measured

- **The charter's persona rule has no code.** Personas are public templates and a private roster;
  names, bios, the subjects run and every weak spot stay private; a persona reads its own subject's
  memory only, never the journal (constraint 18). There is no persona in the tree, so the
  persona-core, law-professors and language-mentors packs wait on this issue with every row VOID
  (`.packs/wiring.json`).
- **What the packs fix.** persona-core's template schema (seven sections, four roster slots,
  `{{name}}`, `{{bio}}`, `{{voice}}` and `{{personality}}`), its output contract (`phx.persona.output.v1`,
  with `memory` as `<source>@<subject>` tokens), and its prompt order; law-professors' thirteen area
  templates and IRAC daily reading; language-mentors' five mentor templates with glosses, a grammar
  spotlight, pronunciation and script notes, and a culture note.
- **What does not exist yet.** The three memory sources (leeches, drill grades, lapses) arrive with
  their own features, and the live CEFR band arrives with Road to C2. Until then a persona reads no
  memory, and a language mentor's band comes from the roster.
- **Prerequisites.** SPEC-020 (configuration) and SPEC-021 (privacy's inventory). It precedes
  SPEC-043, whose gate is proved on the golden outputs this delivery adds. The owner approves the
  roster once, outside the repository (#163).

## 2. Requirements

R1. `agent/personas/` holds the public templates: every law template of the law-professors pack
    and every language template of the language-mentors pack, copied unchanged, with the four
    roster slots unfilled.
R2. A private roster file (schema `deckstreak.agent.roster.v1`) lives outside the repository at a
    path given by configuration. It binds each topic key to a template id, fills the four slots for
    each template it uses, and gives each language topic the CEFR band to use until the live band
    exists. `agent/roster.example.json` is a synthetic example with neutral placeholder values.
R3. Instantiation fills the four slots by plain-text substitution with no HTML escaping, and refuses
    an unknown slot, a slot left unfilled, a roster entry for an unknown template, and a topic bound
    to a template whose `duties` lack the requested duty. An instantiated persona is never written to
    the repository or a log.
R4. A template in the repository with a filled slot is refused by persona-core's `template-schema`
    row.
R5. A persona reads memory only through a reader built for its own subject and only from the
    sources its template declares (`leeches`, `drill-grades`, `lapses`). A read of another subject's
    memory is refused before any input or output, the journal is never a source, and the reader
    records every read it makes.
R6. The engine, never the model, writes an output's frontmatter: `schema`, `persona`, `subject`,
    `duty`, and `memory` listing exactly the reads the reader recorded (possibly none); for a
    language output also `cefr` and `lang`.
R7. Each memory source is a port wired by the delivery that builds its data (#133,
    #136, #83). With none wired, a persona reads
    nothing and declares `memory: []`, which is true.
R8. A language output's `cefr` is the live Road-to-C2 band when that port is wired, and the
    roster's band for the topic otherwise; the output names the band it used.
R9. `agent/golden/daily-reading/` holds synthetic golden outputs of the daily-reading duty, one law
    reading with its `corpus.json` and one language reading, on synthetic topics with no personal
    data. They pass every blocking row of persona-core, law-professors, language-mentors,
    study-duties and learning-science, whose rows run over the tree.
R10. The roster's path and contents, and every instantiated persona, stay out of the repository, the
    issues and the logs. `privacy.json`'s `private` globs name the roster's file name, and the public
    scrub and persona-core's `scrubber` row pass over the tree.
R11. No identifier this delivery declares in the agent context says `character` or `avatar` (the
    lexicon's lock for `persona`).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the public templates live in `agent/personas/`, and the persona packs' template rows are green over them with non-zero examined counts | persona-core `template-schema`, `duty-composition`; law-professors `law-template-schema`, `law-area-registry`, `law-template-area`, `law-template-methods`; language-mentors `mentor-templates`, `rules-table`; `test_the_persona_template_rows_are_green_over_agent_personas` |
| A2 | a roster file outside the repository fills the four slots, and no `{{` token remains in the instantiated persona | `a_roster_outside_the_repository_fills_the_four_slots` |
| A3 | a copy of a template with a filled slot, planted in a temporary tree, is refused by persona-core's `template-schema` | persona-core `template-schema`; `test_a_template_with_a_filled_slot_is_refused` |
| A4 | an output's declared `memory` equals the reads the reader recorded, including the empty case | `the_declared_memory_equals_the_reads_made` |
| A5 | a read of another subject's memory is refused before any input or output, and a journal source cannot be named | `a_read_of_another_subjects_memory_is_refused` |
| A6 | the golden law and language readings make the persona packs' output rows examine them, and every blocking output row is green | persona-core, law-professors and language-mentors output rows; `test_the_persona_output_rows_examine_the_golden_readings` |
| A7 | instantiation refuses an unknown slot, an unfilled slot, an unknown template, and a duty the template lacks | `instantiation_refuses_an_incomplete_roster` |
| A8 | a language output's band is the live band when that port is wired and the roster's otherwise | `the_cefr_band_comes_from_the_live_band_before_the_roster` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_persona_rows.py -k test_the_persona_template_rows_are_green_over_agent_personas
A2: cargo test -p deck-streak-agent --test persona -- --exact a_roster_outside_the_repository_fills_the_four_slots
A3: python3 -m unittest discover -s scripts/tests -p test_persona_rows.py -k test_a_template_with_a_filled_slot_is_refused
A4: cargo test -p deck-streak-agent --test memory -- --exact the_declared_memory_equals_the_reads_made
A5: cargo test -p deck-streak-agent --test memory -- --exact a_read_of_another_subjects_memory_is_refused
A6: python3 -m unittest discover -s scripts/tests -p test_persona_rows.py -k test_the_persona_output_rows_examine_the_golden_readings
A7: cargo test -p deck-streak-agent --test persona -- --exact instantiation_refuses_an_incomplete_roster
A8: cargo test -p deck-streak-agent --test persona -- --exact the_cefr_band_comes_from_the_live_band_before_the_roster
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `agent/personas/*.persona.md` | agent (public) | added: the law-professors and language-mentors templates, copied unchanged |
| `agent/roster.example.json` | agent (public) | added: a synthetic roster with neutral values |
| `agent/golden/daily-reading/law/` | agent (public) | added: one synthetic law reading and its `corpus.json` |
| `agent/golden/daily-reading/language/` | agent (public) | added: one synthetic language reading |
| `crates/agent/src/persona.rs` | `deck-streak-agent` | added: templates and instantiation |
| `crates/agent/src/roster.rs` | `deck-streak-agent` | added: the private roster and its topic binding |
| `crates/agent/src/memory.rs` | `deck-streak-agent` | added: the subject-scoped reader and the source ports |
| `crates/agent/src/output.rs` | `deck-streak-agent` | added: the engine-written frontmatter |
| `crates/agent/src/lib.rs` | `deck-streak-agent` | changed: the modules above |
| `crates/agent/Cargo.toml` | `deck-streak-agent` | changed: the workspace dependencies it uses |
| `crates/agent/tests/persona.rs` | `deck-streak-agent` | added |
| `crates/agent/tests/memory.rs` | `deck-streak-agent` | added |
| `scripts/tests/test_persona_rows.py` | repo | added |
| `.packs/wiring.json` | repo | changed: persona-core, law-professors and language-mentors become `enforced` |
| `privacy.json` | repo | changed: the roster's file name under `private` |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-044-persona-engine-and-private-roster.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-044-private-roster-topic-binding-and-scoped-memory.md` | docs | added |
| `docs/red-first/SPEC-044.md` | docs | added |

## 5. What this does NOT do

- It drafts no persona name, bio or voice; the architect's draft and the owner's approval stay in
  private configuration (#163).
- It reads no leech, drill grade or lapse: those sources are wired by their own features
  (#133, #136, #83).
- It reads no live CEFR band; the Road to C2 wires it (#85).
- It ships no test-preparation coach template and runs no practice set (#52).
- It runs no duty but the daily reading's goldens; the other duties' personas speak in their own
  deliveries (#48, #51).

## 6. Risks

- **A real name or a subject list reaches the repository in an example or a golden.** Detected by
  the public scrub, persona-core's `scrubber` row and review; the examples use neutral placeholders
  and synthetic topics.
- **A golden output drifts from a pack's format after a re-vendor.** Detected by A6, which runs
  every output row over the goldens.
- **A memory source is wired later without the subject scope.** Prevented by the reader's type
  (built for one subject) and detected by A5 in each wiring delivery.
- **The roster's band lags the owner's real level** until the live band exists. Visible in each
  language output's `cefr`, and corrected in the private roster.
