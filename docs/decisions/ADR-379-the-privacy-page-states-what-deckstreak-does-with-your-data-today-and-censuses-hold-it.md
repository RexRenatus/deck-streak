---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-379: the privacy page states what DeckStreak does with your data today, and censuses hold it

Decides SPEC-368 (issue `#722`). `PRIVACY.md` ends with an unheaded sentence that claims
DeckStreak never uses a learner's data to train a model. This record decides what replaces it,
where the new text sits, how it is disclosed, how it is held by tests, and what it does not
promise.

**Amends:** no record. No line of an earlier SPEC or ADR changes.

## Context and Problem Statement

At `dev` `b0935c75` the sentence is the last line of `PRIVACY.md` (`:120`), after the last bullet
of `## What an erase does not reach`, so by position it belongs to that section. No test pins it.
The app does hold things a reader could call fitting or training: scheduling parameters read from
the learner's collection, the card memory state the collection stores, and two AI duties that send
a learner's cards and a summary of their practice as the context of one run. None fits or trains
anything (SPEC-368 section 1b), and no code computes an embedding or similarity data from a
learner's data (section 1c). The sentence says less than is true, in the wrong place, in absolute
words, and a fitting call could land with the page unchanged.

## Decision Drivers

- Every clause of the new text is true at `dev`, and none describes a capability the app lacks.
- The page makes no promise about the future that the owner has not made.
- A change that adds a fit, an AI tool or an embedding computation cannot land with the page
  unchanged.
- The page names no provider and no version.
- `privacy.json` keeps its closed keys.

## Considered Options (the alternatives each was chosen against)

### D1. What the text says

- State what happens today: no model is trained, fitted or built into a dataset, the scheduling parameters are read from the learner's collection, and an AI duty's inputs are the context of one run — chosen, because each clause is checked against the code and none claims more than it.
- Keep the absolute sentence — rejected, because it is a promise about all time, with no heading, that nothing holds, and it would bar a fit of a learner's scheduling parameters to that learner's own reviews that the owner has not ruled out.
- List every model and engine by name — rejected, because it names a provider and versions, which public text does not carry, and the list would go stale with the first change.

### D2. Where it sits

- Its own heading, `## Models and your data`, in the sentence's place — chosen, because the sentence sat inside the erase section by position only, and a heading takes it out.
- Leave it at the end of the erase section — rejected, because a reader looking for what happens to their data would find it under what an erase leaves behind.

### D3. How the change is disclosed

- `PRIVACY.md` plus a changelog fragment with a `### Changed` bullet, and `privacy.json` unchanged — chosen, because it is the house way for a policy edit, and no category, store or purpose changes.
- A change key in `privacy.json` — rejected, because its keys are closed and a new one refuses `inventory-valid`.
- A dated notice on the page — rejected, because public text carries no dates.

### D4. How it is held

- One pin on the paragraph and four guards, each with a planted control refused first, whose red message names the page amendment owed — chosen, because a pin alone would let a fitting call land with the page unchanged, and a message that names the amendment tells the next author what to do.
- A pin on the paragraph alone — rejected, because it holds the words and not the claim.
- Guards that only fail, with no message about the page — rejected, because the author would be told a call is forbidden and not that the forbidding is the page's to change.

### D5. The provider sentence

- Add "What the model's provider keeps is set by that provider's terms." as the paragraph's last sentence — chosen, because it is generic, true, and silent on no party DeckStreak cannot speak for.
- Omit it — rejected, because the page would say nothing of the one party whose retention DeckStreak does not control.
- Name the provider — rejected, because public text names no provider.

### D6. The future

- State present behaviour only, and promise nothing about later — chosen, because the sentence must be true of `dev`, and a promise that excludes a per-learner refit would bind the owner to a stance the owner has not taken.
- Pre-announce a future fit on the page — rejected, because it would describe a capability the app does not have.
- Promise that no fit will ever be added — rejected, because it is the absolute sentence again.

### D7. Embeddings and similarity

- One planted census over `crates/`, `web/` and `ios/` for the words `embedding`, `similarity` and `cosine`, exempt by exact path and line text where the word means frame embedding or a declared list, each exemption required to match — chosen, because the paragraph's "builds no dataset" has no other guard, and exact exemptions keep four known lines from making the guard blind.
- A ban on the word across the repository — rejected, because it fires on the two frame-embedding comments and the declared list.
- No check — rejected, because the claim would rest on review alone.

### D8. Model and mutation

- No formal model and no mutation band — chosen, because no production code changes and each guard carries its own planted control.
- A band of rows over the test module — rejected, because rows are for production code and a guard's plant already proves it can fail.

## Decision Outcome

D1 to D8 as chosen above. `PRIVACY.md` gains `## Models and your data` in the sentence's place,
stating present behaviour only; a changelog fragment discloses it; `privacy.json` is unchanged;
`test_privacy_policy.py` pins the paragraph and guards the engine allow-list, the engine crates,
the AI tasks and the embedding words; and every guard's red message names the amendment owed.

## Consequences

- Good: the page says what the app does with the data a reader would ask about, in one place, in
  words each checked against the code.
- Good: adding a fit, an AI tool or an embedding computation turns a test red whose message names
  the page amendment owed.
- Good: the page binds the owner to nothing about later.
- Bad: the guards read eight names and three words, so a fit under another name is caught by
  review.
- Bad: A5 skips drill-named paths while that surface is parked.
- Neutral: the page's text is now pinned, so rewording it is a test edit too.

### Confirmation

SPEC-368's A1 to A5, their planted controls, and the build's recorded counts of engine methods,
engine files and crates, tasks, and files examined.

## What would make this wrong

- If the owner decides DeckStreak will fit a learner's scheduling parameters, the section is
  amended in that delivery, and A2 and A3 change with it.
- If an AI duty gains a tool or a second input, A4 fails and the section's AI clause is rewritten
  first.
- If a reader takes "does not fit them to your reviews" as a promise, D6's wording needs a
  plainer present-tense frame.
- If the guards' words prove too narrow, D7's list grows by a delivery that names the miss.
