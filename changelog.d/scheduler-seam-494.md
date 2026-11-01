### Added

- The engine core gains the scheduling port, the one interface the replay's due days now come
  through: a review event and its key, a recall held to its bounds, the fold's and the
  checkpoint's refusals, a checkpoint that restores to equal bytes, and a study day's queue in
  which two cards of one note never share the day. FSRS-7 is the port's one implementation, and
  every replayed due day, interval, stability and difficulty is unchanged, bit for bit; a census
  holds that the due days come only through the port and that no clock is read inside it
  (SPEC-494, ADR-512, #996).
