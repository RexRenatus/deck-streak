---
name: lsat-coach
description: One LSAT coach persona template for DeckStreak, with the checks that keep its practice sets, explanations and logic true to the current test. Use it to instantiate the coach, to gate every generated LSAT text before delivery, and to refresh the test's facts.
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/lsat-coach

ONE LSAT® coach template, and the checks that keep it and every text an engine generates from it
true to the current test (SPEC-V2-2215 / ADR-V2-2215). The owner's words: "Professor per subject
+ LSAT coach", one coach covering LR, RC and logic, as the fourth of the DeckStreak persona packs.

It EXTENDS the persona contract of `packs/persona-core` and never redefines it. The template is a
persona template, its outputs are persona outputs, and every document is parsed by persona-core's
own probe. What the LSAT adds lives in one `x-lsat` frontmatter key, one `timing` section, and the
markers and `logic` blocks inside persona-core's sections.

What lives here:

- `templates/lsat-coach.persona.md`: the coach template. It carries the roster SLOTS and fills
  none of them.
- `lsat-format.json`: the current official structure of the test, every fact with its source.
- `taxonomy.json`: the Logical Reasoning and Reading Comprehension question types, each mapped to
  one skill the test maker lists, plus the formal-logic families and the trap vocabulary.
- `logic.json`: the conditional translation table, its equivalences, and the valid and invalid
  inference forms.
- `corpus.json`: what an output may cite.
- `examples/`: six worked outputs, which are the contract's reference and this pack's own subject.

`scripts/lsat-coach-probe.py` is the check. It is standard-library Python and vendorable beside
`scripts/persona-core-probe.py`, and it judges any tree through `--root`, or generated texts
through `--subject`. Which seats consume this pack is its catalog row's `consumes`, the one record
of that edge, so this body names none.

```
phxd pack probe --pack lsat-coach --root PATH --format json
```

## The rows

Twenty-four rows, all `tree`-scoped: twenty blocking and four advisory.
- The sixteen rows of the `format`, `taxonomy`, `logic` and `provenance` stages run
  `python3 {skills}/../scripts/lsat-coach-probe.py --root {root} check <class>`.
- The eight `persona-core.*` rows of the `schema`, `safety` and `voice` stages run persona-core's
  own classes,
  `python3 {skills}/../scripts/persona-core-probe.py --root {root} --kind test-prep check <class>`.
  They are reused, never copied, and each row id names the class it runs.

Every row has a 120-second wall.

