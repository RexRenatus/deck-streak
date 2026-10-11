### Added

- A deck the learner keeps away from AI reaches no AI duty (SPEC-381, ADR-392). Every deck starts
  readable. A mark is one row per deck on the server, so it holds from the moment it is saved, on
  every client, a client that shows no setting included.
- One rule decides every card: a card is kept away when its home deck or the deck it sits in now,
  or an ancestor of either by name, is marked. A deck the tree cannot resolve, or a set of marks
  that cannot be read, keeps the card away.
- The day set holds back every kept-away card and logs only how many it held back. A failed read
  of the marks ends the day's resolution with its named error and records no day.
- Every AI duty asks the deck gate before its input gate, its prompt and its runner. A refusal ends
  the run withheld, class `deck-sensitive` or `deck-unreadable`, with counts only, and a census
  holds every AI call site to the gate.
- Two owner routes, `GET /api/decks/sensitive` and `PUT /api/decks/{id}/sensitive`; the change
  stands behind the same-origin state-change check.
- A screen, "AI and your decks", reached from the deck list: one switch per deck, off unless the
  deck is marked. A deck under a marked deck shows on and names the deck it follows. Its words are
  in all seven languages.
- The marks are exported with the learner's data and erased with the account.
- A TLA+ model of the mark, the day set's selection and the gate, and a Lean proof of the rule.
