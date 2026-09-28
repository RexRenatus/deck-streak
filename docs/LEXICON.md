# Lexicon

The ubiquitous language: one name per concept, per context (docs/CONTEXT-MAP.md). Each fence
line is `<the one name> [in <context>, ...]: <the words it replaces>`, and the box-run ddd
probe's `lexicon-locks` class refuses an identifier declared in a scoped context that says a
replaced word (split into snake and camel segments, singular or plural). Comments and
string contents are prose, not declarations. A new term, or a new replaced word, is added in the
SPEC that introduces the concept.

```lexicon
reading in deck-streak-readings: preread, prereading, prestudy
topic in deck-streak-readings: lane
comeback in deck-streak-readings, deck-streak-notifications: ticket
xp in deck-streak-progression, deck-streak-readings: points, exp
coin in deck-streak-economy, deck-streak-discipline, deck-streak-markets, deck-streak-quests: gold, gem, credit
chest in deck-streak-quests: lootbox, crate
streak in deck-streak-streaks: combo, chain
freeze in deck-streak-streaks: shield
lapse in deck-streak-streaks, deck-streak-notifications: absence, dropout
duty in deck-streak-agent: chore
persona in deck-streak-agent: character, avatar
owner in deck-streak-identity: admin, superuser
celebration in deck-streak-notifications: fanfare, confetti
study_day in deck-streak-kernel, deck-streak-analytics: calendar
leech in deck-streak-curriculum: hardcard
```

## Glossary

| term | context | meaning |
|---|---|---|
| study day | kernel | the day as the scheduler counts it: it turns over at 04:00 local, never at midnight, and every screen shows the server's study day |
| track | kernel | `language` or `law`: a first-class dimension of XP, streaks and the daily rollup |
| verdict | kernel | the outcome of a check or a transition, which a caller must match on (`#[must_use]`) |
| settings generation | kernel | the owner-config generation: one counter, bumped inside every write that changes a runtime setting's value, which the change gate compares |
| data-rights port | kernel | what every stateful context implements: each of its tables once, with what an export and an erase do to it, and its export and its erase |
| review | ingest | one answer the owner gave in Anki: a revlog row of type 0 to 3 with ease 1 or more |
| day set | readings | the new cards Anki's scheduler has queued for today, attributed to topics |
| topic | readings | one reading subject: a law subject or a language, derived from the deck tree at run time, never hard-coded |
| reading | readings | one generated pre-study text for one topic on one study day, with an identity of topic, first generation and digest |
| pause | readings | two or more days without study: daily readings stop until the owner studies again |
| comeback reading | readings | the one reading offered once per lapse, inside the comeback cap |
| lapse | streaks | the governor's episode of three or more zero-review study days, identified by a lapse id |
| XP | progression | experience points, granted once per (day, source, track) through the grant port; never confiscable |
| coin | economy | the only confiscable stake, under the daily loss cap and the zero floor |
| chest | quests | a session reward whose rarity is rolled once on the server and stored; never sold |
| celebration | notifications | an event the router may render at a tier T0 to T5, once ever per event key |
| nudge | notifications | a message that asks for study, under budgets, quiet hours and the lapse rule |
| duty | agent | one kind of AI work: a daily reading, the digest's coaching, a drill, a leech remedy, a writing correction, a conversation turn, a practice set, the daily note, the weekly synthesis, inbox filing, or the comeback |
| persona | agent | a named mentor instantiated from a public template and the private roster |
| gate | agent | the packs' blocking checks, run on an output before it is delivered |
| owner | identity | the single learner the deployment serves, pinned by configuration |
| leech | curriculum | a card with eight or more lapses that is not suspended |
