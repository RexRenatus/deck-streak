# Red-first record: SPEC-368

A1's test is committed before the page edit it judges, and was read red at that commit. A2 to A5
are guards over a tree that is already clean, so none is red at the base; each one's planted input
is refused first, and its refusal is quoted below as the proof the guard can fail.

```red-first
A1: red at d6816970: AssertionError: "deckstreak does not train, fine-tune or fit a model on your data, ..." not found in ''
A1: green at 8791ba41
A2: not red: the base's allow-list holds no fitting method; the plant is refused: the engine allow-list holds ['SchedulerService.ComputeFsrsParams']: the section `## Models and your data` of PRIVACY.md says what DeckStreak does with a learner's data, so amend that section and its changelog entry in the same delivery
A3: not red: the base's engine crates name no fitting call; the plant is refused: an engine crate names a fitting call at ['crates/planted/src/lib.rs:1']: the section `## Models and your data` of PRIVACY.md says what DeckStreak does with a learner's data, so amend that section and its changelog entry in the same delivery
A4: not red: the base's two AI tasks hold no tool; the plant is refused: the AI task(s) ['planted-task'] hold a tool: the section `## Models and your data` of PRIVACY.md says what DeckStreak does with a learner's data, so amend that section and its changelog entry in the same delivery
A5: not red: the base holds only the exempt lines; the plant is refused: an embedding or similarity word at ['crates/planted.rs: let v = embedding(card);']: the section `## Models and your data` of PRIVACY.md says what DeckStreak does with a learner's data, so amend that section and its changelog entry in the same delivery
```
