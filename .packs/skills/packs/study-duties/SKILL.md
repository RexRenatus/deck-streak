---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/study-duties

The duty-quality checks for DeckStreak's six study duties (SPEC-V2-2216 / ADR-V2-2216). The owner
chose "Agent DUTY packs. ALL were chosen." and bound how they compose: "The duty packs COMPOSE with
persona packs: a duty names WHAT is produced, and the persona shapes HOW." Blocking is for facts,
privacy and safety, advisory for voice and heuristics, and there are no dates or timelines anywhere.

A study-duty output IS a persona output (`phx.persona.output.v1` with a `duty`), so this pack parses
it through persona-core's API and never re-parses it. It adds four things:

- six duty-instruction templates, `templates/<duty>.duty.md`: the "duty's instructions" step of the
  engine's prompt;
- the study-duty output extension v1: the `retrieval` section and two `x-` declarations;
- fourteen classes in `scripts/study-duties-probe.py` that hold each duty's STRUCTURE;
- fifteen `compose` rows that call the classes this duty composes with, and never copy them.

Which seats consume the pack is its catalog row's `consumes`, the one record of that edge.

```
phxd pack probe --pack study-duties --root PATH --format json
```

## The six duties

The registry is persona-core's `contract.json`; this pack adds sections and never removes one.

| duty | persona-core's sections | this pack adds | what the output must show |
|---|---|---|---|
| `daily-reading` | reading | retrieval, last | a law primer of 800 to 1500 prose words; closing retrieval prompts |
| `drill-coach` | drill | none | the drill asks before anything tells |
| `leech-doctor` | explanation, mnemonic, contrast | none | a new angle, a short mnemonic, a contrast against the confusable card |
| `writing-tutor` | corrections | none | corrections that point at the learner's words, and a next step |
| `conversation-partner` | reply | none | one turn, never a scripted exchange |
| `practice-questions` | questions, explanations | none | one keyed answer per item, and every choice explained |

## The rows

Twenty-nine rows, all `tree`-scoped, each under a 120-second wall. `{skills}` is the skills
directory the catalog was read from, so each script and its data come from their own packs,
whatever tree `--root` names. The first fourteen run
`python3 {skills}/../scripts/study-duties-probe.py --root {root} check <class>`; the `compose` rows
run the named class of the pack that owns it.

