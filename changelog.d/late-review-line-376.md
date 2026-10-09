### Added

- The web review says when a review cannot count toward the streak (SPEC-376, ADR-387). When the
  card shown is past its due day, one line above the card, on the question and on the answer side,
  says the card was due on an earlier day, so this review does not count toward the streak for that
  day. A card on time shows no line.
- The engine core decides it in the engine's own day: it reads the scheduler's timing of today
  itself, through no adapter pair, and judges a review or day-learning card by its due day, an
  intraday learning card by the instant the day began, and a card in a filtered deck by the due it
  keeps for its home deck. A new or preview card is never late.
- The line is in all seven locales, each with its own word for the streak.
