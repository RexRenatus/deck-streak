### Fixed

- The engine core's and the native adapter's scheduler tests take their time from one injected
  clock, with each test collection's next day cutoff pinned eleven to twelve hours ahead, so they
  no longer fail when a run falls in the minutes before the day's cutoff (#747).
