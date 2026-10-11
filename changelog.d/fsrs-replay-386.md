### Added

- The engine core replays a deck set's review history through the FSRS-7 scheduler crate into
  stock-field values (SPEC-386, ADR-400; #641). One fixed statement reads the review rows of the
  cards whose home deck is in the set; the crate cuts each card's history at its last reset, drops
  manual, rescheduled and unrated rows, and replays the rest under the preset's parameters; each
  card's stability is projected to the stock 90-percent interval, its difficulty is held to 1 to 10,
  and a review card gets an interval and a due on the engine day its caller read. Nothing is written:
  the write is the scheduler switch (#611), and a test alone proves the mapping survives a sync. One
  engine-core file names the crate, and a Lean proof with its vectors holds the history selection.
