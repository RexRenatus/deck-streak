### Fixed

- Each planted card test on iPhone and iPad now holds one loaded page across the views it builds,
  so no view's load waits on a relaunch of WebKit's GPU process, which had ended a reference
  view's page before it loaded (SPEC-689).
