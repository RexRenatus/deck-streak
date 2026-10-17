---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-377: the web review grades with Again and Good, and a stored mapping drops the others when read

Decides SPEC-366 (issue `#712`). It is built on the owner's grading decision, which settles the
question issue #175 raised: grading is two buttons everywhere. ADR-376 decides the engine's half
and picks the pair, Again and Good; this record decides how the web review offers that pair on
each of its surfaces, before the engine refuses the other two.

**Amends, by an insert-only amendment section appended at its end, with no existing line changed:**
- ADR-342's mapping: the web maps two grades, Again and Good. The native list changes with the
  native half of the work.

## Context and Problem Statement

At `dev` `dee337bc` the web review offers four grades on every surface it has: four buttons
(`AnswerButtons.svelte:12-17`), four grade cells and `RATING = { again: 1, hard: 2, good: 3,
easy: 4 }` (`review.ts:56`, `:79`), keys `1` to `4`, gamepad buttons 12 to 15 and both axes of
the stick (`mapping.ts:8-44`), four rows on the mapping screen, a stored mapping that may name any
of the four (`mapping-store.ts:19-29`), a page protocol whose `Rating` is `1 | 2 | 3 | 4`
(`protocol.ts:40`), and 14 messages naming Hard and Easy across 7 locales. The two-button rule
needs two. The engine at `dev` still accepts all four wire ratings, so the web can narrow first.

## Decision Drivers

- Every surface that grades a card offers exactly two grades: Again and Good (ADR-376 D4).
- No press records a grade the learner did not press: a gesture that named Hard records nothing,
  never another grade.
- A learner's stored mapping keeps every binding it can, and no new write path is added.
- The wire numbers stay as the engine reads them: 1 for Again and 3 for Good (ADR-376 D8).
- The web lands before the engine refuses 2 and 4, so no pull request leaves a press refused.

## Considered Options (the alternatives each was chosen against)

### D1. The keys

- Key `1` is Again and key `3` is Good, and keys `2` and `4` fire nothing — chosen, because the key a learner presses keeps the number the wire sends, and a press from habit records nothing.
- Keys `1` and `2` — rejected, because key `2` was Hard, so a press from habit would record Good, a grade the learner did not press.
- Keys `2` and `4` kept as aliases of Again and Good — rejected, because a Hard press would silently record Again, and an Easy press Good.

### D2. The gamepad buttons

- Button 14 (d-pad left) is Again and 15 (d-pad right) is Good, unchanged, and 12 and 13 (d-pad up and down) fire nothing — chosen, because the two grades keep the places a learner already uses for them.
- Moving the grades to the d-pad's up and down — rejected, because it moves both grades a learner already knows.
- Buttons 12 and 13 kept as aliases — rejected, because a Hard or Easy press would record another grade.

### D3. The stick

- Axis 0 is Again to the left and Good to the right, unchanged, and axis 1 fires nothing — chosen, because it is D2's layout on the stick.
- Axis 1 kept as aliases — rejected, for D2's reason.

### D4. A stored mapping that names Hard or Easy

- It loads with those pairs dropped by the filter that already keeps only the named intents (`mapping-store.ts:56-58`), every other pair kept, and nothing is written until the learner next changes a mapping — chosen, because it adds no write path and no migration, and each read gives the same answer.
- Rewriting the stored value when it loads — rejected, because it adds a write on every load and an order between two tabs reading the same storage, for a value every read already drops.
- Rebinding a stored Hard pair to Again and an Easy pair to Good — rejected, because a key the learner bound for Hard would then record Again without being asked.
- Discarding a stored mode that names Hard or Easy — rejected, because it loses every other binding the learner made.

### D5. The answer buttons and the engine's four labels

- Two buttons, Again showing the engine's first label and Good its third (`labels[0]`, `labels[2]`), in two columns — chosen, because each button shows its own grade's interval and the engine's view is unchanged.
- Narrowing the engine's card view to two labels — rejected, because it is an engine change, outside the surfaces this delivery owns.
- Showing the first two labels in order — rejected, because Good would then show Hard's interval.

### D6. The page's protocol

