### Fixed

- The pre-push dynamic-imports check now agrees with the test-module census: a module that binds
  `exec` by any binding the census reads is no longer refused, a register site added by a `|=` of
  `allowed` groups is read and any other write to the register stops the run, a file that is not
  UTF-8 text reads VOID instead of a traceback, and every object is read from the one commit
  `HEAD` names when the check starts (SPEC-612, ADR-634, #804).
