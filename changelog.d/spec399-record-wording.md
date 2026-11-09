### Changed

- SPEC-399 and its red-first record now say exactly how the unclosed projects array case was
  proved: with its refusal deleted, the parse reads to the last character before the final
  newline, and that red was read by running the A4 acceptance line again.
