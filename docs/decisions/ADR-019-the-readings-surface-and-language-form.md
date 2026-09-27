---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The readings: the Mini App is the primary surface, and a language reading is its mentor's daily reading

## Context and Problem Statement

No daily reading has ever reached the owner: the predecessor's lane refused every night on a
stale-snapshot gate and never wrote a note. The owner answered the open reading decisions
(staleness, the read tap, XP, length, surface, drafts). The predecessor deferred the language form,
whose "hook sheet" existed only as a specification; the owner since chose language mentors whose
daily reading carries glosses of the day's new words, a grammar spotlight, pronunciation and
script notes, and a culture note.

## Decision Drivers

- The owner's reading decisions (SPEC-001, section readings) are binding requirements.
- Fail loud; never a placeholder; honest states (no new cards, could not tell, failed, paused).
- The persona packs' output contract, and the study-duties and learning-science checks.

## Considered Options (the alternatives it was chosen against)

- The Mini App is the primary reading surface and the vault keeps an archive copy; a law reading is its professor's IRAC primer of 800 to 1500 words scaled to the topic's new cards; a language reading is its mentor's daily reading (i+1 text with glosses of today's new words, grammar, pronunciation, culture) — chosen: the owner's surface decision and persona decisions, and one output contract for every reading.
- The predecessor's language hook sheet — rejected because the owner's language-mentor decision covers its intent (card-anchored new words with glosses) inside one persona contract, and the hook sheet was never built.
- Keep the vault note as the primary surface — rejected by the owner ("Mini app first, vault copy").
- Port the predecessor's lane as it was — rejected because it never produced a reading, and its freshness gate refused every night the owner did not study.

## Decision Outcome

Chosen option. A reading is generated whenever the last sync succeeded (not when the collection
file is new), for each topic with new cards today, with no daily cap. Readings pause after two
days without study, and the owner gets one comeback reading per lapse, inside the comeback cap. The
read tap writes the vault's "I read it" line only on the owner's tap, and code never clears it.
XP is 40 on read plus 60 when 80% of its new cards are studied within two study days, at most 100
per reading, granted once. The W1 SPECs carry each rule as an acceptance criterion.

### Consequences

- Good, because the first reading can reach the owner in the first deploy.
- Bad, because generation depends on the agent's path to the proxy; the Mini App shows the honest reason when a reading is unavailable.

### Confirmation

The W1 SPECs' acceptance tests; the persona, study-duties, learning-science and nudge-duties rows over the committed golden readings.

## What would make this wrong

- The owner revises a reading decision (a new ADR supersedes this one).

## More Information

The second-brain inventory's readings analysis (private input); the language-mentors, law-professors, study-duties and learning-science packs.
