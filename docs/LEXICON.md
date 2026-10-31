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
xp in deck-streak-progression, deck-streak-readings, deck-streak-xp: points, exp
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
review in deck-streak-collection: revlog
preset in deck-streak-collection: deckconfig, dconf
notetype in deck-streak-collection: model
media_hash in deck-streak-media: checksum, fingerprint
binding in deck-streak-media: attachment
sweep in deck-streak-media: purge, cleanup
operation in deck-streak-sync: mutation, changeset
conflict in deck-streak-sync: clash
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
| review facts | xp | the three facts of one answer the XP rule reads: its ease, its new interval and its review type; progression builds them from an ingest review |
| coin | economy | the only confiscable stake, under the daily loss cap and the zero floor |
| chest | quests | a session reward whose rarity is rolled once on the server and stored; never sold |
| celebration | notifications | an event the router may render at a tier T0 to T5, once ever per event key |
| nudge | notifications | a message that asks for study, under budgets, quiet hours and the lapse rule |
| duty | agent | one kind of AI work: a daily reading, the digest's coaching, a drill, a leech remedy, a writing correction, a conversation turn, a practice set, the daily note, the weekly synthesis, inbox filing, or the comeback |
| persona | agent | a named mentor instantiated from a public template and the private roster |
| gate | agent | the packs' blocking checks, run on an output before it is delivered |
| owner | identity | the single learner the deployment serves, pinned by configuration |
| leech | curriculum | a card with eight or more lapses that is not suspended |
| grade | engine-core | the owner's judgement of one answered card, Again or Good: the engine records it as rating 0 or 2, and no type names a third |
| owner answer | engine-core | the token one press mints for one card and one grade (`OwnerAnswer`): the dispatcher's one door that records a grade consumes it, and refuses a request that names another card or another grade |
| store | collection | DeckStreak's own collection, one SQLite file per user with one schema on the server and each device, stamped with the application id `DSTC` |
| review | collection | one answer in the store's review log, made by a DeckStreak client or imported; a row never changes |
| retraction | collection | the row that withdraws one review from the effective log, which keeps the review itself |
| client record | collection | which client made a review (web, iOS or stock), one row beside it; no row reads unknown |
| origin | collection | a store's writer id, 16 bytes fixed when the store is made |
| slot | collection | a store's id slot: 0 for the server, 1 to 63 for an enrolled device |
| erase gate | collection | the one switch under which a review, a retraction, a client record or a field history row may leave; only an erase opens it |
| read port | collection | `CollectionRead`, what the server's contexts read of a collection, answered by the store or by ingest's schema-18 adapter |
| media hash | media | the SHA-256 of a media file's bytes: the one identity the media store knows a file by |
| certified name | media | `ds-`, a file's media hash in lowercase hex, and an optional extension: a name that carries its file's hash and reads back to itself |
| imported name | media | any other plain media name, brought by an import and kept exactly as written; it claims no hash |
| binding | media | a name's one hash, live while its file is held and released after a sweep deletes it; a name is bound once and comes back only for the same hash |
| sweep | media | the plan of which stored media files to delete: every file that is not live, pinned, in grace or imported |
| operation | sync | one change a device or the server makes to study data, typed (`Op`) and written in one byte encoding: an answer, an undo of one, or an edit of a note, card, deck, notetype or media name |
| server seq | sync | an operation's place in the server's log, given when the server accepts it; every device folds the log in that order and pulls by it |
| pending | sync | a device's own operation the server has not judged: it leaves the device's queue only on the server's verdict on it or its appearance in a pull |
| conflict record | sync | an edit that lost to a concurrent edit of the same register, kept with the winner's key, so no edit is lost silently |
| sealed event | sync | an event the devices encrypt and the server only relays and orders, unable to read it; a tag edit travels as one |
| hybrid clock | sync | an operation's clock: one past every clock its device made or saw, never below the device's wall clock |
| slot | sync | a number 0 to 63 given once per device (0 is the server's); every id a device makes has its slot as its residue mod 64 |
