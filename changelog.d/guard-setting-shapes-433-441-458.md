### Fixed

- The settings-shape guard now reads a `mod tests;` declared inside an inline module from the file
  rustc reads, or from the `#[path]` it names, with the inner attributes that open that file as the
  module's own (#433). It refuses a crate file whose `macro_rules!` body declares a `cfg(test)`
  module, by an attribute before the `mod`, before a `$( ... )` repetition or inside its braces,
  naming the file (#441). It refuses a module file that any visible declaration compiles without
  `test`, reads a file that only test reaches through its own inner attributes or a
  `cfg_attr(test, path = ...)`, and takes a declaration whose file it cannot name to name every
  file (#458). SPEC-192 sections 12 to 15, ADR-304, rows S19305 to S19314.
