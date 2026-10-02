### Fixed

- A hand-proved mutation row whose mutant is a shell script that does not parse now reads VOID,
  never KILLED: the runner parse-checks the mutant with `bash -n` for a bash script and `sh -n`
  otherwise, before it runs the row's killer.
