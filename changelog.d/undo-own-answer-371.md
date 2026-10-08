### Changed

- Undo reverts only the review's own last answer, and asks first (SPEC-371, ADR-382). The engine's
  Undo is an owner's exempt write on the answered card, and the engine core undoes only while the
  review's recorded answer is still the front of the engine's undo queue, its review row is still
  there, of that card, and has not synced; a later change, such as a bury, refuses it, and the
  answer stands. An undo press asks for the offer and writes nothing: a dialog names the card as
  one line of text, the answer and the state the card returns to, with focus on "Keep it", and
  only its confirmation undoes. A synced answer is not offered, and a refused undo reads as
  `undo-synced` or `not-undoable`. The native app's allow-list no longer runs Undo.
