---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The persona engine: a private roster file binds topics to templates, the engine writes the frontmatter, and memory is read through a subject-scoped reader

## Context and Problem Statement

The persona packs fix the public half: neutral templates with four roster slots, the output
contract, and the rule that a persona reads only its own subject's memory and never the journal.
The private half is DeckStreak's to decide: where the roster lives, which persona teaches which
readings topic, who writes an output's `memory` declaration, and which CEFR band a language mentor
uses before the live Road-to-C2 band exists (a later wave). A static check can read only what an
output declares, so the declaration must be true by construction.

## Decision Drivers

- Public templates, private roster (constraint 18); the owner approves the roster once.
- persona-core requires `cefr` on every language output, so a language reading needs a band from
  its first night.
- A declared read must equal the actual read; a model-written declaration could lie.
- No new edge: the agent context depends on the kernel only.

## Considered Options (the alternatives it was chosen against)

- A private JSON roster file at a configured path, binding each topic key to a template id, filling the four slots, and giving each language topic a band until the live band exists; the engine writes the output's frontmatter from what it did; memory read only through a reader built for one subject that logs each read — chosen: the private half is one reviewable document, and every declaration is the engine's record.
- Keep the roster in Secret Manager — rejected because a roster is private configuration, not a credential, and the owner reviews it as a document; the private rail delivers it like any other private file.
- Keep the roster in the database, seeded at deploy — rejected because the owner approves it once as a document, and a table would put names and bios into every export.
- Bind topics to personas in the readings taxonomy — rejected because which mentor teaches a topic is the persona roster's concern, and the readings context would then have to know templates.
- Let the model write the output's frontmatter — rejected because it could declare reads it never made, or omit ones it did, and persona-core's `memory-scope` row can read only the declaration.
- Wait for the live band before generating any language reading — rejected because no language reading could exist until the curriculum wave, against the owner's decision that readings are the first delivery.

## Decision Outcome

Chosen option. The roster is `deckstreak.agent.roster.v1` JSON outside the repository, named by
configuration and placed by the private rail; `agent/roster.example.json` shows its shape with
neutral values. The engine instantiates a template by plain-text substitution, assembles each
output's frontmatter itself, and declares in `memory` exactly the reads its subject-scoped reader
logged. Until each memory source is wired, a persona reads nothing and says so. A language output's
band is the live band when that port exists and the roster's band otherwise.

### Decisions the delivery made (SPEC-044 §7)

- **The templates are compiled into the engine.** Every file of `agent/personas/` is compiled into
  the binary and loaded at start, so the text the engine instantiates is the text the box run
  judged. Chosen against loading a configured directory at start, which adds a setting and a
  second deployed artifact, and lets a running engine read text no run judged.
- **The engine refuses a filled slot itself.** A template loads only when each of the four slots
  stands in its section (`identity` holds `{{name}}` and `{{bio}}`, `voice` holds `{{voice}}`,
  `personality` holds `{{personality}}`, `disclosure` holds `{{name}}`) and no other `{{...}}`
  token is in it. Chosen against leaving the refusal to the box run alone, which judges the tree
  and not the text a build compiled in.
- **The roster's setting, file name and shape.** `DECKSTREAK_AGENT_ROSTER` names the roster by an
  absolute path, and the private rail places it as `roster.json`, the name `privacy.json`'s
  `private` globs hold. It has three keys: `schema`; `personas`, each template id the roster fills,
  with its four slots; and `topics`, each topic key, with its `template` and, for a language
  mentor's template, its `cefr` band. Any other key is refused, so a misspelt key never passes
  silently. Chosen against a roster keyed by persona name (the name is a slot, which the owner may
  change) and against a band per template (two topics of one language could not then differ).
- **A slot is one line of plain text.** A slot's value is not blank and holds no control
  character, no `{{` and no `}}`. Chosen against free text, where a line break in a bio could open
  a section of its own inside the persona and a brace pair could leave a slot token behind.
- **A refusal never quotes the roster.** A refusal names the rule and, for a slot, the slot's
  public name; never a topic key, a template id, a slot's value or the roster's path, and the
  roster's own `Debug` shows only its counts. Chosen against naming the entry, which is easier to
  fix but writes which subjects the owner runs into the journal (SPEC-044 R10).
- **The journal is refused by name.** A template that declares `journal` or `diary`, persona-core's
  journal names, is refused as naming the journal, and no memory source can be one. Chosen against
  refusing it as an unknown source, which would not say why.
- **The reader's record is what a port answered.** A read is recorded once per source, when its
  port answered, in the order of first read; a declared source that no port serves reads as
  nothing and is not recorded. A read of another subject, or of a source the template does not
  declare, is refused before the port is called. Chosen against recording each attempt, which
  would declare a read that never returned anything.
- **The ports are asynchronous.** A memory source's port and the live band's port return the
  kernel's `PortFuture`, because the contexts behind them read the database. Chosen against
  synchronous ports, which would block the runtime or need an offload in every wiring.
- **A live band that has none falls back.** A live band's port that answers no band for the
  subject gives the roster's band, as an unwired port does. Chosen against refusing the reading,
  which would stop a language's readings until the curriculum knows its band.

### Consequences

- Good, because persona-core's `memory-scope` row judges a declaration that cannot be false.
- Good, because the language readings run from the first deploy.
- Good, because a mistake in the roster refuses start by its rule, and the refusal names nothing
  private.
- Bad, because the roster's band can lag the owner's real level until the live band is wired; each
  output names the band it used, and the owner corrects the roster.
- Bad, because a new template needs a build: the engine loads no template at run time.

### Confirmation

SPEC-044's tests (A1 to A8) and its hand-proved rows (the subject filter, the journal exclusion and
the band's source), and the persona packs' rows over `agent/personas/` and `agent/golden/`, which
the box run judges (SPEC-044 B1 and B2, ADR-069).

## What would make this wrong

- The owner wants to change a persona's binding from the Mini App (then the binding moves into the
  database behind the settings screen).
- A memory source cannot be read without touching another subject's rows (then its port needs a
  per-subject query the source context must offer).
- A template must change between releases (then the templates move to a directory the deploy
  places, and the box run judges that directory).

## More Information

SPEC-044; ADR-069; the persona-core, law-professors and language-mentors packs; owner gate OWN-G4
in docs/OWNER-SETUP.md.
