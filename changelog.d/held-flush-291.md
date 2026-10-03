### Fixed

- Held notifications are now flushed by a scheduled step of their own after the quiet window ends,
  not only after a sync (#291). A flush never sends one held item twice across two flushers.
