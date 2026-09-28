# Schematic: the persona engine

Kind: data flow. Drawn for SPEC-044 at DeckStreak `dev` 16ed8e2, from constraint 18, ADR-044 and
the persona packs' template schema v1 and output contract v1 (persona-core, law-professors,
language-mentors). The engine loads the public templates and the private roster at start, fills a
template's four slots for a topic and a duty, reads the subject's memory through a reader built for
that subject, and writes the output's frontmatter from what it did. SPEC-043 composes the prompt
and gates the output; SPEC-046 writes the reading's body and its seed-derived keys.

```mermaid
flowchart TD
  templates["agent/personas: the public templates, compiled into the binary"] --> load{"load each template: schema v1, id, subject, lang, duties, memory; each slot in its section; no other slot"}
  load -- "refused" --> down["start refused, naming the rule"]
  load -- "ok" --> set["the template set"]
  setting["DECKSTREAK_AGENT_ROSTER, an absolute path"] --> file[("the private roster file, deckstreak.agent.roster.v1, placed by the private rail")]
  file --> roster{"read the roster: known templates; the four slots, one line each; each topic bound to a template; a band on each language topic"}
  set --> roster
  roster -- "refused" --> down
  roster -- "ok" --> ready["the roster: its Debug shows counts, never a name, a topic or a path"]
  ready --> instantiate{"persona for a topic and a duty: is the topic bound, and does its template offer the duty?"}
  instantiate -- "no" --> refused["refused by the rule; nothing instantiated"]
  instantiate -- "yes" --> persona["the persona: the template's body with its four slots filled by plain-text substitution; never logged or written"]
  persona --> reader["a memory reader built for the persona's subject and its declared sources"]
  reader --> read{"a read: the reader's own subject, and a source the template declares?"}
  read -- "another subject, or undeclared" --> refusedread["refused before the port is called; nothing recorded"]
  read -- "yes, and no port serves the source" --> nothing["nothing read, nothing recorded"]
  read -- "yes, and a port serves it" --> port[("a source port: leeches, drill grades or lapses, wired by its own feature")]
  port --> record["the reader records source at subject, once per source"]
  persona --> band{"a language persona: is the live band's port wired, and does it answer a band?"}
  band -- "yes" --> live["the live band"]
  band -- "no" --> rosterband["the roster's band for the topic"]
  record --> frontmatter["the engine writes the frontmatter: schema, persona, subject, duty; cefr and lang on a language output; memory is the record"]
  live --> frontmatter
  rosterband --> frontmatter
  frontmatter --> output["the output: the frontmatter, then the model's body, gated before delivery (SPEC-043)"]
```

The journal has no path into this flow: no memory source can be named `journal` or `diary`, and a
template that declares one does not load.

| guard | where it holds | proved by |
|---|---|---|
| no filled slot | loading a template | A1, A3; B1 in the box run |
| the four slots filled, one line each, no unknown slot | reading the roster | A2, A7 |
| the duty offered | instantiation | A7 |
| another subject's memory refused before any input or output | the reader | A5; row S04401 |
| the journal is never a source | the source names, loading a template | A5; row S04402 |
| the declaration equals the reads | the frontmatter | A4, A6 |
| the band's source | the band's resolution | A8; row S04403 |
| nothing private logged | the roster's, the persona's and the path's `Debug`; every refusal | A2, A7 |
