### Changed

- The browser tests of the web engine, its sync and the study suite now run in Firefox as a third
  engine beside Chromium and WebKit, in the existing `web-engine` job, which installs Firefox on its
  existing install line; each engine project syncs as a server user of its own (#764; SPEC-613)
