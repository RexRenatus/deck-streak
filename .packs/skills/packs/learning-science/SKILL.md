---
name: learning-science
description: >-
  Teaches, and checks, the shared pedagogy every AI study text follows: retrieval practice that
  ends a reading, answers hidden until the learner tries, drills that ask before they tell, why
  and how questions, concrete and worked examples, interleaved practice, defined key terms, and
  feedback that addresses the work and says where to go next. Runs over the persona outputs of
  any repository through scripts/learning-science-probe.py --root, parsed by persona-core. Use
  when writing or reviewing a duty's instructions, a study text generator, or its outputs.
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/learning-science

The shared pedagogy classes every AI study text must follow (SPEC-V2-2224 / ADR-V2-2224). The
owner's words: "learning-science owns the SHARED pedagogy classes that every AI study text must
follow". DeckStreak's engine writes every study text as a persona output: the daily reading, the
drill, the leech remedy, the writing tutor's corrections, the conversation turn and the practice
set. A duty names what is produced, a persona shapes how, and this pack decides whether the text
teaches the way the evidence says it should.

Three things live here:

- **the classes**, `scripts/learning-science-probe.py`, standard-library Python that vendors
  beside `scripts/persona-core-probe.py` and reads any tree through `--root`;
- **the table**, [pedagogy.json](pedagogy.json): the duty-to-genre map, the section ids, the
  thresholds and the per-language patterns (en, fr, es, zh, ja, ko). A new language or a new
  genre is a row, never a code branch;
- **the teaching** below: what each class refuses, the evidence behind it, and what the classes
  cannot see and the engine must do instead.

Which seats consume this pack is its catalog row's `consumes`, the one record of that edge, so
this body names none.

```
phxd pack probe --pack learning-science --root PATH --format json
```

## How it composes

- **persona-core** owns the file format. This pack finds and parses outputs only through its
  `load_population` and `parse_document`, scans lines through its public `scan`, and reads the
  duty registry from its `contract.json`. It copies none of them.
