### Changed

- The engine's refusal of an answer on a card that changed after it was shown is now held at the
  core's answer door: two tests record an answer on the states a card was shown with, once after
  another answer of the card and once after the owner set its due date, and assert the engine's
  refusal and an unchanged collection; two mutation rows show each test failing when the core sends
  the engine's own state in place of the shown one or reads the refusal as a recorded answer; and a
  TLA+ model holds that a recorded answer carries the state its card was shown with and that no
  answer is recorded on a changed card (SPEC-460, #760).
