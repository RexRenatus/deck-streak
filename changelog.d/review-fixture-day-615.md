### Fixed

- The review fixture the iPhone and iPad tests read now sets its scheduler's rollover hour half a
  day ahead of its writer, measured through the engine and read back, so its next day starts
  about twelve hours after it is written. A ten-minute learning step no longer reads as a day when
  the tests run in the last minutes before the engine's default hour. The interval label stays as
  Anki gives it, and no Swift test or expected interval changes (#779; SPEC-615).
