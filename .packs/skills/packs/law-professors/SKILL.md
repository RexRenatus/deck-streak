---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/law-professors

Thirteen law professor TEMPLATES for DeckStreak's teaching personas, and the checks that hold a
professor's output to the law's own standards (SPEC-V2-2214 / ADR-V2-2214). The owner chose
"Professor per subject + LSAT coach". Each professor owns its outline, its rule statements and its
classic traps, and teaches by four law methods:

- rule primers in IRAC;
- Socratic questioning;
- issue-spotter hypotheticals;
- grading with citations to the corpus.

A cited source for law content is blocking, like the owner's other rules for facts, privacy and
safety.

This pack builds to persona-core's contract v1 (`packs/persona-core`) and never re-states it. The
template schema, the output contract, the parser and the shared checks are persona-core's.
`scripts/law-professors-probe.py` loads persona-core's probe and parses every document through it.
This pack adds five things:

- the area registry, `areas.json`;
- the thirteen templates, `templates/law-<area>.persona.md`;
- the law sections each duty adds;
- the citation contract;
- sixteen law classes.

Which seats consume the pack is its catalog row's `consumes`, the one record of that edge.

```
phxd pack probe --pack law-professors --root PATH --format json
```

## The rows

Twenty-four rows, all `tree`-scoped, each under a 120-second wall. `{skills}` is the skills
directory the catalog was read from, so the scripts and the registry always come from this pack,
whatever tree `--root` names.

- The first eight run persona-core's classes scoped to law:
  `python3 {skills}/../scripts/persona-core-probe.py --root {root} --kind law check <class>`.
- The other sixteen run this pack's classes:
  `python3 {skills}/../scripts/law-professors-probe.py --root {root} --areas {skills}/packs/law-professors/areas.json check <class>`.

