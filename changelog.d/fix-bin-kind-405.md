### Fixed

- The mutation runner's `bin` killer kind now reads a crate's source files, test targets and
  binaries as the compiler and cargo define them: a `mod` with `#[path]` no longer contributes a
  stray default file, a `[[test]]` target named `bin` is refused at any path, and the refusal's
  binary count is cargo's own. Paths through `..` and absolute paths, entries whose names start
  with a dot, and a manifest with no `edition` key are read as cargo reads them, and a `mod`
  inside a macro invocation is refused by name. What the reader cannot decide is refused by name.
  A generated population judged by `cargo metadata` and `rustc` holds the rule (#405). SPEC-039
  takes an insert-only amendment (ADR-299).
