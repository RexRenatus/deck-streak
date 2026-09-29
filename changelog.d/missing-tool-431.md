### Fixed

- The mutation runner no longer reports a missing tool as a surviving mutant (SPEC-039 section 19,
  ADR-291, issue #431). When a process it must run (`git`, `cargo`, the interpreter, `bash` or
  `sh`) is absent from `PATH`, not executable, or a directory, every verb now ends with one line
  naming the tool and exit 2, never a traceback and never exit 1, and a mutant that was installed
  is restored byte for byte. A shell parser that cannot be run now refuses the proof the same way,
  where it used to leave the mutant VOID.
