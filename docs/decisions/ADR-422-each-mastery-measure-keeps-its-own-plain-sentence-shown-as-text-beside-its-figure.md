---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-422: Each mastery measure keeps its own plain sentence, shown as text beside its figure

Decides SPEC-408 (issue #799).

## Context and Problem Statement

Two different figures are shown to the learner as "mastery" percentages, and neither says what it measures. Every citation below was read at commit `a68db18a`.

- **Course mastery** (Road to C2, and the bot's progress reply) is computed in `crates/curriculum/src/progress.rs`. `card_mastery` (:85-112) gives each counted card a value from 0 to 1. A suspended card (queue -1) is 0. A card with a memory state is `min(1, ln(1+S)/ln(1+100)) × R`: its stability `S` scaled against 100 days, times its recall chance `R` now. A card with no memory state is 1 if it is a review card with an interval of at least 21 days, else 0. `course_progress` (:141-236) averages the counted cards' values over each band and over the course, times 100. SPEC-077 R2 to R4 state it, and the parity goldens `card_mastery.json` and `course_progress.json` hold it.
- **Law mastery** (the law block) is computed in `crates/curriculum/src/law.rs:10-19`: `mastery_pillar(n)` is 100 less 3 points per active law leech, the penalty capped at 30, so its floor is 70. SPEC-077 R9 states it, the golden `law_mastery_pillar.json` holds it, and coordination shows the line only when the leech count is not 0 and the figure is above 0 (`crates/coordination/src/law/mod.rs:79-86`).
- The surfaces: the web law block renders `m.law_mastery({ pct })` at `web/app/src/lib/law/LawBlock.svelte:30`, inside `<li data-line={key}>` at :48. Road to C2 renders `m.progress_summary` at `web/app/src/lib/progress/CourseLadder.svelte:18`, and each band cell's `progress_band_mastery` and meter. The bot's `course_line` formats `{}% mastery` at `crates/bot/src/progress_commands.rs:59-70`. None of them says what the figure measures.
- The score screen's Mastery pillar (`web/app/src/lib/score/ScoreBreakdown.svelte:10-16`, :32) is a third computation (`crates/analytics/src/score.rs:167-176`: graduations against a target of 3, less a leech penalty). It is shown as a bare score from 0 to 100 beside four other pillars, not as a percentage.
- The native clients show no mastery figure: no file under `ios/` names mastery.

## Decision Drivers

- The sentence is true to the computation, and claims nothing the code does not compute.
- A learner reads the meaning where the figure is, by touch, keyboard or screen reader, with no extra act.
- The two measures never share a sentence, because they are different computations.
- Every shipped locale reads the sentence in its own language.
- Each criterion is pinned by a test that is red before the change.

## Decision Outcome

### D1. What each percentage measures, in plain words

Each measure gets its own sentence and its own message key, and no surface shows the other measure's sentence.

- Course mastery, key `progress_mastery_about`: "Mastery is an estimate from your reviews: the average, over the cards it counts, of how likely you are to recall each card now, with cards not yet firmly learned counted for less and new or suspended cards counted as 0."
- Law mastery, key `law_mastery_about`: "Law mastery starts at 100% and drops 3 points for each active law leech, a law card you keep forgetting, never below 70%; it does not measure how much law you know."

The course sentence holds on both branches of `card_mastery`. A card with a short stability counts for less than its recall chance. A young card with no memory state counts 0. A new or suspended card counts 0. Its only figure is 0. The law sentence's figures, 100, 3 and 70, are `mastery_pillar`'s start, its step per leech and its floor.

### D2. Where the description shows on each surface

- **Law block:** visible text inside the mastery line's own list item, after the figure, as `<span class="block text-sm" data-mastery-about="law">`. The pending line carries none, because it shows no figure.
- **Road to C2:** a paragraph `<p class="text-sm" data-mastery-about="course">` inside each course's section, directly after its `data-course-summary` paragraph, once per course. Reading order carries it to a screen reader, after the heading and the summary.
- **Bot:** the progress reply, when it lists at least one course, ends with one line, the course sentence in italics (`<i>…</i>`). The sentence passes through the transport's `escape_html` like every text that enters the reply's markup. It holds no `<`, `>` or `&`. The reply stays within the transport's message limit (`MAX_TEXT_UTF16`), which the transport also chunks. The no-course and failed replies are unchanged.
- Neither web element sets a colour of its own. It keeps the surrounding text's colour, so it adds no new colour pair for the rendered contrast audit to judge.

### D3. Clients in scope

The web Mini App and the bot change now. The native clients show no mastery figure, so nothing changes there. The agent read tool's text and the data export's field are machine-read data, and they stay as they are.

### D4. Locales

Both keys are written in English and in all six other shipped locales (es, fr, ja, ko, zh-Hans, zh-Hant) by this delivery. `progress_mastery_about` goes directly after `progress_band_meter`, and `law_mastery_about` directly after `law_mastery`, in every message file. Each translation keeps the English figures and the locale file's own terms for mastery, law and leech. The bot's replies are English text, so it carries the English course sentence, equal to `en.json`'s.

### D5. The tests, the rows and FORMAL

- One red-first test per surface (A1 law block, A2 Road to C2, A4 bot), a locale census (A3), and a pin of the law sentence's figures to `mastery_pillar` (A5). Each test spells the English sentence literally.
- Hand rows S40801 to S40803 on the bot's new line and its constant. StrykerJS judges the two Svelte components whole, at break 100.
- FORMAL is not applicable by surface. The delivery adds display text: no actor, no shared state and no write path. 0 of the 239 `@phx covers` lines name a file it edits.

### D6. Shape

One delivery under SPEC-408 and ADR-422.

### D7. The score screen's Mastery pillar keeps its present form

The score pillar is not a percentage, and this delivery does not describe it.

## The alternatives each decision was chosen against

### D1. What each percentage measures

- Chosen: two sentences, one per measure, because the course figure and the law figure are computed by different functions from different inputs.
- Chosen against one sentence shared by both figures: it would be false for one of them.
- Chosen against naming the memory model's terms (stability, retrievability): they are jargon a learner does not need to read the figure.
- Chosen against "how well you remember", or any claim about the learner's mind or ability: the figure is computed from review records, and the sentence says no more than that.
- Chosen against stating the recall chance as a fact rather than an estimate: it is a model's output, and the house rule names a model's predicted recall an estimate.
- Chosen against spelling out the no-memory-state branch and the 100-day scale: such detail is false for some cards on one branch, and the learner needs one sentence, not a formula.
- Chosen against a law sentence that presents the figure as law knowledge: the pillar counts only active leeches, so the sentence says plainly that it does not measure knowledge.

### D2. Where the description shows

- Chosen: visible text beside each figure, because the learner reads the figure and its meaning in one place, with no extra act.
- Chosen against a hover tooltip: content shown on hover alone fails touch and keyboard users, and fails the content-on-hover-or-focus criterion.
- Chosen against a disclosure the learner opens: it hides the meaning behind one more act on every visit, and the sentence is short enough to show.
- Chosen against `aria-describedby` on the figure alone: it reaches screen-reader users only, and sighted users see nothing.
- Chosen against one description per page: a learner who jumps to one course's heading would skip a sentence placed once at the page's top.
- Chosen against repeating the sentence in each band cell: six copies per course add noise, and the cells show the same measure as the summary the sentence follows.
- Chosen against a separate bot help command: the learner would have to know it exists, while a line in the reply sits where the figure is.
- Chosen against a sentence on the law block's pending line: that line shows no figure, so there is nothing to describe.

### D3. Clients in scope

- Chosen: the web Mini App and the bot, because they are the clients that show a mastery percentage at `a68db18a`.
- Chosen against changing the native clients now: no native view shows a mastery figure, so there is nothing there to describe.
- Chosen against changing the agent read tool's text: it is pinned by its own golden, and its own issue owns it (#157).
- Chosen against describing the data export's field: it is machine-readable data, not a figure shown to a reader.

### D4. Locales

- Chosen: all seven locales written by this delivery, because a key missing from one locale falls back to English beside a translated figure.
- Chosen against English only, with the other locales falling back: a reader of another locale would meet an English sentence in a translated screen.
- Chosen against appending the keys at each file's end: every delivery that appends at the end collides there, and a key beside its siblings sits where a reader of the file looks.
- Chosen against a second English wording for the bot: two wordings of one measure drift apart, so the bot's sentence is pinned equal to `en.json`'s.

### D5. The tests, the rows and FORMAL

- Chosen: one red-first test per surface, a locale census and a figure pin, because each surface can lose its sentence on its own, any locale can lose a key, and the law sentence's figures can drift from the pillar.
- Chosen against whole-page snapshot tests: they pin unrelated text and fail for reasons outside the criterion.
- Chosen against reading the expected sentence from the message files in the surface tests: the test would then take its expected value from the code under test, so each test spells the English literally.
- Chosen against a figure pin for the course sentence: its one figure is the 0 that new and suspended cards count, which needs no tuning constant to pin.
- Chosen: hand rows on the bot's new line and constant, because the mutation tool does not mutate a constant's literal value and the line is new code.
- Chosen against hand rows on the Svelte components: web code with no row table is judged whole by StrykerJS at break 100.
- Chosen: FORMAL not applicable, because no actor, shared state or write path is added.
- Chosen against a proof that the law sentence's figures equal the pillar's: A5 checks them against the shipped function itself.

### D6. Shape

- Chosen: one delivery under SPEC-408 and ADR-422, because one requirement spans two contexts' surfaces and seven locales, and a new SPEC states it whole.
- Chosen against an insert-only amendment of SPEC-077: that SPEC owns the surfaces but has landed and has been amended, and an added requirement across two surfaces and seven locales would be a large insert into a finished record.

### D7. The score screen's Mastery pillar

- Chosen: the score pillar keeps its present form, because it is not a percentage: it is one of five pillars, shown as a score from 0 to 100, computed from graduations and leeches.
- Chosen against describing that one pillar alone: one described pillar among four bare ones would be inconsistent, and describing the pillars is a change of its own.

## Consequences

- A learner who sees a mastery figure in the Mini App or the bot reads, beside it, what that figure measures.
- The law sentence is first seen when the law figure leaves pending, which waits on the leech port (#133). At `a68db18a`, `crates/api/src/law_routes.rs:52` passes no leech count, so the figure is pending in production.
- Each new key adds one line to each of the seven message files, beside its siblings.
- The bot's progress reply grows by one line.
- A later change to `card_mastery` or `mastery_pillar` must revisit its sentence. A5 fails on any change to the pillar's figures. No test catches a change to the course computation that leaves the sentence false.

## What would make this wrong

- `card_mastery` stops counting suspended or new cards as 0, or stops scaling by stability: the course sentence becomes false.
- `mastery_pillar` changes its start, step or floor: A5 fails, and the law sentence must change with it.
- A native client gains a mastery figure: D3 must be revisited, and the sentence goes there too.
- Readers find "estimate" or "counted for less" unclear in review: the wording changes. The keys and the placement stay.
