The owner amends ADR-301 (a), the never-list, and ADR-089 (ii) as ADR-301 binds it, in the owner's own words: "option 1 and 2, deckstreak will have the capability to be interchange based on user preference", "the agents should be able to add fields where necessary" and "create the commit for me to sign for the live edit mode": one declared write class, live edit, may change a reviewed note's type (entry 7, `docs/decisions/ADR-301-deckstreak-writes-to-the-collection-only-through-declared-write-classes.md:118`) and add fields to a note type that has reviewed notes, only while the owner's live-edit preference is on; entry 4 and every other entry stay bound on it, and every other path stays bound by entries 1-8 as ADR-301 states them.

# OWNER RULING 2026-10-07: live edit mode, by the owner's preference

## What was held

At dev `f3392ec3`, ADR-301 says "No write class makes any of these writes, at any rung, and no owner approval admits
one" (line 107). Three of its rules stop the agents from changing a note that is already in the collection:

| where | the rule | what live edit needs |
|---|---|---|
| ADR-301:118, entry 7 | a write class never changes a reviewed note's type | move a reviewed note to a note type with the fields it needs |
| ADR-301:61, :87 | a note type's fields are a write, and a write aborts on a one-way sync demand (ADR-089 (ii)) | add a field to a note type, which makes the next sync a one-way sync |
| ADR-301:132-134 | a packaged note (ADR-151) is changed at its package's source, never in the collection | change a packaged note where it is studied |

## What replaces it

- **Two modes, the owner's choice.** The default is unchanged: agents add the fields a deck needs at the package
  source, in the deck's own note types, and keep data a note type cannot hold in DeckStreak's own tables, keyed to
  the note. Live edit is the second mode. A user preference turns it on, and only the owner's own gesture in the study
  client sets that preference. It is off by default, and turning it off stops the class before its next batch.
- **One write class.** Live edit is a declared write class with its own ADR in ADR-089's form, before any of it is
  built. ADR-301 (b) to (f) bind it as they bind every class: the preview, the backup and its restore drill, the undo,
  the change budget, the promotion ladder, the single writer and the objective.
- **What it may change.** Only:
  - a reviewed note's type, when every card template of the old type maps to one of the new type, so every card
    keeps its id and its review history, and every non-empty field maps to a field;
  - a field added to a note type, or a field renamed or moved, on a note type that has reviewed notes.

  It never removes a field, never loses a field's content and never drops a card: a change that would do any of these
  is not made. Entries 1-3, 5, 6 and 8 bind it exactly as ADR-301 states them.
- **The one-way sync stays the owner's own act.** Entry 4 binds the class: it never forces a full or one-way sync.
  When its change makes the server ask for one, the class does not abort under ADR-089 (ii). It stops, makes no
  further change, and the change waits on the device until the owner's own one-way sync tap carries it, with that
  tap's guards (`docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md`). Each batch's preview says, before it runs,
  that it will need that sync and what the sync replaces.
- **Interchange.** A packaged note that live edit changes takes the same change at its package's source in the same
  batch, so the next newer package carries it and never reverts it. A note whose source cannot take the change is not
  edited live. The shape in which data moves between the two modes is the class's ADR to state.
- **Containment the build must prove.** A test proves that the class runs only while the preference is on, and that
  no other write class, agent route, server job or sync repair changes a reviewed note's type or a note type's fields.
- **Order.** Nothing builds or calls live edit before this ruling is signed and on dev. ADR-301 keeps its text; the
  delivery that builds the class adds a dated note at line 107 naming this ruling.

## Why it is admitted

It is a weakening: ADR-301 says no owner approval admits a never-list write, and this admits entry 7 and a note type's
field changes for one class. Its reach is narrow. It runs only while the owner keeps the preference on, every change
is previewed, backed up and undone like any class's, no card or field content is ever lost, and the one-way sync that
entries 4 and 7 protect stays the owner's own confirmed tap. DeckStreak is not bound by the limits of the client it
syncs with, and the owner wants agents able to add the fields that make study more effective, in whichever form the
owner prefers.

The rejected alternatives:
- **The package source alone.** It cannot change a note with no package source, and the owner chose both modes.
- **Letting the class make its own one-way sync.** It would overwrite other clients' unsynced changes with no owner in
  the loop, which is what entry 4 protects.
- **Exempting every agent and job.** It would break ADR-301 (d)'s single writer and (c)'s ladder.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