- `Rating` is `1 | 3`, and both `rate`'s and `answer`'s rating parse only as 1 or 3, refused as malformed otherwise — chosen, because the page then sends no grade it does not offer, whichever op carries it.
- Leaving the protocol at four until the engine refuses — rejected, because a page with two buttons would still parse four ratings.
- Narrowing `rate` alone — rejected, because `answer` would still parse 2 and 4 until the engine delivery removes it.

### D7. The messages

- `study_hard` and `study_easy` leave all seven locales — chosen, because no surface names them.
- Keeping them unused — rejected, because every locale would carry two strings no screen reads, and each translation would still be kept.

### D8. What holds the two grades

- A text census over the shipped sources and the locale files, with examined counts and plants (SPEC-366 R9), beside the narrowed type — chosen, because the cast at `review.ts:280` lets any `RATING` compile, so only a read of the text holds it.
- The narrowed type alone — rejected, because of that cast.
- Removing the cast in this delivery — rejected, because `RATING` is partial over every action, so dropping the cast needs a new guard on the grade cell's path, a change of behaviour the two-button rule does not ask for.

### D9. The order

- The web lands first, before the engine refuses 2 and 4 — chosen, because the engine at `dev` accepts 1 and 3, so the web stands alone and no press is ever refused (ADR-376 D12).
- After the engine delivery — rejected, because the web's Hard and Easy presses would be refused until it landed.
- Composed with the engine delivery — rejected, because the web needs nothing from the engine change, so composing them only enlarges one review.

### D10. The records

- ADR-342 gains an insert-only amendment section for the web's two grades; SPEC-343's default map and SPEC-350 R7 and A13 are recorded in SPEC-366 and not edited; the schematic's two lines change in place — chosen, because each record that states four grades for the web then states two, and each SPEC keeps what it shipped.
- Amending ADR-342 only with the native half — rejected, because ADR-342 would state four grades for the web while the web offers two.
- Amending ADR-361 — rejected, because none of its lines names the web's four grades: its D5, D10 and D15 name the map, the stored mapping and the readers, and those stand.
- Editing SPEC-343's and SPEC-350's text — rejected, because a SPEC records what its delivery shipped, and this SPEC records the change.

### D11. The schematic

- No new schematic; `docs/schematics/web-study-screens.md` names two grades at `:26` and `:120` — chosen, because no component, state, edge or store is added or removed.
- A new schematic of the two grades — rejected, because it would draw the same machine with two edges fewer.

### D12. Model and mutation

- No model, and no mutation rows: StrykerJS mutates each changed production file whole in CI — chosen, because no state, actor or write path is added, and the rows table has no killer for a web test.
- A TLA+ model of the stored mapping — rejected, because the read drops pairs in memory and writes nothing, so there is no interleaving to explore.
- Rows whose Python killers read the web sources' text — rejected, because such a killer tests the text and not the behaviour, which StrykerJS's run already tests.

## Decision Outcome

D1 to D12 as chosen above. The web review offers Again and Good on its buttons, keys, remote and
stick; a stored mapping's Hard and Easy pairs are dropped when it is read; the page's protocol
parses two ratings; the 14 Hard and Easy strings go; and a census holds the grade cell and the
locales.

## Consequences

- Good: every web surface offers exactly two grades, and the engine delivery can refuse 2 and 4
  with no press refused.
- Good: no new write path, store or state.
- Bad: a press of key `2` or `4`, d-pad up or down, or the stick's axis 1 does nothing and says
  nothing.
- Bad: a stored Hard or Easy pair stays in storage, unread, until the learner next changes a
  mapping.
- Neutral: the engine's view still carries four interval labels, and the web shows two of them.

### Confirmation

SPEC-366's A1 to A12, the census's plants, and the `mutation-web` verdict on the pull request.

## What would make this wrong

- If the owner's session finds the silent keys confusing, the review answers a dropped key with a
  word, and the keys stay unbound.
- If a stored mapping is read by anything other than `pairs()`, the stored value is rewritten
  (D4's rejected first option) and a model of the write is decided then.
- If the two grades are not Again and Good, `RATING`, the keys, the buttons and the stick change
  together, and nothing else does.
