### Fixed

- The settings-shape guard now reads a `mod tests;` declared inside an inline module from the file
  rustc reads, or from the `#[path]` it names (#433), refuses a crate file whose `macro_rules!`
  body declares a `cfg(test)` module, naming the file (#441), and refuses a module file that any
  visible declaration compiles without `test` (#458). SPEC-192 sections 12 to 14, ADR-304, rows
  S19305 to S19309.
