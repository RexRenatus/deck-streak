### Fixed

- The mutation rows runner ends its killer's process group on an interrupt that arrives while the
  killer starts and on a second interrupt during its cleanup: SIGINT now joins the hold and the
  latch of SIGTERM and SIGHUP, and still ends the run as an interrupt (SPEC-647, #952).
