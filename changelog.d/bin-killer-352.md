### Added

- A mutation row's killer can now run a binary's own unit tests, as `bin::<test path>`
  (SPEC-039 section 15, issue #352). The runner reads the binary's name from the crate's
  manifest, resolves the test in the binary's own sources, and runs it by its exact name; a
  killer that selects no test is VOID, as for every cargo killer. The kind follows the compiler's
  module rule for a root file of any name, and a crate whose `tests/bin.rs` it would shadow is
  refused by name. Row `S05754` is its first
  user: the daemon's exit grace, one second to two.
