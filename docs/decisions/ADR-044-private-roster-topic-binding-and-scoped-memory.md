---
status: proposed
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

### Consequences

- Good, because persona-core's `memory-scope` row judges a declaration that cannot be false.
- Good, because the language readings run from the first deploy.
- Bad, because the roster's band can lag the owner's real level until the live band is wired; each
  output names the band it used, and the owner corrects the roster.

### Confirmation

SPEC-044's tests and the persona packs' rows over `agent/personas/` and `agent/golden/`.

## What would make this wrong

- The owner wants to change a persona's binding from the Mini App (then the binding moves into the
  database behind the settings screen).
- A memory source cannot be read without touching another subject's rows (then its port needs a
  per-subject query the source context must offer).

## More Information

SPEC-044; the persona-core, law-professors and language-mentors packs; owner gate OWN-G4 in
docs/OWNER-SETUP.md.
