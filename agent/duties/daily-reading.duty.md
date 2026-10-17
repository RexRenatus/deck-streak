---
schema: "phx.persona.rules.v1"
x-duty: "daily-reading"
---

# Duty: the daily reading

The engine sends these instructions after the shared rules and the persona's template, and before
the learner's memory. The persona shapes the voice; this duty fixes what the text must hold.

## What it produces <!-- section:produces -->

One pre-study reading for one topic, written from the new cards queued for that topic today. There
is one reading for each topic that has new cards, and no cap on how many topics get one. A topic
with no new cards gets no reading.

## Its sections <!-- section:sections -->

- `reading`: the reading itself, anchored in the text of today's new cards.
- The persona's own sections for this duty, in its template's order: a law professor adds `issue`,
  `rule`, `application` and `conclusion`; a language mentor adds its glosses, grammar spotlight,
  pronunciation notes and culture note.
- `retrieval`: always the LAST section. Two or more prompts, each a list item that ends with a
  question mark or opens with a recall verb, and at least one asks why or how.

## The rules <!-- section:rules -->

- A law reading is a primer of 800 to 1500 prose words. Its length grows with the topic's count of
  new cards inside that band, and the output declares the count as `x-new-cards`.
- A language reading follows its mentor's form: comprehensible input one step above the learner's
  band, with a gloss for every new word. Its length is the mentor's, not the law band.
- Every law rule statement cites the corpus, in the law pack's citation form.
- The retrieval prompts ask the learner to recall, not to recognise. No answer is shown beside a
  prompt; if the engine offers answers, it hides them until the learner asks.

## Fail loud <!-- section:fail-loud -->

If the reading cannot be written in full, write nothing and report the failure. Never write a
stand-in, a partial reading or a note that says the reading is missing.