Stage `schema`: 3 rows (3 blocking, 0 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `law-template-schema` | block | `template-invalid` | a law template breaks persona-core's template schema v1 |
| `law-output-contract` | block | `output-invalid` | a law output breaks persona-core's output contract v1 |
| `law-duty-composition` | block | `duty-sections-missing` | a law output lacks a section its duty or its template's `sections` requires |

Stage `safety`: 4 rows (4 blocking, 0 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `law-no-dates` | block | `date-or-timeline` | a law template or output states a date, a deadline or a countdown |
| `law-scrubber` | block | `scrubber-denied` | a law document carries a denied shape: a secret, an address, a private deck name |
| `law-memory-scope` | block | `memory-out-of-scope` | a professor reads another subject's memory, or the journal |
| `law-no-human-claim` | block | `human-claim` | a professor claims to be human |

Stage `voice`: 1 row (0 blocking, 1 advisory)

| row | severity | reason | reports when |
|---|---|---|---|
| `law-voice` | advisory | `voice-drift` | stage directions, emoji bursts or invented backstory |

Stage `areas`: 2 rows (2 blocking, 0 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `law-area-registry` | block | `area-registry-invalid` | `areas.json` is malformed, does not hold 13 areas, names a format outside its vocabulary, or an area's template is missing, names another subject, or a template in the pack names no area. An unreadable registry is VOID |
| `law-template-area` | block | `template-area-drift` | a law template names an unregistered area, or its outline or assumptions drift from the registry |

Stage `templates`: 1 row (1 blocking, 0 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `law-template-methods` | block | `law-methods-missing` | a template lacks a method duty; a law section in its `sections` map; a law body section (outline, assumptions, traps, questioning, hypotheticals, grading); the four methods in `method`; one of the six `ask:` tags; one of the four NCBE criteria or the Total line; or three traps |

Stage `irac`: 2 rows (1 blocking, 1 advisory)

| row | severity | reason | refuses or reports when |
|---|---|---|---|
| `irac-structure` | block | `irac-incomplete` | a daily reading, or any output with an issue, application or conclusion, lacks one of the four IRAC sections, has them out of order, or states the issue other than as a question (a `?` or "whether") |
| `irac-analysis` | advisory | `irac-thin` | the application shares no fact word with the reading or fact pattern, or the conclusion none with the issue |

Stage `citations`: 4 rows (4 blocking, 0 advisory)

| row | severity | reason | refuses when |
|---|---|---|---|
| `rule-cites-corpus` | block | `rule-uncited` | a rule statement carries no citation |
| `citations-resolve` | block | `citation-unresolved` | there are no `sources`, or they are empty, repeated or not citation keys; a citation is not in `sources`; a source is never cited; there is no corpus; the corpus is malformed or for another subject; or a source is not in it |
| `quotes-grounded` | block | `quote-ungrounded` | a rule statement quotes three or more words that none of its cited sources contains |
| `authority-grounded` | block | `authority-uncited` | a statement asserts a named authority with no citation: a reporter citation, a case name, the U.S. Code, a section sign, a federal rule, a numbered rule, the UCC, a Restatement, a Model Rule or Code, an amendment, or the Constitution |

Stage `socratic`: 2 rows (1 blocking, 1 advisory)

| row | severity | reason | refuses or reports when |
|---|---|---|---|
| `socratic-prompts` | block | `socratic-malformed` | a Socratic turn has no prompt or more than five; a prompt does not end with `?`, has no `ask:` tag, several, or an unknown one; or the section states an answer (`Answer` or `Rule:`) |
| `socratic-coverage` | advisory | `socratic-narrow` | three or more prompts in one category, or no deep category |

Stage `hypos`: 3 rows (2 blocking, 1 advisory)

| row | severity | reason | refuses or reports when |
|---|---|---|---|
| `hypo-structure` | block | `hypo-malformed` | the fact pattern is under 40 words; no call ends in `?`; there is no `###` issue block; or an issue block lacks a quoting `Trigger:` line or a `Rule:` line |
| `hypo-anchored` | block | `issue-unanchored` | a `Trigger:` or `Immaterial:` quote is not in the fact pattern, word for word |
| `hypo-depth` | advisory | `hypo-shallow` | fewer than two issues, or no immaterial fact |

Stage `grading`: 2 rows (1 blocking, 1 advisory)

| row | severity | reason | refuses or reports when |
|---|---|---|---|
| `grading-cites` | block | `grade-uncited` | the rubric lacks one of the four NCBE criteria, names an unknown one, repeats one, or scores out of range; an issues, rules or reasoning line cites nothing; the `Total:` line is missing, doubled or not the sum; or the model answer is missing or uncited |
| `grading-feedforward` | advisory | `feedback-not-actionable` | a next step names no rubric criterion, or is praise alone |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`. It
exits 0 when green, 1 on a finding, 2 on a usage error, and 3 when VOID. VOID means nothing was
examined, or the registry or persona-core's probe could not be read, and it is never a pass.

An output class counts every law output it read, and owes nothing for an output its duty does not
reach, such as a drill for `irac-structure`. A tree with no law output is VOID.

## The thirteen areas

`areas.json` (`phx.law.areas.v1`) is the registry. The areas are the union of the subjects NCBE
names across the MBE, the MEE (its current and its earlier list) and the NextGen UBE, which is
twelve, plus professional responsibility. Professional responsibility is the MPRE's subject, and
NextGen assesses named ABA Model Rules from recall inside its integrated question sets. NCBE's
compound subjects stay one professor each.

| id | professor of | format codes |
|---|---|---|
| `business-associations` | Business Associations | mee, nextgen |
| `civil-procedure` | Civil Procedure | mbe, mee, nextgen |
| `conflict-of-laws` | Conflict of Laws | mee-legacy |
| `constitutional-law` | Constitutional Law | mbe, mee, nextgen |
| `contracts` | Contracts | mbe, mee, nextgen |
| `criminal-law-and-procedure` | Criminal Law and Procedure | mbe, mee, nextgen |
| `evidence` | Evidence | mbe, mee, nextgen |
| `family-law` | Family Law | mee-legacy, mpt, nextgen-resources |
| `professional-responsibility` | Professional Responsibility | mpre, nextgen-integrated |
| `real-property` | Real Property | mbe, mee, nextgen |
| `secured-transactions` | Secured Transactions | mee-legacy |
| `torts` | Torts | mbe, mee, nextgen |
| `trusts-and-estates` | Trusts and Estates | mee-legacy, mpt, nextgen-resources |

The codes carry no dates. `mee-legacy` means a subject on the MEE's earlier list only.
`nextgen-resources` means NextGen tests the subject only with the legal resources provided.
`nextgen-integrated` means inside integrated question sets, from recall. The dated facts behind
the codes (which exam dropped what, and when) are in SPEC-V2-2214 §4.1, and in no template or
output.

Each area records:

- its NCBE names per format;
- its outline, NCBE's top-level categories verbatim;
- its assumptions, NCBE's own governing-law notes and question weights.

`law-template-area` holds each template's outline and assumptions sections equal to them.

**The registry is data.** The DeckStreak architect maps the owner's private decks onto these ids in
private configuration. It fills the roster slots there too. Neither needs a change here.

**Keeping it true.** When NCBE changes a subject list, change the area's `formats` and
`ncbe_names` in `areas.json` by hand, with the dated fact in a SPEC. Nothing here fetches NCBE.
Family law's move among NextGen's foundational subjects is the next such change. When it happens,
its `nextgen-resources` becomes `nextgen`, with its date in the SPEC.

## The four methods, and the sections each duty adds

A duty names WHAT is produced (persona-core's registry), and the professor adds its sections
through the template's `sections` map:

| duty | persona-core's sections | the law sections added | the method |
|---|---|---|---|
| daily-reading | reading | issue, rule, application, conclusion | a rule primer in IRAC |
| conversation-partner | reply | socratic | Socratic questioning |
| practice-questions | questions, explanations | facts, issues | an issue-spotter hypothetical |
| writing-tutor | corrections | rubric, model, next-step | grading that cites the corpus |
| leech-doctor | explanation, mnemonic, contrast | rule, trap | the rule and its classic trap |
| drill-coach | drill | rule | the rules drilled, cited |

Each template's body carries persona-core's seven sections and its four roster slots, `{{name}}`,
`{{bio}}`, `{{voice}}` and `{{personality}}`. It adds six law sections:

- `outline` and `assumptions`, from the registry;
- `traps`: the classic traps, as analysis questions whose answers come from the corpus;
- `questioning`: the six Socratic categories and their tags;
- `hypotheticals`: how to build an issue-spotter, with the area's ingredients;
- `grading`: the five criteria.

## The law output contract

A law output is persona-core's output (`phx.persona.output.v1`) with subject `law/<area>`, and
these additions:

- **`sources` is required.** It is a non-empty list of unique corpus ids, each a Pandoc citation
  key.
- **The body cites with Pandoc's syntax.** A citation is `[@id]`, `[@id, locator]`,
  `[see @id, locator]` or `[@a; @b]`. A key starts with a letter, a digit or `_`, and holds single
  internal punctuation. Every citation is in `sources`, and every source is cited. A link,
  `[@id](url)`, is not a citation.
- **A corpus manifest answers for every output.** It is `corpus.json`, schema `phx.law.corpus.v1`,
  the nearest one above the output, or the file `--corpus` names. It holds:
  - `subject`: the output's subject;
  - `sources`: a list, each with exactly `id`, `title` and `text`. The text is the passage the
    engine gave the model.
- **A rule statement** is every list item or paragraph of a `rule` section, and every line
  labelled `Rule:` in any section. A `Rule:` line is one statement, and a citation on the next line
  does not answer for it. Every rule statement cites. A quote of three or more words in one must
  appear, word for word, in a source it cites. Case, spacing, curly quotes, brackets and a trailing
  mark are ignored, and `...` joins fragments that must appear in order.
- **A named authority is cited in its own statement.** This covers a case, a statute, a rule
  number, a Restatement, a Model Rule, an amendment or the Constitution. The exceptions are the fact
  pattern, the calls of the question, the Socratic prompts, the issue, a daily reading's retrieval
  prompts, and a sentence that ends in `?`.

The formats per method:

- **IRAC.** The sections run in the order issue, rule, application, conclusion. The issue is a
  question: it ends in `?` or asks "whether". The application uses the reading's facts.
- **Socratic.** Each prompt is a list item ending in `?`, then its tag:
  `- <question>? <!-- ask:clarify -->`. The categories are Paul's six: `clarify`, `assumptions`,
  `evidence`, `viewpoints`, `implications` and `question`. A turn asks one to five prompts and
  never states the answer.
- **Issue-spotter.**
  - `facts` is a fact pattern of at least forty words, with fictional parties.
  - `questions` holds the calls, each ending in `?`.
  - `issues` holds one `### <issue>` block per issue. Each block has a `Trigger: "<the fact, word for
    word>"` line and a `Rule: <statement> [@id]` line.
  - An `Immaterial: "<a fact>"` line marks a fact that decides nothing.
- **Grading.**
  - `rubric` has one line per criterion: `` - `facts`: 2/2 <why> ``, and likewise `issues`,
    `rules`, `reasoning` and `writing`.
  - The `issues`, `rules` and `reasoning` lines cite.
  - A `Total: <got>/<max>` line gives the sum.
  - `model` is a model answer that cites.
  - `next-step` holds list items, each naming the criterion it raises. It is the section id
    learning-science reads for feed-forward, one name for one concept across the packs.

`examples/evidence/` holds one worked output per duty and its `corpus.json`, which quotes the
Federal Rules of Evidence word for word. Every class is green on them.

**Composing with the duty packs.** A duty pack names WHAT a duty produces, and its own rules hold
alongside these. study-duties requires three things:

- a law daily reading of 800 to 1500 words of prose, ending with a `retrieval` section of prompts
  that never answer;
- an `x-new-cards` count on the reading;
- `x-leech` on a leech explanation.

The worked examples carry all three. The professor's sections come before the duty's last section,
so IRAC runs issue, rule, application, conclusion, and retrieval follows.

## How DeckStreak's engine uses this pack

1. **Instantiate.** The engine reads `templates/law-<area>.persona.md` for each area the owner
   studies, and fills `{{name}}`, `{{bio}}`, `{{voice}}` and `{{personality}}` from private
   configuration.
2. **Generate.** It builds the prompt in persona-core's order: `rules.md`, the instantiated template,
   the duty's instructions, the subject's memory as data, and the learner's text fenced as
   untrusted. It writes one output per duty in the format above.
3. **Export the corpus.** Beside each output it writes a `corpus.json` holding every passage it gave
   the model, with its id, title and text, and the output's subject.
4. **Check each output before delivery.** It runs both probes on the output's directory AND the
   persona's template. persona-core's `output-contract` resolves the output's persona only among
   the files it examines, so the template must be one of them.
   - persona-core's eight classes:
     `python3 scripts/persona-core-probe.py --root . --subject DIR --subject TEMPLATE --kind law check <class>`
   - this pack's thirteen output classes, from `irac-structure` to `grading-feedforward`:
     `python3 scripts/law-professors-probe.py --root . --subject DIR --subject TEMPLATE check <class>`

   Only the blocking classes decide, and an output any of them refuses is not delivered. The two
   scripts and the two packs' data are vendorable. Keep their relative paths, or pass
   `--persona-core` and `--areas`.
5. **Check the templates in CI.** The three template classes (`law-area-registry`,
   `law-template-area`, `law-template-methods`) and persona-core's `template-schema` judge the
   vendored templates, once per change, through this pack's rows or the same commands with `--root`.
   On a directory with no template, a template class is VOID, so it never runs per output.
6. **What refuses.**
   - A rule stated without a source.
   - A source the corpus does not hold, or a corpus for another subject.
   - A quote the source does not contain.
   - A case or a statute named without a citation.
   - A primer out of IRAC order.
   - A Socratic turn that answers itself.
   - An issue-spotter whose key invents a fact.
   - A grade with a wrong total or an uncited criterion.
   - Anything persona-core refuses: a date, a denied shape, another subject's memory, a human claim.

## References

The dated research record, with access dates and every Context7 id, is SPEC-V2-2214's References.

- NCBE subject outlines: the MBE, MEE and MPRE Subject Matter Outlines, and the NextGen UBE
  Content Scope (https://www.ncbex.org/exams/nextgen/content-scope).
- NCBE, Instructions for Taking the MEE, and The Bar Examiner's grading columns.
- Turner, "Finding Consensus in Legal Writing Discourse Regarding Organizational Structure" (IRAC
  and its variants).
- Paul and Elder, The Thinker's Guide to Socratic Questioning.
- Sullivan et al., Educating Lawyers (Carnegie).
- Dunlosky et al., "Improving Students' Learning With Effective Learning Techniques".
- Hattie and Timperley, "The Power of Feedback".
- Magesh et al., "Hallucination-Free? Assessing the Reliability of Leading AI Legal Research Tools".
- ABA Formal Opinion 512.
- Pandoc's citation syntax (https://pandoc.org/MANUAL.html#citation-syntax).