Stage `format`: 5 rows (4 blocking, 1 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `format-facts` | block | `format-fact-wrong` | `lsat-format.json` is inconsistent. Its sections are not its scored plus unscored sections, a retired section is listed as current, the unscored section is not a current type, the choice letters or the scale are malformed, the RC passage ranges cannot make its sets, or a fact's source does not resolve in the corpus |
| `retired-section-taught` | block | `retired-section-taught` | a template or output names a retired section, "logic games" or "Analytical Reasoning", in a sentence that does not mark it retired, or a set is in a retired section |
| `timed-set` | block | `timed-set-invalid` | a practice set's `x-lsat` is incomplete or disagrees with its items. A full section runs other than ceil(35 × `time_multiplier`) minutes, a full RC section is not 4 sets of 5-8 questions with 3-4 single passages and 0-1 comparative sets, the timing section omits the limit or the answer-every-question rule, an untimed set carries a clock, or any sentence claims a penalty for a wrong answer |
| `timed-set-pace` | advisory | `pace-off-official` | a drill's minutes stray more than one minute from the official pace (35 minutes for about 25 LR questions, 35 minutes for 4 RC sets, times the multiplier), or a full LR section is not 24-26 questions |
| `score-claims` | block | `score-claim-invalid` | coach text states a scaled score, a percentile or a score band. The scale fact, 120 to 180, is the one exception |

Stage `taxonomy`: 5 rows (3 blocking, 2 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `taxonomy-complete` | block | `taxonomy-gap` | a skill the test maker lists is covered by no type, a type maps to a skill of another section or belongs to a retired section, a type id repeats or has no cues, a formal-logic family is not an LR type, or a trap repeats |
| `question-taxonomy` | block | `question-type-unknown` | an item's type is not in the taxonomy, or is of the other section; a leech-doctor or drill-coach output names no types, or an unknown one; or an output lacks the `phx.lsat.coach.v1` extension |
| `stem-cue` | advisory | `stem-reads-as-other-type` | an original item's stem carries no cue of its declared type, and reads as another |
| `every-choice-explained` | block | `choice-unexplained` | an item lacks five choices, (A) to (E), or its explanation misses a choice. Other failures: it credits other than exactly one choice, credits a choice that is not the item's key, gives a reason under six words or the same reason twice, or reveals an answer in the questions section |
| `trap-named` | advisory | `trap-unnamed` | an incorrect choice names no trap, or one outside the taxonomy's closed list |

Stage `logic`: 2 rows (2 blocking, 0 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `logic-rules-sound` | block | `logic-rule-unsound` | a translation in `logic.json` breaks its equivalence group or its converse pair, or an inference's `valid` flag disagrees with the truth table |
| `logic-valid` | block | `logic-invalid` | a `logic` block's line is malformed or unclosed, a letter is undeclared, a `translate` line disagrees with the table, or a `valid`, `invalid`, `equivalent` or `inequivalent` claim is false by truth table |

Stage `provenance`: 4 rows (4 blocking, 0 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `template-contract` | block | `template-incomplete` | an LSAT template lacks the extension contract, the practice-questions `timing` section, or a rule of its method. The rules are: the current sections, the retired section marked retired, every answer choice explained, attempt before the review, answers from the given text, reading all five choices, the time multiplier from configuration, plain words, causal kept apart from conditional, and no scaled scores. It also refuses a template that carries a time multiplier value |
| `item-provenance` | block | `provenance-invalid` | an item or passage is neither `original` with its text nor `official` with exactly one located citation to an official-items source and no text. Other failures: a comparative passage lacks passage A or passage B, an original item sits on an official passage, or an item has no key |
| `sources-cited` | block | `source-uncited` | `sources` is missing or does not resolve in the corpus, a citation is not in `sources`, a source is never cited, or a timed set's timing section lacks a format source or a scoring source |
| `lsac-marks` | block | `lsac-mark-misused` | a template carries no non-affiliation notice with the ® mark; text claims the coach is official, endorsed, certified or affiliated; or the persona introduces itself by a name carrying a mark |

Stage `schema`: 3 rows (3 blocking, 0 advisory). `persona-core.template-schema`,
`persona-core.output-contract` and `persona-core.duty-composition`, over the test-prep documents.

Stage `safety`: 4 rows (4 blocking, 0 advisory). `persona-core.no-dates`, `persona-core.scrubber`,
`persona-core.memory-scope` and `persona-core.no-human-claim`.

Stage `voice`: 1 row (0 blocking, 1 advisory). `persona-core.voice`.

Each class prints `<class>: <finding>` lines and ends with `examined N`. N is the entries a data
class validated, or the documents an output class read. The probe exits 0 when green, 1 on a
finding, 2 on a usage error, and 3 when VOID. VOID means persona-core is absent, a data file is
unreadable, or no document was examined; it is never a pass.

## The test, as this pack states it

`lsat-format.json` holds these facts, each with its source in `corpus.json`:
- **Sections.** The multiple-choice part has four sections of 35 minutes: two scored Logical
  Reasoning, one scored Reading Comprehension, and one unscored section of either type, placed
  anywhere. A 10-minute intermission follows section 2.
- **Answering.** Each question has five choices, (A) to (E), and asks for the best one. The score
  is the number answered correctly, with no deduction for incorrect answers and every question
  weighted the same. It is converted per form to a scale of 120 to 180.
- **Logical Reasoning** gives one question per short passage, about 25 per section.
- **Reading Comprehension** is four sets of 5-8 questions: three or four single passages, and zero
  or one comparative set.
- **Analytical Reasoning, the logic games, is no longer part of the test.** Logical Reasoning
  covers the deductive reasoning it focused on, which is what "logic" means here.
- **LSAT Argumentative Writing** is separate and unscored.
- **Accommodations.** A learner with extended time gets a multiplier, and a section's minutes are
  rounded up.

## The question-type taxonomy

The test maker lists skills, not question types, so every type below maps to exactly one listed
skill, and a drill grade recorded by type always rolls up to an official skill.

| section | skill, as the test maker lists it | types |
|---|---|---|
| LR | Recognizing the parts of an argument and their relationships | `main-conclusion`, `role`, `method` |
| LR | Recognizing similarities and differences between patterns of reasoning | `parallel-reasoning`, `parallel-flaw` |
| LR | Drawing well-supported conclusions | `inference`, `most-strongly-supported` |
| LR | Reasoning by analogy | `principle-parallel` |
| LR | Recognizing misunderstandings or points of disagreement | `point-at-issue`, `point-of-agreement` |
| LR | Determining how additional evidence affects an argument | `strengthen`, `weaken`, `evaluate` |
| LR | Detecting assumptions made by particular arguments | `necessary-assumption`, `sufficient-assumption` |
| LR | Identifying and applying principles or rules | `principle-justify`, `principle-apply` |
| LR | Identifying flaws in arguments | `flaw` |
| LR | Identifying explanations | `resolve` |
| RC | The main idea or primary purpose | `rc-main-point` |
| RC | Information that is explicitly stated | `rc-detail` |
| RC | Information or ideas that can be inferred | `rc-inference` |
| RC | The meaning or purpose of words or phrases as used in context | `rc-meaning-in-context` |
| RC | The organization or structure | `rc-structure` |
| RC | The application of information in the selection to a new context | `rc-application` |
| RC | Principles that function in the selection | `rc-principle` |
| RC | Analogies to claims or arguments in the selection | `rc-analogy` |
| RC | An author's attitude as revealed in the tone of a passage or the language used | `rc-attitude` |
| RC | The impact of new information on claims or arguments in the selection | `rc-new-information` |
| RC | how the two passages of a comparative reading set relate | `rc-comparative` |

The formal-logic families, the owner's "logic", are `inference`, `necessary-assumption`,
`sufficient-assumption`, `flaw`, `parallel-reasoning`, `parallel-flaw` and `principle-apply`.

The traps an incorrect choice may name are these:
- `true-but-unresponsive` and `outside-knowledge`, from the test maker's own approach;
- `unsupported`;
- `out-of-scope`, `too-strong`, `too-weak`, `half-right`, `opposite`;
- `reversal`, `negation`, `necessary-vs-sufficient`;
- `correlation-causation`, `irrelevant-comparison`, `wrong-part`.

## Timed sets

- A set holds one section's questions: `set_kind` is `section`, `drill` or `untimed`.
- A full section runs 35 minutes times the learner's time multiplier, rounded up. Time-and-a-half
  is 53 minutes.
- A drill runs at the official pace.
- The multiplier is private configuration. It never appears in a template, and an output records
  only the one it was generated with.
- The timing section states the limit and the scoring rule's consequence, answer every question.
  It cites a format source and a scoring source.
- The learner answers every item before the review. Nothing in the questions section reveals an
  answer.

## Explanations

Every item's explanation gives each of the five choices its own line.

```text
(A) incorrect, out-of-scope: why it fails, in at least six words.
(B) credited: why it is the best answer.
```

- Exactly one choice is credited, and it is the item's key, fixed when the item was written.
- The four incorrect reasons differ from one another, and each names its trap.
- This follows the test maker's own sample explanations. Feedback after a multiple-choice test is
  what stops its lures being learned as facts.

## Logic blocks

A text shows its conditional work in a fenced `logic` block, and the probe proves every claim by
truth table:
- **Letters.** `let A = <phrase>` declares a letter, and every letter used is declared.
- **Translations.** `translate: "A only if B" => A -> B` must agree with the pack's table.
- **Claims.** `valid: P; Q |= C` must follow, and `invalid: P |/= C` must not.
  `equivalent: F == G` and `inequivalent: F =/= G` are decided the same way.

The notation uses capital letters, `~` for "not" (`!` is read too), `&`, `|`, `->` and `<->`. Write
`~` in outputs, because persona-core's voice check counts a `!` as an exclamation mark even inside
code. Reasoning with "most" and "some" is not truth-functional, so the block's grammar refuses it
and it stays in prose.

## Provenance, citations and marks

- **Original items** carry their full text.
- **Official items** carry one citation and no text:
  - the citation is `[@lsac-lr-samples, question 5]`, or `[@<preptest-id>, section 2, question 14]`
    for a PrepTest a deployment adds through `--corpus`;
  - the test maker's questions are copyrighted and are never reproduced;
  - the explanation names the choices by letter.
- **Citations** use Pandoc syntax, `[@id]` or `[@id, locator]`, the same as `packs/law-professors`.
  An output's `sources` lists exactly the ids its body cites.
- **The template** carries the test maker's non-affiliation notice with the ® mark.
- **Names and claims.** No text claims the coach is official, endorsed or certified. The persona's
  name, which the owner approves, carries no mark of the test maker.

## The output contract, `phx.lsat.coach.v1`

An extension of persona contract v1. Every LSAT template and output carries
`x-lsat: {"contract": "phx.lsat.coach.v1", ...}`, and the remaining keys depend on the duty:

| duty | `x-lsat` keys | sections, beyond persona-core's |
|---|---|---|
| `practice-questions` | `set_kind`, `section`, `items`, and for a timed set `minutes` and `time_multiplier` | `timing` |
| `leech-doctor` | `types` (required) | none |
| `drill-coach` | `types` (required) | none |
| `daily-reading` | `types` and `concepts` (optional) | none |

Inside `questions`, each item is a level-3 heading ending in
`<!-- lsat-item {"id","type","provenance","key"} -->`, followed by its stimulus, its stem and five
choice lines. An RC passage ends in `<!-- lsat-passage {"id","kind","provenance"} -->`, and each of
its items names `"passage"`. Inside `explanations`, each item's heading carries
`<!-- lsat-item {"id"} -->` and five choice lines. The worked examples show every shape.

## How the DeckStreak engine uses this

1. **Vendor.** Copy both probes and this pack's directory, with its data and examples, beside
   persona-core's. Instantiate the template with the owner-approved roster.
2. **Generate and gate.** For each generated text, run every blocking class before delivery:
   `python3 scripts/lsat-coach-probe.py --subject <the template> --subject <the text> check <class>`,
   and persona-core's classes with `--kind test-prep`. Add `--corpus <private file>` for the
   PrepTests a learner can open. A red blocking class means the text is not delivered.
3. **Run it in CI.** Run the pack over the tree, with the examples as the subject, through
   `phxd pack probe --pack lsat-coach --root PATH --format json` or the vendored probes.
4. **Show the notice.** Every page of the app that shows the coach's text also shows the
   non-affiliation notice, as the test maker's guidelines ask of a site.

## Refreshing the facts

When the test changes:
1. Re-research against the test maker's own format, scoring and question-type pages, and a disclosed
   form, with WebSearch and Context7.
2. Edit `lsat-format.json`, and `taxonomy.json` if the skills change, keeping every source.
3. Update the committed-pack tests that pin the verified values.
4. Run the pack.

`format-facts` checks consistency; only the refresh can check truth.

## References

The test maker (primary): the types of LSAT questions, the Logical Reasoning and Reading
Comprehension pages with their suggested approaches and sample questions, LSAT Scoring, the
specifications of the LSAT and LSAT Argumentative Writing, the intermission page, the accommodation
FAQs, a disclosed form, and the trademark guidelines. Each is listed with its URL in `corpus.json`.

Logic: forall x: Calgary, chapter 5, and the Stanford Encyclopedia of Philosophy's "Necessary and
Sufficient Conditions". Learning research: Butler and Roediger; Marsh, Roediger, Bjork and Bjork;
Van der Kleij, Feskens and Eggen; Bisra and colleagues; Dunlosky and colleagues; Brunmair and
Richter; Bastani and colleagues; Schroeder, Adesope and Gilbert. The full study, with access dates,
the disclosed-form measurements and the Context7 ids, is SPEC-V2-2215.
