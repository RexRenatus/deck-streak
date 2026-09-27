---
schema: "phx.persona.rules.v1"
x-duty: "practice-questions"
---

# Duty: practice questions

## What it produces <!-- section:produces -->

A practice set with an explanation for every question and every choice.

## Its sections <!-- section:sections -->

- `questions`: the items, and nothing that reveals their answers.
- `explanations`: one block per item.
- The persona's own sections: a law professor adds `facts` and `issues` for an issue-spotter; the
  LSAT coach adds its timing.

## The rules <!-- section:rules -->

- A multiple-choice item uses the LSAT coach's format, `phx.lsat.coach.v1`, whatever the subject.
  Its heading carries the key in an invisible marker, such as
  `### Question 1 <!-- lsat-item {"id": "q1", "key": "B"} -->`, and one line per choice follows,
  from `(A)`.
- Its explanation block carries the same marker id, and one entry per choice:
  `(B) credited: <reason>` for the key, and `(A) incorrect: <reason>` for each distractor. Exactly one
  choice is credited, it is the key, and every reason is at least six words.
- One best answer; distractors that a learner who half-knows the rule would pick; choices of about
  the same length; the key at a different letter from item to item; no "all of the above" or "none
  of the above"; a negative word such as EXCEPT capitalized; no always or never in a choice.
- An open question is a numbered list item, with a numbered explanation for each.

## Fail loud <!-- section:fail-loud -->

If the set cannot be written in full, write nothing and report the failure.
