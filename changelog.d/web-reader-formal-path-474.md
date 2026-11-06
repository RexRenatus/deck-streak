### Added

- The docs check's `mermaid` reader has a machine-checked statement. A Lean entry ports its fence
  rules and proves, for every text, that each line that may open a `mermaid` fence is read or
  refused by name, with two witnesses that catch the reader without its opener rule and without
  its info rule. The entry's writer prints 841 vectors, and a web test asserts each against the
  reader itself and catches the reader without its opener rule (SPEC-474).
