### Fixed

- The setting-shape guard now reads an implementation from rustc's tokens: a raw identifier, a
  `use ... as` alias (followed through a chain), an attribute or a second item on the line are read,
  and an implementation a macro writes, one whose trait path is a repetition or whose `impl` keyword
  is passed in among them, refuses its file (#436). A pin counts only in an item rustc compiles
  under `--cfg test`, in the implementation's own test module, in a file under `tests/` that cargo
  and rustc compile (a `tests/<name>.rs` or `tests/<dir>/main.rs` root, or a module a compiled file
  declares under attributes the evaluator keeps) and in an out-of-line test module's file, judged
  by one three-valued `cfg` evaluator (#449). A crate file that passes an attribute into a macro's
  module, has a macro name a module's `path`, passes a `cfg(test)` module through an invocation,
  declares an out-of-line module in a block, holds `include!`, imports `include` under any name or
  takes a macro's name from a macro is refused by name (#535). A macro module the evaluator proves
  is no test-only module is read, a module is read from the directory the walk from the crate roots
  gives it, and a `cfg_attr` path below an inline module is read as its predicate chooses under test
  (#536); a macro module under `cfg(any(test, feature = "..."))` stays refused. SPEC-192 section
  16, ADR-310, rows S19315 to S19436.
