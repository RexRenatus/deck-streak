---
status: "proposed"
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-667: the scripted form card tries its form five times, as its navigation sibling does, and no reading is retried

Decides SPEC-644 (R1 to R3).

## Context and Problem Statement

SPEC-355's scripted-view test read the `script-form` card's reference blind once, on the iPad simulator (job 115407471652,
#843). In that session the reference loaded, its script set its marker, and its one form navigation was allowed;
WebKit then failed that navigation's provisional load as offline (`NSURLErrorDomain` -1009) before anything
reached the listener, and the card never tried again, because it submits its form once while its sibling
`script-nav` assigns its location up to five times. Across the 209 `apple / harness` jobs read from run
38355861156 to run 42416540106, that is the one session whose `script-form` reference reached nothing. The
offline verdict came from the simulator's network, which the tree does not control, so the red can recur.

## Decision Drivers

- A reference that a try failing before it is sent leaves blind is cured at its cause, the card's single try.
- Nothing is weakened: no reading, wait, deadline, window, pinned set or assertion changes, and nothing is
  retried, skipped or admitted as an expected failure.
- The scripted view must still reach nothing, from every try the card makes.
- The cure is proved red first: a planted refusal of the first try reads red with one try and green with five.

## Considered Options (the alternatives it was chosen against)

- **Chosen: the card's own tries,** because `script-form` then submits its form up to five times on the loop
  `script-nav` already uses, so one try that fails before it is sent leaves its reference another that reaches.
- A retry of a reference reading that reads blind: rejected because a test-level retry is a weakening and would
  admit any blind reading, whatever its cause.
- A longer reference `deadline`: rejected because a try that failed at once is not sent again by waiting, and
  every pinned blind reference would wait longer in every session.
- Adding `script-form` to `BLIND_SCRIPTED`: rejected because it would hide the channel the reference must prove.
- Recording why the red cannot recur: rejected because the offline verdict came from the simulator's network,
  which the tree does not control, so the red can recur.

## Decision Outcome

Chosen: the card's own tries, because they remove the cause the job read and change no reading, wait or
assertion:

- D1: `script-form` tries its form up to five times on `script-nav`'s loop
  (`let tries = 0; const go = () => { ...; if (tries < 5) { setTimeout(go, 200); } }; setTimeout(go, 0);`), every
  try the same POST to `/script-form/1`; its id, held set, `alone`, observable, method and action stay as they
  are.
- D2: the plant, a refused first try in the reference's recorder (`Recorder.refusesFirstTry`, counted in
  `Recorder.refusedTries`), stands for a try that fails before it is sent; it refuses one navigation, clears
  itself, and only A1 sets it.
- D3: no reading, wait, deadline, window, pinned set or assertion changes, and the scripted view, every control
  on, and each view with one control off must reach nothing from any try.

### Consequences

- Good: one try that fails before it is sent no longer blinds the form's reference, on either simulator.
- Good: five tries are a stronger attack on the scripted view than one, so the scripted view's zero means more, never less.
- Bad: an offline verdict that outlasts all five tries still reads the reference blind, a red by name.
- Neutral: the plant matches text, so it refuses only a navigation whose request URL holds `/script-form/`.

### Confirmation

SPEC-644 A1, red at the commit that carries the plant and its test with the card unchanged, and green at the
commit that adds the five tries, with the test unchanged.

## More Information

- SPEC-644; SPEC-355 R9 and its scripted-view criterion, SPEC-349 R11 and SPEC-609, kept with every bound they name.
- What would make this wrong: a later blind `script-form` reference whose log holds no failed provisional load of
  the form's navigation, which would name another cause; or a scripted view reading that reaches the listener from
  a later try, which would mean a control holds only the first.
