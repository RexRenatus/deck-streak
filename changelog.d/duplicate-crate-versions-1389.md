### Added

- A guard test reads every package entry of the workspace's `Cargo.lock` and lists each crate name
  it holds at more than one version, or from more than one source, with the reason each version is
  there. A name that is not listed is reported as a warning beside cargo-deny's own, a listed name
  the lock no longer holds twice fails, and a lock with no package entry is VOID (SPEC-668).