- **study-duties** calls these classes: its rows run `learning-science-probe.py`, and its own
  checks keep the per-duty STRUCTURE (the leech doctor's three parts, assessment design, feedback
  specifics, the readings' length and scaling).
- **language-mentors** owns i+1 for language texts: its `i1-glosses` and `i1-density` rows check
  glossed new words and the 95% coverage floor. `key-terms-defined` here judges only the law,
  test-prep and general kinds, so the two never judge the same text.
- **law-professors** owns citation and IRAC; **lsat-coach** owns answer-choice explanations.

## The rows

Twelve rows, all `tree`-scoped, one per class of `learning-science-probe.py`. Each runs
`python3 {skills}/../scripts/learning-science-probe.py --root {root} check <class>` under a
60-second wall. A class judges the persona OUTPUTS whose duty maps to a genre it names; a claimed
output that does not parse is a finding in every class, never a skip.

Stage `contract`: 1 row (1 blocking, 0 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `pedagogy-contract` | block | `pedagogy-contract-invalid` | `pedagogy.json` has a wrong schema or an unknown key, a duty in persona-core's registry has no genre or a mapped duty is not in it, a class names an unknown genre or kind, a section id is not a slug, a threshold is not a whole number of at least 1, or a pattern does not compile |

Stage `retrieval`: 3 rows (3 blocking, 0 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `retrieval-prompts` | block | `retrieval-missing` | a daily reading has no `retrieval` section, the `retrieval` section is not the LAST section, or it holds fewer than 2 prompts |
| `answers-hidden` | block | `answer-revealed` | a prompt in a reading's `retrieval`, a drill's `drill` or a practice set's `questions` section shows an answer label (`Answer:`, `Réponse :`, `Respuesta:`, `答案：`, `答え：`, `정답:` and the rest of the table) outside a hidden block. `请回答：` is an instruction and passes |
| `ask-before-tell` | block | `told-before-asked` | a drill's first list item asks nothing, or a practice set's `explanations` section comes before its `questions`. A drill section with no list item is the debrief of a graded drill and passes; the feedback rows judge it |

Stage `elaboration`: 2 rows (1 blocking, 1 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `deep-questions` | block | `deep-question-missing` | no prompt in a law, test-prep or general reading's `retrieval` section asks why, how or explain in any table language. `How many` is recall and does not count. A language reading's comprehension questions are language-mentors' |
| `concrete-examples` | advisory | `example-missing` | a law, test-prep or general reading's `reading` section has no concrete-example marker (for example, e.g., 例如, 例えば, 예를 들어, par exemple, por ejemplo) and no quoted example of three words or more, and the text has no `worked-example`, `application`, `hypothetical`, `example` or `scenario` section. A language reading is itself the concrete input and is not judged. Advisory |

Stage `practice`: 3 rows (0 blocking, 3 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `interleaving` | advisory | `practice-blocked` | a law, test-prep or general drill or practice set holds at least 4 topic-marked prompts over at least 2 topics and every topic runs as one block. Language drills are not judged: interleaving shows no benefit for word learning. Advisory |
| `worked-example-steps` | advisory | `worked-example-incomplete` | a `worked-example` section shows fewer than 2 steps (ordered items or `Step N` labels), or no prompt follows it, inside it after the last step or in a later section. Advisory |
| `study-advice` | advisory | `low-utility-advice` | the text recommends cramming, massed study, rereading as the method, or highlighting, in any table language. `Reread the second paragraph and answer` is a task and passes. Advisory |

Stage `comprehension`: 1 row (1 blocking, 0 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `key-terms-defined` | block | `key-term-undefined` | a law, test-prep or general reading's `key-terms` section holds no item, an item names no term or gives no definition, a term is listed twice, or a term is never used outside the section. A reading without the section passes |

Stage `feedback`: 2 rows (0 blocking, 2 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `feedback-task-focused` | advisory | `feedback-person-focused` | feedback in a writing-tutor, drill, practice, leech-doctor or conversation output praises or blames the person (you're so smart, 你真聪明) or compares the learner with others (better than most learners). A blockquote is the learner's own text and is not read. Advisory |
| `feedback-next-step` | advisory | `feed-forward-missing` | a writing-tutor output has no feed-forward section (`next-step`, or the siblings' `next-steps` and `focus`), or it is empty once comments are removed. Advisory |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`: the
outputs of a genre it judges, plus the broken outputs. The script exits 0 when green, 1 on a
finding, 2 on a usage error, and 3 when VOID: nothing was examined, or the table or persona-core's
probe could not be read. VOID is never a pass. The pack's card turns a non-zero `block` row red
and prints an `advisory` row's failure without reddening the card.

Severity follows the evidence: `block` where the IES practice guide rates the practice STRONG
(quizzing to re-expose content, deep explanatory questions) or Dunlosky et al. rate it HIGH
utility (practice testing), and where the check is structural; `advisory` where the rating is
moderate or the check is a phrase heuristic.

## The contract a study text follows

**Genres.** `pedagogy.json` maps each persona-core duty to a genre: daily-reading is `reading`,
drill-coach `drill`, leech-doctor `remediation`, writing-tutor `feedback`, conversation-partner
`dialogue`, and practice-questions `practice`.

**Sections.** persona-core requires the registry's sections and lets a duty add more. The classes
read these added ids:

- `retrieval`: the retrieval set that ENDS every daily reading, at least 2 prompts;
- `key-terms`: new terms for a law, test-prep or general reading, `- **term** — definition`;
- `worked-example`: an example solved in at least 2 steps, followed by a problem;
- `next-step`: a writing tutor's feed-forward. law-professors writes `next-steps` and
  language-mentors `focus`; the table accepts all three, and new texts use `next-step`.

**A prompt** is a top-level list item that ends in `?` or `？`, holds a cloze blank (`___`), or
opens with a retrieval verb (`Name`, `Explain`, `Nommez`, `Explica`, `说出`) or closes with one
(`答えなさい`, `설명하세요`).

**A hidden answer** sits inside `<details>`, `<tg-spoiler>`, `<span class="tg-spoiler">` or a
MarkdownV2 `||spoiler||`. The Mini App renders `<details>`, and Telegram renders the spoilers.

**A topic marker** closes a prompt, `<!-- topic:<slug> -->`, and is invisible when rendered. The
engine knows each card's topic and writes it, so `interleaving` can see the order.

A daily reading that passes every class:

```markdown
## Key terms <!-- section:key-terms -->

- **duty of care** — the obligation to act as a reasonable person would.
- **breach** — conduct that falls below that standard.

## Reading <!-- section:reading -->

Negligence asks four questions, and the duty of care comes first. A breach is measured
against that duty. For example, a driver owes a duty of care to other road users.

## Check yourself <!-- section:retrieval -->

1. Why does negligence ask about the duty of care before the breach?
   <details><summary>Answer</summary>Because a breach is measured against the duty.</details>
2. Name the four elements of negligence.
   ||duty, breach, causation and damage||
```

## The evidence, and what the engine must do

| practice | evidence | here |
|---|---|---|
| practice testing, retrieval | Dunlosky et al. 2013: HIGH utility. IES guide rec 5b: STRONG. Roediger and Karpicke 2006: tested beats restudied at a delay | `retrieval-prompts` |
| generate before you see the answer | Kornell, Hays and Bjork 2009: even failed retrieval attempts enhance learning | `answers-hidden`, `ask-before-tell` |
| deep explanatory questions, elaborative interrogation, self-explanation | IES rec 7: STRONG. Dunlosky: MODERATE | `deep-questions` |
| abstract joined to concrete | IES rec 4: MODERATE | `concrete-examples` |
| interleaved practice | Dunlosky: MODERATE. Brunmair and Richter 2019: g = 0.42, strongest for similar categories, none for expository text or words | `interleaving`, not for language drills |
| worked examples paired with problems | IES rec 2: MODERATE. Atkinson et al. 2000; Sweller and Cooper 1985 | `worked-example-steps` |
| low-utility techniques | Dunlosky: rereading, highlighting, summarising, imagery and the keyword mnemonic LOW | `study-advice` for cramming, rereading and highlighting |
| key terms before the lesson | Mayer's pre-training principle; cognitive load | `key-terms-defined` |
| i+1 comprehensible input | Krashen; Hu and Nation 2000 (98%); Laufer and Ravenhorst-Kalovski 2010 (95% minimum, 98% optimal) | language-mentors' `i1-glosses`, `i1-density` |
| feedback at the task, process and self-regulation levels, never the self | Hattie and Timperley 2007; Kluger and DeNisi 1996; Shute 2008: no normative comparison | `feedback-task-focused` |
| feed forward, where to next | Hattie and Timperley 2007 | `feedback-next-step` |

What no text check can see, and the engine does:

- **Spacing.** Distributed practice is rated HIGH (Dunlosky; Cepeda et al. 2006), and it is the
  scheduler's job. FSRS schedules the reviews; keep desired retention inside the range FSRS's
  optimiser searches, 0.70 to 0.95, with its default at 0.9, and never schedule a reading's
  retrieval set as a same-day repeat.
- **Fading and expertise reversal.** Worked examples help novices and hinder experts. Fade the
  worked steps as a topic's drill grades rise, from the learner model the engine already reads.
- **Feedback after retrieval.** The drill is graded in the conversation, after the learner
  answers; the grade and the correct answer go back in the same turn.
- **Recall over recognition.** Prefer free recall and cloze prompts to multiple choice for
  retrieval; a one-in-four guess is not recall.
- **The keyword mnemonic.** Dunlosky rates it low for durability, and the owner's leech doctor
  still gives a mnemonic for one hard card. That is a targeted remedy, not a study method, and
  `study-advice` does not refuse it.

## The Python API

Load the script with `importlib.util.spec_from_file_location`, exactly as it loads persona-core.

- `load_pedagogy(path=None) -> dict` reads the table, beside the script when None. It raises
  `PedagogyError`, a `ValueError`, naming every problem.
- `CLASSES` is the tuple of class ids; `genre_for(duty, pedagogy=None)` returns a genre or None.
- `applies(class_id, doc, pedagogy=None) -> bool` says whether a class judges a document.
- `findings(class_id, doc, pedagogy=None) -> list[str]` returns `<path>:<line>: <finding>`
  strings, or `[]` when green or not applicable. `doc` is persona-core's `Document`, or any
  object with `path`, `frontmatter` (`duty`, `subject`), `sections` (`id`, `title`, `line`,
  `body`), `body` and `body_line`.
- `evaluate(class_id, root, subjects=(), kinds=(), duties=(), pedagogy_path=None)` returns an
  `Outcome(examined, findings)`.

## How a project applies this

1. **Vendor the two probes.** Copy `scripts/persona-core-probe.py` and
   `scripts/learning-science-probe.py` into the same directory, with persona-core's pack
   directory and this pack's `pedagogy.json` where their defaults look, or pass
   `--pedagogy FILE`.
2. **Instruct the engine.** Each duty's instructions tell the model the sections above: end every
   daily reading with a `retrieval` set that asks at least one why or how question, hide every
   answer, open drills with a prompt, and close writing feedback with a `next-step`.
3. **Gate the delivery.** Run every block class on each generated output before it is sent:
   `python3 scripts/learning-science-probe.py --root OUT check retrieval-prompts`. A red block
   row withholds the text; an advisory row is logged.
4. **Run the pack in CI.** `phxd pack probe` with `--pack learning-science` over a directory of
   golden outputs keeps the prompts honest as they change.

## References

The access date of every source and the Context7 answers are recorded in SPEC-V2-2224 §8.

- Dunlosky, Rawson, Marsh, Nathan and Willingham 2013, Psychological Science in the Public
  Interest 14(1): https://journals.sagepub.com/doi/abs/10.1177/1529100612453266
- Pashler et al. 2007, IES practice guide NCER 2007-2004:
  https://ies.ed.gov/ncee/wwc/Docs/PracticeGuide/20072004.pdf
- Roediger and Karpicke 2006: https://journals.sagepub.com/doi/10.1111/j.1467-9280.2006.01693.x
- Kornell, Hays and Bjork 2009: https://pubmed.ncbi.nlm.nih.gov/19586265/
- Cepeda, Pashler, Vul, Wixted and Rohrer 2006: https://escholarship.org/uc/item/3rr6q10c
- Brunmair and Richter 2019: https://www.uni-wuerzburg.de/fileadmin/06020400/2019/Brunmair_Richter_in_press__2019_META-ANALYSIS_OF_INTERLEAVED_LEARNING.pdf
- Atkinson, Derry, Renkl and Wortham 2000: https://journals.sagepub.com/doi/10.3102/00346543070002181
- Hattie and Timperley 2007: https://journals.sagepub.com/doi/10.3102/003465430298487
- Shute 2008: https://journals.sagepub.com/doi/10.3102/0034654307313795
- Hu and Nation 2000: https://www.wgtn.ac.nz/lals/resources/paul-nations-resources/paul-nations-publications/publications/documents/2000-Hu-Density-and-comprehension.pdf
- Laufer and Ravenhorst-Kalovski 2010: https://files.eric.ed.gov/fulltext/EJ887873.pdf
- Jurenka et al. 2024, LearnLM: https://arxiv.org/pdf/2407.12687
- Telegram Bot API formatting (spoilers): https://core.telegram.org/bots/api
- Context7 ids: `/open-spaced-repetition/fsrs-rs`, `/open-spaced-repetition/ts-fsrs`,
  `/websites/core_telegram_bots_api`, `/python/cpython`
