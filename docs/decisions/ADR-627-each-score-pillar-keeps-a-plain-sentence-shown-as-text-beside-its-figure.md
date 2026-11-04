---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-627: Each score pillar keeps a plain sentence, shown as text beside its figure

Decides SPEC-606 (issue #800).

## Context and Problem Statement

The score screen shows five pillars, each as a bare score from 0 to 100, and none of them says what it measures. Every citation below was read at commit `b065fce`.

- The breakdown renders each pillar as one list item: its name, its value or the gap line, and a meter named by `aria-labelledby` (`web/app/src/lib/score/ScoreBreakdown.svelte:24-44`).
- The five pillars are five functions of different inputs (`crates/analytics/src/score.rs:109-176`), each with its own constants (`crates/analytics/src/constants.rs:13-45`).
- The Mastery pillar counts graduations, review answers whose interval crosses into 21 days or more, against a target of 3, less 3 points per active leech, at most 30 (`score.rs:167-176`, `crates/analytics/src/metrics.rs:122-127`). It shares its name with the mastery percentages of Road to C2 and the law block, which SPEC-408 describes, while it counts something else. ADR-422 D7 left it for this follow-up.
- The breakdown's texts are in `web/app/messages/en.json:14-22` only: none of the six other locale files carries them.

## Decision Drivers

- The sentence is true to the computation, and claims nothing the code does not compute.
- A learner reads the meaning where the figure is, by touch, keyboard or screen reader, with no extra act.
- The Mastery pillar is told apart from the mastery percentages in so many words.
- Every shipped locale reads the breakdown in its own language.
- Each criterion is pinned by a test that is red before the change.

## Decision Outcome

### D1. One sentence per pillar, as ordinary text, last in its row

Each pillar gets its own message key, `pillar_<pillar>_about`, and its sentence is a paragraph `<p id="pillar-{pillar}-about">`, the last element child of the pillar's list item: after the bar, or after the gap line when the retention is absent. The component maps each pillar to its sentence by a `Record<Pillar, () => string>` beside its map of names.

### D2. What each sentence says

Each sentence says what its pillar measures and states the figures that decide it:

- Consistency: whether the learner studied today and the streak's length; 70 for studying, up to 30 more as the streak grows to 25 days.
- Retention: the share of review cards recalled on their first answer today; 0 at 50% or less, 100 at 95% or more, and halfway toward 70 with fewer than 5 answers.
- Workload: the share of today's due review cards cleared, less 1 point for every 5 overdue cards, at most 40.
- Volume: today's reviews and minutes against the usual day; 80 for a usual day, 100 for a quarter more.
- Mastery: it opens by saying it is not a mastery percentage, then states 3 cards reaching an interval of 21 days or more for 100, and 3 points off per active leech, at most 30.

### D3. Every stated figure is pinned to the shipped function

A4 reads the English sentences from `en.json`, takes their digit runs, and compares them with the figures `deck_streak_analytics::score` computes from fixed inputs, 19 figures in all. A change to a pillar's function or constant that moves a stated figure fails A4.

### D4. Every text the breakdown reads is translated

The 13 texts the breakdown reads (the title, the list's name, the five labels, the gap line and the five sentences) are written in English and in all six other shipped locales (es, fr, ja, ko, zh-Hans, zh-Hant). In `en.json` the five sentences go directly after `retention_absent`; in each other file the 13 texts go directly after `tagline`. A translation keeps the English figures as the same digit runs, and writes a word where the English writes a word ("a quarter more").

### D5. The meter names its sentence

Each pillar's meter carries `aria-describedby="pillar-{pillar}-about"` beside its `aria-labelledby`, so a screen reader that lands on the bar hears its description too. The sentence stays visible text, so sighted readers see it without the attribute.

### D6. The sentence adds no colour

The sentence carries spacing and size classes only (`mt-1 text-sm`), with no colour class and no inline style. It keeps the surrounding text's colour, so it adds no new colour pair for the rendered contrast audit to judge.

### D7. The two existing breakdown tests' literals grow by the sentence

`shows five pillars and an absent retention as a gap` and `shows a retention the day has as its value and its bar` pin each row's whole text. Each such literal gains the row's sentence at its end. The gap row's "no digit" assertion moves from the whole row, which now holds the sentence's figures, to the row's value text, and the exact whole-row literal still covers the row. No assertion is removed.

## The alternatives each decision was chosen against

### D1. One sentence per pillar

- Chosen: one sentence per pillar, because the five pillars are five computations, and a sentence true of one is false of the others.
- Chosen against one sentence for the whole breakdown: it would describe the total, which is the weighted sum, and say nothing of what each figure measures.
- Chosen against a hover tooltip: content shown on hover alone fails touch and keyboard users, and fails the content-on-hover-or-focus criterion.
- Chosen against a disclosure the learner opens: it hides the meaning behind one more act on every visit, and each sentence is short enough to show.
- Chosen against placing the sentence between the name and the bar: it would split the figure from its bar, which read as one.

### D2. What each sentence says

- Chosen: what the pillar measures and the figures that decide it, because a learner reading a bare 76 needs to know both what moves it and how far.
- Chosen against naming the pillar's weight in the total: the weight explains the total, not the pillar's own figure, and the screen shows each pillar's own figure.
- Chosen against a formula: the learner needs a sentence, and the constants behind it are pinned by A4.
- Chosen against a Mastery sentence that only describes graduations: a learner who knows the mastery percentage would still read the two as one measure, so the sentence says first that it is not one.
- Chosen against the bare word "leech" in the Mastery sentence: it is a term of the review app, and the short gloss "a card you keep forgetting" names it in plain words.

### D3. The figure pin

- Chosen: a test that computes every stated figure from the shipped functions, because a typed list of expected figures would drift from the code with the sentence.
- Chosen against a pin of the constants alone: a figure such as 25 days or 80 for a usual day is a result of several constants and a formula, not one constant.
- Chosen against reading the expected figures from the message files in the web tests: a test would then take its expected value from the data under test, so A1 spells each English sentence literally.

### D4. Locales

- Chosen: all seven locales written by this delivery, because a key missing from one locale falls back to English beside a translated screen.
- Chosen against translating the five sentences alone: the labels and the title exist only in English, so a reader of another locale would meet English labels beside translated sentences.
- Chosen against appending the keys at each file's end: every delivery that appends at the end collides there, and a key beside its siblings sits where a reader of the file looks.
- Chosen against writing "a quarter more" as "25%": the English states no such figure, and the figure census would then read a figure the English lacks.

### D5. The meter's description

- Chosen: `aria-describedby` on the meter beside the visible text, because a screen-reader user who moves by control reaches the bar and hears what it measures.
- Chosen against `aria-describedby` alone with no visible text: sighted users would see nothing.
- Chosen against folding the sentence into the meter's name: the name is the pillar's label, which other tests and the reader rely on, and a sentence is a description.

### D6. Colour

- Chosen: no colour class, because the sentence then shares the surrounding text's colour and needs no new contrast pair.
- Chosen against a muted colour for the sentence: a lighter tone adds a colour pair for the contrast audit to judge, and the score page's colours are another change's.

### D7. The existing tests

- Chosen: grow the two literals by the sentence, because each test pins the row's whole text, and an exact literal that holds the sentence is stronger than one that drops it.
- Chosen against keeping the old literals by reading only the row's first line: it would stop pinning what the row shows, and a sentence placed in the wrong row would pass.
- Chosen against dropping the gap row's "no digit" assertion: it moves to the value text it was written for, so the gap still shows no 0.

## Consequences

- A learner who sees a pillar on the score screen reads, beside it, what that pillar measures, in every shipped locale.
- The Mastery pillar says in so many words that it is not a mastery percentage.
- Each pillar's row grows by one paragraph of body text.
- `en.json` grows by 5 lines and each of the six other message files by 13 lines.
- A later change to a pillar's function or constant must revisit its sentence: A4 fails when a stated figure moves.

## What would make this wrong

- A pillar's computation changes: A4 fails, and its sentence must change with it.
- A native client gains a score screen: the sentences go there too.
- Readers find a sentence unclear in review: the wording changes. The keys and the placement stay.
- The score screen stops showing a pillar: its sentence and key go with it.