Stage `contract`: 2 rows (2 blocking, 0 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `study-duty-templates` | block | `duty-template-invalid` | a registry duty has no template, or two; a template does not parse as persona-core's rules document, names a duty off the registry, lacks a `produces`, `sections`, `rules` or `fail-loud` section, or never names a section its duty requires |
| `study-extension` | block | `duty-extension-invalid` | `x-new-cards` is not a whole count of 1 or more, or sits on another duty; a law daily reading does not declare it; `x-leech` is not `{"card", "confusable", "answer"?}` with a card term and a confusable term or null, or sits on another duty |

Stage `readings`: 2 rows (1 blocking, 1 advisory)

| row | severity | reason | refuses or reports when |
|---|---|---|---|
| `reading-length` | block | `reading-out-of-band` | a law daily reading's primer holds fewer than 800 or more than 1500 prose words |
| `reading-scale` | advisory | `reading-not-scaled` | a law reading with more new cards is shorter than one with fewer, by more than the tolerance |

Stage `leeches`: 4 rows (1 blocking, 3 advisory)

| row | severity | reason | refuses or reports when |
|---|---|---|---|
| `leech-contrast` | block | `leech-contrast-missing` | with `x-leech` declared: the explanation never names the card, or the contrast never names the confusable card (the card itself when there is none) |
| `leech-pair-declared` | advisory | `leech-pair-undeclared` | a leech-doctor output declares no `x-leech`, so its contrast cannot be checked |
| `leech-angle` | advisory | `leech-restated` | 70% or more of the explanation's words are the card's declared answer |
| `leech-mnemonic` | advisory | `mnemonic-long` | the mnemonic runs more than 40 words |

Stage `assessment`: 3 rows (2 blocking, 1 advisory)

| row | severity | reason | refuses or reports when |
|---|---|---|---|
| `question-key` | block | `question-key-invalid` | an item's marker is not JSON, names no id or no key, or repeats an id; it has fewer than 2 choices, or they skip a letter; its key is not one of its choices; its explanation credits no choice, several, or one that is not the key; the questions section reveals an answer |
| `choices-explained` | block | `choice-unexplained` | an item has no explanation block, a choice has no entry, a reason runs under six words or repeats another; open questions outnumber their explanations; the set holds no question |
| `distractor-cues` | advisory | `distractor-cue` | the key is conspicuously the longest choice; a choice is all or none of the above; a stem's `except` is lowercase; a choice says always, never, completely or absolutely; an item offers fewer than 3 choices; every item of a set keys the same letter |

Stage `feedback`: 1 row (0 blocking, 1 advisory)

| row | severity | reason | reports when |
|---|---|---|---|
| `feedback-specific` | advisory | `feedback-vague` | a correction points at none of the learner's words: no quote, strike-through, code span or arrow |

Stage `integrity`: 2 rows (2 blocking, 0 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `no-placeholder` | block | `placeholder-text` | a section is an empty stand-in (`...`, `n/a`, `none`), or any line, comments included, holds a work marker or a placeholder: lorem ipsum, a bracketed or angled fill-in, TBD, "coming soon", "goes here", "unavailable", a failed generation, model boilerplate |
| `reply-one-turn` | block | `reply-scripts-a-turn` | a conversation reply holds two or more speaker-labelled lines, in any of six languages |

Stage `compose`: 15 rows (10 blocking, 5 advisory)

| row | severity | reason | the owner's class |
|---|---|---|---|
| `learning-science.retrieval-prompts` | block | `retrieval-missing` | a daily reading ends with a `retrieval` section of two or more prompts |
| `learning-science.answers-hidden` | block | `answer-revealed` | no answer label shows outside a spoiler in the retrieval, drill or questions sections |
| `learning-science.ask-before-tell` | block | `told-before-asked` | a drill's first item is a prompt, and questions come before their explanations |
| `learning-science.deep-questions` | block | `deep-question-missing` | the retrieval prompts include a why, how or explain prompt |
| `learning-science.key-terms-defined` | block | `key-term-undefined` | every key term of a reading is defined, used and not repeated |
| `learning-science.concrete-examples` | advisory | `example-missing` | a reading grounds the abstract in a concrete example |
| `learning-science.worked-example-steps` | advisory | `worked-example-incomplete` | a worked example shows its steps, then prompts |
| `learning-science.feedback-task-focused` | advisory | `feedback-person-focused` | feedback is about the task, not the person |
| `learning-science.feedback-next-step` | advisory | `feed-forward-missing` | a writing-tutor output feeds forward |
| `law-professors.rule-cites-corpus` | block | `rule-uncited` | every law rule statement cites the corpus |
| `law-professors.citations-resolve` | block | `citation-unresolved` | every citation resolves in the corpus manifest |
| `language-mentors.cefr-ratio` | block | `instruction-off-mode` | the language of instruction fits the CEFR band |
| `language-mentors.i1-glosses` | block | `gloss-missing` | every new word of a reading is glossed |
| `language-mentors.write-focus` | advisory | `feedback-unfocused` | one sample's corrections treat three categories or fewer |
| `lsat-coach.every-choice-explained` | block | `choice-unexplained` | every LSAT choice is explained |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`, the
outputs it read. An output class counts every persona output and owes nothing for an output its
duty does not reach, such as a drill for `reading-length`. It exits 0 when green, 1 on a finding, 2
on a usage error, and 3 when VOID: no persona output was examined, or persona-core's probe or
`duties.json` could not be read. VOID is never a pass. A claimed output that does not parse is a
finding in every output class, never a skip. The card turns any non-zero exit of a `block` row red,
and prints an advisory row's failure as `advisory` without reddening the card.

On phoenix-v2's own tree the rows examine every persona output the persona packs ship, and this
pack's worked examples in `examples/`. A committed example is judged like any output.

## The study-duty output extension, v1

It adds to persona-core's output contract through `x-` keys and sections, and changes nothing there.

| key or section | on | required | value |
|---|---|---|---|
| `retrieval` section | daily-reading | yes, as the LAST section | two or more prompts, each a list item ending in `?` or `？`, or opening with a recall verb; answers hidden |
| `x-new-cards` | daily-reading | on a `law` reading | the topic's count of new cards today, a whole number of 1 or more |
| `x-leech` | leech-doctor | recommended; advisory when absent | `{"card": "<term>", "confusable": "<term>" or null, "answer": "<the card's answer>"}` |

**The law band.** A law daily reading's primer is its body without the closing `retrieval`
section. Its prose words are the words outside headings, HTML comments, fenced code, Pandoc
citations (`[@id, locator]`) and link targets, counted as Unicode word tokens. The owner's band is
800 to 1500 of them, and length grows with `x-new-cards` inside it. A language reading takes its
mentor's form and no band.

**The leech pair.** The explanation names the card, and the contrast names the confusable card; it
may call its own card "this card". A term is named when every word that tells it apart from the
other card's term stands in the text as a whole word, an English plural allowed: against
"necessary assumption", the contrast may name "sufficient assumption" by "sufficient". Words are
read one by one, so a term broken across a line wrap still counts. A Chinese, Japanese or Korean
term, which has no word boundaries, is matched as a substring with whitespace ignored.

## The multiple-choice format: lsat-coach's, adopted

Practice questions of every subject use `phx.lsat.coach.v1`, so the engine writes one format and
the LSAT coach's checks and these agree.

- **Questions.** One `###` block per item. Its heading carries the key invisibly:
  `### Question 1 <!-- lsat-item {"id": "q1", "type": "flaw", "provenance": "original", "key": "B"} -->`.
  One line per choice follows, `(A) ...`, in order. The questions section reveals nothing: no
  `credited` entry, no `Answer:` label and no "the answer is".
- **Explanations.** One `###` block per item, matched by the marker's `id`, not by position:
  `### Question 1 <!-- lsat-item {"id": "q1"} -->`. One entry per choice:
  `(B) credited: <reason>` for the key and `(A) incorrect, <trap>: <reason>` for each distractor.
  Exactly one credited, equal to the key, and every reason at least six words.
- **An official item** (`"provenance": "official"`) is cited by its locator and never reproduced,
  so it has no choice lines; its explanation entries give its choice letters.
- **Open questions**, such as a law issue-spotter's calls, are numbered list items, each with a
  numbered explanation.

## How DeckStreak's engine runs these checks

1. **Compose the prompt** in persona-core's order: `rules.md`; the instantiated persona template;
   then this pack's `templates/<duty>.duty.md` for the duty; the subject's memory as data; the
   learner's text fenced as untrusted.
2. **Write the output** in persona-core's contract, with this pack's declarations: `x-new-cards` on a
   law reading and `x-leech` on a leech output.
3. **Gate before delivery.** With `--subject` naming the output's directory and the templates, run
   persona-core's blocking classes, the subject persona pack's, this pack's blocking classes, and the
   classes the `compose` rows name:
   `python3 scripts/study-duties-probe.py --root . --subject DIR check <class>`. A red blocking class
   means the text is not delivered: regenerate it, or report that it is unavailable. Never deliver a
   stand-in. On the box, also run vault-duties' `journal-never-leaks` with `--subject DIR --vault
   VAULT`: only there can it prove that no text repeats a journal note.
4. **Gate in CI.** Run `phxd pack probe --pack study-duties --root PATH` over the repository's golden
   outputs. A compose row is VOID on a tree with no output of its pack's subject, so keep a golden
   output for each subject the engine runs.

What the engine owns, and this pack teaches but no text check can prove:

- one reading for each topic with new cards and no daily cap, and no reading without new cards;
- `x-new-cards` equal to the real count, and `x-leech` naming the card the learner actually failed;
- a leech output ready before that card's next review;
- tell sections, spoilers and answers kept hidden until the learner has attempted the drill;
- spacing and interleaving, which the scheduler decides;
- the XP a reading or a graded drill earns.

What the gate refuses: a law primer outside 800 to 1500 words, or one that does not declare its new
cards; a leech contrast that never names the confusable card it was written against; an item whose
key is missing, doubled, not a choice, or revealed in the questions; a choice left unexplained; a
placeholder; a reply that scripts both sides of a conversation; and whatever the composed classes
refuse.

## References

The dated research record, with access dates and every Context7 id, is SPEC-V2-2216's References.

- Roediger and Karpicke (2006), "Test-enhanced learning"; Dunlosky et al. (2013), "Improving
  Students' Learning With Effective Learning Techniques".
- Pan and Carpenter (2023), prequestioning and pretesting; Kornell, Hays and Bjork (2009),
  "Unsuccessful retrieval attempts enhance subsequent learning".
- Butler and Roediger (2008), feedback after multiple-choice testing.
- Haladyna, Downing and Rodriguez (2002), the 31 multiple-choice item-writing guidelines; the NBME
  Item-Writing Guide, 6th edition.
- Wozniak, "Twenty rules of formulating knowledge" (interference, rules 11 and 13); the Anki manual,
  Leeches; Kang and Pashler (2012); Birnbaum, Kornell, Bjork and Bjork (2013); Bellezza (1981).
- Hattie and Timperley (2007); Shute (2008); Ellis (2009); Kang and Han (2015); Lyster and Ranta
  (1997); Lyster and Saito (2010).
- Brysbaert (2019), reading rate; Atkinson, Derry, Renkl and Wortham (2000), worked examples;
  Kalyuga, Ayres, Chandler and Sweller (2003); Cepeda et al. (2006), distributed practice.
