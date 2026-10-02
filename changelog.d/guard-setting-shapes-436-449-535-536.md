### Fixed

- The setting-shape guard now reads an implementation from rustc's tokens: a raw identifier, a
  `use ... as` alias (followed through a chain), an attribute or a second item on the line are read,
  and an implementation a macro writes refuses its file (#436). A pin counts only in an item rustc
  compiles under `--cfg test`, in the implementation's own test module, in a file under `tests/` and
  in an out-of-line test module's file, judged by one three-valued `cfg` evaluator (#449). A crate
  file that passes an attribute into a macro's module, has a macro name a module's `path`, passes a
  `cfg(test)` module through an invocation, declares an out-of-line module in a block or holds
  `include!` is refused by name (#535). A macro module the evaluator proves is no test-only module
  is read, a module is read from the directory the walk from the crate roots gives it, and a
  `cfg_attr` path below an inline module is read as its predicate chooses under test (#536); a macro
  module under `cfg(any(test, feature = "..."))` stays refused. SPEC-192 section 16, ADR-310, rows
  S19315 to S19328.
