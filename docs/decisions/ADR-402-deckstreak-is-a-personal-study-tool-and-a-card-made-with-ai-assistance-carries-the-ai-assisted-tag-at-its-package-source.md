---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-402: DeckStreak is a personal study tool, and a card made with AI assistance carries the `ai-assisted` tag at its package source

Decides SPEC-388 (issue `#840`). Before a card made with AI assistance ships, this record states
DeckStreak's intended purpose and reads the EU AI Act's Article 6 against it. It decides the one
neutral marker such a card carries, where it is recorded and how far it travels, what each client
shows, and the one sentence that tells the learner how such cards are marked.

## Context and Problem Statement

- DeckStreak helps one learner study their own collection. It reads the reviews the learner does,
  keeps a streak and a ladder per course, coaches through a bot whose coach says it is an AI at the
  first contact, and delivers vault cards as a package the learner imports (ADR-151).
- No card is made with AI assistance today (#379), and no tag, field or text names one
  (SPEC-388 section 1a).
- A package is the engine's own export of a throwaway collection with a frozen note type, and a
  packaged note changes only at its package's source (ADR-151, ADR-301). The owner's live-edit
  ruling (in `docs/rulings/`) has agents add what a deck needs at that source.
- So four things are open: what the app is for, under the Act; what marks a card made with AI
  assistance, where, and how far the mark travels; what each client shows; and what one sentence
  tells the learner.

## Decision Drivers

- The text a learner reads names no model and no maker, and claims nothing the app cannot know.
- The marker is recorded once, at the source, and travels with the card on every path the engine
  already carries, with no DeckStreak copy to drift.
- No new note type, store, table or AI task.
- One rule, in the engine core, for both clients (ADR-356 D4).
- The reading of the Act comes from its own text, and names what would change it.

## The intended purpose, read against the Act

Read from the Act's official text, and for Annex III from the Commission's AI Act Service Desk's
rendering of it. Articles are cited by number.

- **The intended purpose.** DeckStreak is a personal study tool. One learner studies their own
  collection on their own devices, and the AI in it coaches that learner and may assist in making
  cards for that learner's own decks. It is not offered to, or put into service by, an educational
  or vocational training institution, and nothing in it admits, assigns, grades, places or proctors
  anyone.
- **Article 6(1)** makes a system high-risk when it is a safety component of a product, or is
  itself a product, under the Union harmonisation law listed in Annex I and needs a third-party
  conformity assessment. A study app is no such product or component, so it does not apply.
- **Article 6(2) and Annex III.** The Annex's education area, point 3, lists four uses: deciding
  access or admission to institutions, or assigning persons to them (a); evaluating learning
  outcomes, including where they steer a person's learning (b); assessing the level of education a
  person will receive or can access (c); and monitoring and detecting prohibited behaviour during
  tests (d). Each of the four is scoped to educational and vocational training institutions, in
  the context of or within them. DeckStreak's ladder and streak describe the learner's own study,
  for the learner, outside any institution. No point of Annex III describes its intended purpose,
  so Article 6(2) does not make it high-risk.
- **Article 6(3)** is a derogation for a system that Annex III lists. Such a system is not
  high-risk when it poses no significant risk of harm and meets one of conditions (a) to (d): a
  narrow procedural task, improving a completed human activity, detecting decision patterns under
  human review, or a preparatory task. Its last subparagraph keeps a listed system high-risk
  whenever it profiles natural persons. No point of Annex III applies to DeckStreak, so it needs no
  derogation. Article 6(4)'s documented assessment and Article 49(2)'s registration bind a provider
  who relies on the derogation, and do not arise here. This record is the written reading all the
  same.
- **Article 50 still applies.** Article 2(12) exempts systems released under free and open-source
  licences from the Act, but not from Article 50, and DeckStreak's repository is open.
  Article 50(1) asks that a person interacting with an AI system is told so; the coach's
  first-contact line already does that, and the AI-safety manifest declares it. Article 50(2) asks
  a provider of a system that generates text to mark its outputs in a machine-readable form,
  detectable as generated, unless the system only assists standard editing or does not
  substantially alter its input. The `ai-assisted` tag (D2) is the card's machine-readable mark,
  recorded whatever that exception would allow. Article 50(4) concerns text published to inform
  the public on matters of public interest, which a study card is not.
- **Article 2(10)** exempts a natural person who uses an AI system in a purely personal,
  non-professional activity from the deployer's obligations, which is how the learner uses
  DeckStreak.

**What would change the answer.** Each of these is a new ADR that supersedes D1.

- An institution putting DeckStreak into service, or DeckStreak being offered to one, to admit or
  assign learners, evaluate learning outcomes, set a level or watch tests. That is Annex III
  point 3, so high-risk under Article 6(2). The derogation would not cover it wherever it evaluates
  a learner's study performance, because that is profiling of a natural person (as GDPR
  Article 4(4) defines it), which Article 6(3) never derogates.
