### Fixed

- Two recomputes that overlap settle each closed study day once, in turn (SPEC-071, issue #311).
  The scheduled cycle and the owner's recompute could both read the settle cursor before either
  settled, and both settle the same closed day. Each owed day's write now reads the cursor again
  inside its own transaction and settles only the day that cursor owes, the day after it or, with
  none yet, the run's own day; a write for any other day commits nothing and the run goes on from
  the owed day (ADR-313). SPEC-071 also states which days before the first settled day have a row:
  a study day of the window or a recompute's current day, and no other.

### Added

- The scheduled sync's spread test counts what it judged: its judges record each member they are
  handed, and the spread is refused unless it hands 3,600 distinct members, so a generator that
  repeats an offset, an hour or a day reads red (SPEC-027, issue #462).
