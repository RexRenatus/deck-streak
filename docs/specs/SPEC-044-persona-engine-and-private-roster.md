# SPEC-044: personas are public templates filled from a private roster, and each reads only its own subject's memory

- **Wave:** W1. **Issue:** #30 (epic #2). **Context(s):** `deck-streak-agent`; the public `agent/personas/` and `agent/golden/` directories.
- **Decided by:** ADR-013 (private material never enters the tree), ADR-019 (a language reading is
  its mentor's daily reading), ADR-069 (every pack is judged by the box run, never in public CI), and
  ADR-044 (the roster file, the topic binding, the band fallback, and memory read through a
  subject-scoped reader).
- **Status:** judged: delivered with its tests, its hand-proved rows and `docs/red-first/SPEC-044.md`
  (ADR-016). The delivery made R1 to R3, R5 and R7 to R10 exact where the code decided them (§7).

## 1. The problem, measured

- **The charter's persona rule has no code.** Personas are public templates and a private roster;
  names, bios, the subjects run and every weak spot stay private; a persona reads its own subject's
  memory only, never the journal (constraint 18). There is no persona in the tree, so the
  persona-core, law-professors and language-mentors packs wait on this issue with every row VOID
  (the box-run packs' wiring, ADR-069).
- **What the packs fix.** persona-core's template schema (seven sections, four roster slots,
  `{{name}}`, `{{bio}}`, `{{voice}}` and `{{personality}}`), its output contract (`phx.persona.output.v1`,
  with `memory` as `<source>@<subject>` tokens), and its prompt order; law-professors' thirteen area
  templates and IRAC daily reading; language-mentors' five mentor templates with glosses, a grammar
  spotlight, pronunciation and script notes, and a culture note.
- **Where a pack row is judged.** Every pack verdict is the box run's (ADR-069): the public tree
  holds no probe, so no public test can run a pack's row. Each criterion over the persona packs
  therefore has a public test of DeckStreak's own behaviour (section 3), and the box run judges the
  rows themselves over the committed tree (section 3a).
- **What does not exist yet.** The three memory sources (leeches, drill grades, lapses) arrive with
  their own features, and the live CEFR band arrives with Road to C2. Until then a persona reads no
  memory, and a language mentor's band comes from the roster.
- **Prerequisites.** SPEC-020 (configuration) and SPEC-021 (privacy's inventory). It precedes
  SPEC-043, whose gate is proved on the golden outputs this delivery adds. The owner approves the
  roster once, outside the repository (#163).

## 2. Requirements

R1. `agent/personas/` holds the public templates: every law template of the law-professors pack
    and every language template of the language-mentors pack, copied unchanged, with the four
    roster slots unfilled. The engine loads every one of them.
R2. A private roster file (schema `deckstreak.agent.roster.v1`) lives outside the repository at a
    path given by configuration. It binds each topic key to a template id, fills the four slots for
    each template it uses, and gives each language topic the CEFR band to use until the live band
    exists. `agent/roster.example.json` is a synthetic example with neutral placeholder values.
R3. Instantiation fills the four slots by plain-text substitution with no HTML escaping, and refuses
    an unknown slot, a slot left unfilled, a roster entry for an unknown template, and a topic bound
    to a template whose `duties` lack the requested duty. An instantiated persona is never written to
    the repository or a log.
R4. A template with a filled slot is refused: the engine loads no template whose four slots are not
    each in their sections, and in the tree persona-core's `template-schema` row refuses one, in the
    box run.
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
    data. Each opens with the frontmatter the engine writes for its persona, duty, band and reads.
    They pass every blocking row of persona-core, law-professors, language-mentors, study-duties and
    learning-science, whose rows the box run runs over the tree.
R10. The roster's path and contents, and every instantiated persona, stay out of the repository, the
    issues and the logs. `privacy.json`'s `private` globs name the roster's file name, and the public
    scrub and persona-core's `scrubber` row pass over the tree.
R11. No identifier this delivery declares in the agent context says `character` or `avatar` (the
    lexicon's lock for `persona`).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the public templates live in `agent/personas/`, and the engine loads every one of them with its four roster slots unfilled | `every_public_template_loads_with_its_roster_slots_unfilled` |
| A2 | a roster file outside the repository fills the four slots, and no `{{` token remains in the instantiated persona | `a_roster_outside_the_repository_fills_the_four_slots` |
| A3 | a copy of a template with a filled slot is refused by the engine | `a_template_with_a_filled_slot_is_refused` |
| A4 | an output's declared `memory` equals the reads the reader recorded, including the empty case | `the_declared_memory_equals_the_reads_made` |
| A5 | a read of another subject's memory is refused before any input or output, and a journal source cannot be named | `a_read_of_another_subjects_memory_is_refused` |
| A6 | each golden reading opens with the frontmatter the engine writes for its persona, duty, band and reads | `the_golden_readings_open_with_the_frontmatter_the_engine_writes` |
| A7 | instantiation refuses an unknown slot, an unfilled slot, an unknown template, and a duty the template lacks | `instantiation_refuses_an_incomplete_roster` |
| A8 | a language output's band is the live band when that port is wired and the roster's otherwise | `the_cefr_band_comes_from_the_live_band_before_the_roster` |

```acceptance
A1: cargo test -p deck-streak-agent --test persona -- --exact every_public_template_loads_with_its_roster_slots_unfilled
A2: cargo test -p deck-streak-agent --test persona -- --exact a_roster_outside_the_repository_fills_the_four_slots
A3: cargo test -p deck-streak-agent --test persona -- --exact a_template_with_a_filled_slot_is_refused
A4: cargo test -p deck-streak-agent --test memory -- --exact the_declared_memory_equals_the_reads_made
A5: cargo test -p deck-streak-agent --test memory -- --exact a_read_of_another_subjects_memory_is_refused
A6: cargo test -p deck-streak-agent --test persona -- --exact the_golden_readings_open_with_the_frontmatter_the_engine_writes
A7: cargo test -p deck-streak-agent --test persona -- --exact instantiation_refuses_an_incomplete_roster
A8: cargo test -p deck-streak-agent --test persona -- --exact the_cefr_band_comes_from_the_live_band_before_the_roster
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. When this delivery merges, the maintainer's private
wiring makes persona-core, law-professors and language-mentors `enforced`, so a blocking row of
theirs that is red, VOID or in error fails the run.

| id | criterion | decided by |
|---|---|---|
| B1 | every template row of the three persona packs passes over `agent/personas/`, examining every template, so a template in the tree with a filled slot fails the run | persona-core `template-schema`, `duty-composition`; law-professors `law-template-schema`, `law-area-registry`, `law-template-area`, `law-template-methods`; language-mentors `mentor-templates`, `rules-table` |
| B2 | the golden readings make the persona packs' output rows examine them, and no blocking row of persona-core, law-professors, language-mentors, study-duties or learning-science is red over them | persona-core, law-professors and language-mentors output rows; study-duties; learning-science |

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
| `privacy.json` | repo | changed: the roster's file name under `private` |
| `Cargo.lock` | workspace | changed |
| `scripts/mutation-rows.d/S04400-S04499.json` | repo | added: the hand-proved rows of the subject filter, the journal exclusion and the band's source |
| `docs/schematics/persona-engine.md` | docs | added: the persona engine's data flow |
| `docs/specs/SPEC-044-persona-engine-and-private-roster.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-044-private-roster-topic-binding-and-scoped-memory.md` | docs | changed: accepted |
| `docs/red-first/SPEC-044.md` | docs | added |
| `changelog.d/feat-persona-044.md` | repo | added |

## 5. What this does NOT do

- It drafts no persona name, bio or voice; the architect's draft and the owner's approval stay in
  private configuration (#163).
- It reads no leech, drill grade or lapse: those sources are wired by their own features
  (#133, #136, #83).
- It reads no live CEFR band; the Road to C2 wires it (#85).
- It ships no test-preparation coach template and runs no practice set (#52).
- It runs no duty but the daily reading's goldens; the other duties' personas speak in their own
  deliveries (#48, #51).
- It commits no pack verdict and no pack wiring: the box run judges the rows, and the maintainer
  changes the private wiring's states (#60).

## 6. Risks

- **A real name or a subject list reaches the repository in an example or a golden.** Detected by
  the public scrub, persona-core's `scrubber` row and review; the examples use neutral placeholders
  and synthetic topics.
- **A golden output drifts from a pack's format after a re-pin.** Detected by the box run, which
  runs every output row over the goldens (B2), and by A6, which holds them to the engine's
  frontmatter.
- **A template drifts from its pack after a re-pin.** The box run judges the copies against the
  pinned pack's rows at every run (B1), so a new rule the copies break fails the run.
- **A memory source is wired later without the subject scope.** Prevented by the reader's type
  (built for one subject) and detected by A5 in each wiring delivery.
- **The roster's band lags the owner's real level** until the live band exists. Visible in each
  language output's `cefr`, and corrected in the private roster.

## 7. Amendments at delivery

- **R1: the templates are compiled in.** The engine compiles every file of `agent/personas/` into
  its binary and loads them at start, so the text it instantiates is the text the box run judged.
  A1 holds the compiled set equal to the directory, template by template (ADR-044).
- **R2: the roster's setting, file name and shape.** `DECKSTREAK_AGENT_ROSTER` names the roster by
  an absolute path. The private rail places it as `roster.json`, the name `privacy.json`'s `private`
  globs hold (`roster.json` and `*/roster.json`). Its keys are `schema`; `personas`, each template id
  the roster fills, with its four slots; and `topics`, each topic key, with its `template` and, for a
  language mentor's template, its `cefr` band. Any other key is refused.
- **R3: a slot is one line.** A slot's value is not blank and holds no control character, no `{{`
  and no `}}`, because a line break could open a section of its own in the persona and a brace pair
  could leave a slot token behind. A topic bound to a language mentor's template needs a band, and
  any other topic takes none.
- **R3 and R10: a refusal names the rule.** No refusal names a topic key, a template id, a slot's
  value or the roster's path, since a refusal reaches the journal; it names the rule and, for a
  slot, the slot's public name. The roster's `Debug` shows its counts, and the persona's never its
  text.
- **R5 and R7: the reader's record.** A read is recorded once per source, when its port answered, in
  the order of first read; a declared source that no port serves reads as nothing and is not
  recorded. The ports are asynchronous (the kernel's `PortFuture`), because their sources read the
  database. The journal is refused by name: a template that declares `journal` or `diary` does not
  load.
- **R8: a live band that has none.** A live band's port that answers no band for the subject gives
  the roster's band, as an unwired port does.
- **R9: the goldens' further keys.** The law golden also carries `sources` and `x-new-cards`, and the
  language golden `x-new-words`, after the engine's keys: SPEC-046's duty writes them. A6 holds each
  golden's opening lines to the engine's frontmatter.
- **The criteria over pack rows.** No public test can run a pack's row (ADR-069), so A1, A3 and A6
  are tests of the engine, and the box run judges the rows themselves (§3a).