- An AI duty that grades or steers a learner for anyone but that learner.
- An AI duty in DeckStreak that itself makes cards. It is declared in the AI-safety manifest, its
  gate carries the output mark, and every card it makes carries the tag (D2).

## Considered Options (the alternatives each was chosen against)

### D1. The intended purpose, and Article 6

- Chosen: a personal study tool for one learner, read as outside Annex III and so not high-risk, with Article 50 kept, because every education point of Annex III is scoped to institutions, and the app serves one learner's own study.
- Chosen against: claiming the Article 6(3) derogation, because a derogation is for a system Annex III lists, so claiming it would assert a listing the purpose does not have and bind Article 6(4) and 49(2) duties for nothing.
- Chosen against: treating the app as high-risk out of caution, because a high-risk provider's obligations would follow a listing the Act's text does not make, and would read as a claim of an institutional use the app does not have.
- Chosen against: reading only summaries of the Act, because the word that decides the answer, institutions, sits in the Annex's own text, and a summary can drop it.

### D2. The marker

- Chosen: the note tag `ai-assisted`, written in the note's own tags at the package source, because a tag is the note's own data outside the frozen note type, travels with the note on every path the engine already has, and the learner can see it in their own collection.
- Chosen: a note is marked when its tags hold the tag as a whole tag, without regard to ASCII case, and a note without it is unmarked and claims nothing, because the engine compares tags that way and an absent mark is no statement about how a card was made.
- Chosen: the engine core holds the one rule (`ai_assisted::carries`) and the one read (`Dispatcher::ai_assisted`, through a fixed statement), the web engine's view carries the answer, and the web review shows one line above the card, because the core already holds the rules both clients share, as it holds the late rule.
- Chosen: no DeckStreak code carries the tag through export, import, update, the collection file or sync, because the engine carries a note's tags on each of those paths, which SPEC-388 A7 to A9 prove.
- Chosen against: a new field in the vault card's note type, because the type is frozen (ADR-151), a type change is a new type and a new ADR, and a field shows on a card only if a template prints it.
- Chosen against: a DeckStreak table or a server store keyed by note, because it is not at the source, does not travel with the package, the import or sync, and adds a store for a fact the note already carries.
- Chosen against: a mark in the package's deck name or description, because it marks a deck rather than a card, and a card moved to another deck loses it.
- Chosen against: a hierarchical tag such as `source::ai-assisted`, or counting child tags under `ai-assisted`, because one exact spelling is what a source writes and a reader checks, and a child tag is the owner's own filing.
- Chosen against: a case-sensitive match, because the engine treats tags without regard to case, and the owner's collection can spell the same tag `AI-Assisted`.
- Chosen against: each client matching the tag itself, because two copies of one rule drift (ADR-356 D4).
- Chosen against: admitting the engine's own note read as an adapter pair in the web engine's study table, because it widens a closed table to hand the page every field of a note when the view needs one answer.
- Chosen against: reading the tag in ingest and storing it on the server, because ingest keeps no tag by design and no server consumer needs it; the marker is a fact about a card, not about the learner, so the privacy inventory does not change.
- Chosen against: a line on the native review now, because the native app holds no localized strings (#738).
- Chosen against: wording that warns, ranks or credits (a warning word, a generated-by line, a named maker or model), because the line states a fact neutrally, and a maker's or a model's name is the text the requirement forbids.

### D3. The course sentence

- Chosen: one sentence on the progress screen, below the course list and only when a course is listed, `Cards made with AI assistance carry the tag ai-assisted.`, in all 7 locales with the tag kept literal, because the progress screen is the one screen that lists the learner's courses, the sentence names the tag the learner can find in their own collection, and it names no model and no maker.
- Chosen against: a sentence in each package's deck description, because a description is written per package and per deck, so it would be many sentences, and it is part of the package the owner edits.
- Chosen against: the persona disclosures, because they disclose the coach, a different fact, and there are many of them.
- Chosen against: the bot's first reply, because it already carries the coach's AI disclosure, and a second AI fact there blurs both.
- Chosen against: the about screen, because a learner studying a course does not look there.
- Chosen against: a sentence that names the tool, the model or its maker, because public text names none of them, and the name tells the learner nothing they need to study.
- Chosen against: the sentence on the progress screen with no course listed, because there is then nothing it describes, and the screen's one line says no course exists yet.

### D4. The tests and the model

- Chosen: red-first tests per criterion (the rule's arms over literal tag sets; the core's read against the engine's own note; the tag through the engine's own package export and import, an update in both directions, and full and normal sync through the engine's own sync server; a census of the web view; the line and the sentence drawn and absent; the locales; and a no-maker census with planted refusals), nine hand rows from `S38800`, and StrykerJS on the changed web files, because each claim is a function of one note's tags read in one synchronous call, and the paths the tag crosses are the engine's own, which the tests drive end to end.
- Chosen against: a TLA+ model, because no actor, store or write path is added, and the only movers of a tag are the owner's import and sync, each of which builds the view again.
- Chosen against: a Lean proof of `carries`, because no recorded failure motivates it, and its arms are few enough for the tests to cover each one.
- Chosen against: a deny list of maker and model names in a test, because the list would put those names in the public tree; the census reads the texts' shape instead, and the English texts are pinned whole.
- Chosen against: rows whose killers read the web sources' text, because StrykerJS mutates each changed production file under `web/app` on the pull request.

### D5. The shape

- Chosen: one delivery under SPEC-388 and ADR-402, because the work is one tag rule and one read in the core, one key in the web engine, one line and one sentence on the page, and their tests: 31 files, 14 of them fixture or locale edits of one or two lines, the scale of one builder's round.
- Chosen against: this ADR alone first and the marker after, because neither is useful without the other, and both are to be merged before a card made with AI assistance ships.
- Chosen against: amending ADR-151 and its SPEC, because they decide the package's build and send, which this does not change; the schematic that draws the package gets one appended section instead.

### D6. The view's key

- Chosen: `assisted`, one word, in the web engine's view and the page's `CardView`, because every key of that view and every field of the page's interfaces is one word (SPEC-388 section 1c).
- Chosen against: `aiAssisted`, because no key or field in either file is camel-cased, so it would be the first.
- Chosen against: `ai_assisted`, because no key or field in either file is snake-cased, and the page's fields are its own names, not the core's.

## Decision Outcome

DeckStreak's intended purpose is a personal study tool for one learner. Read against the Act, it is
not high-risk, needs no Article 6(3) derogation, and keeps Article 50: the coach says it is an AI at
first contact, and a card made with AI assistance carries the machine-readable tag `ai-assisted`,
written at its package source and carried by the engine wherever the note goes. The engine core
judges the tag, the web review shows `This card was made with AI assistance.` on such a card, and
the progress screen says `Cards made with AI assistance carry the tag ai-assisted.` once below the
courses. All of it lands in one delivery, before any card made with AI assistance ships.

## Consequences

### Good

- A card made with AI assistance says so wherever the owner's collection goes, in the owner's own
  tools too, with no DeckStreak store behind it.
- The learner reads one neutral line on such a card and one sentence on the course screen, and
  neither names a model or a maker.
- The intended purpose is written down, with what would change it.

### Bad

- The native review shows no line until the native app has localized strings (#738).
- A tag the owner removes reads unmarked until a newer package of the card (SPEC-388 section 5).
- The no-maker census cannot read a maker written in lowercase; review carries that.

### Neutral

- Ingest, the server, the privacy inventory and the AI-safety manifest do not change.

### Confirmation

- SPEC-388 A1 to A16, each red first where section 3 says so, then green; C1 to C4 on the pull
  request; V1 by the owner on a device.

## What would make this wrong

- An institutional use, or an AI duty that evaluates learners for others (D1).
- The engine merging tags when a newer package updates a note: the source could then never take a
  marker back (SPEC-388 A8).
- A package builder that writes another spelling, or a card-making AI duty that writes no tag.
