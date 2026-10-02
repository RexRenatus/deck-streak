### Added

- The open lapse walk is proved in Lean over the day type it walks (#472). The entry
  `lean/OpenLapse` proves that `open_lapse` takes at most one step per day from the window's first
  day to today, never overflows at the smallest day, and answers as the rule does, with the one
  exception at the smallest day; each theorem has a witness that the formal check catches. Vectors
  the port writes hold it to the Rust function, and SPEC-076 records the proof and section 24's
  count of its amendment's insertions (#513).
