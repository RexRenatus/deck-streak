### Added

- A Lean proof, the entry `lean/AgentPinGroups`, holds that the agent's prune pin refuses a source
  as a shebang exactly when, after one byte order mark is removed, it opens with `#!` and not
  `#![`, whatever tokens its lexer returns, and that the pin's run count and word count descend
  into exactly the same token groups, so neither reads a doc attribute group the other skips. The
  entry's writer prints `formal/vectors/agent-pin-groups.jsonl`, and one new test in each pin's
  file reads those vectors, which ties both copies of the shebang check, and both counts, to the
  one port (SPEC-469, ADR-486, #474).
