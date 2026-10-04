### Fixed

- The three table censuses no longer refuse files that hold one unrelated character (SPEC-331,
  #604). A literal piece of one character leaves the workspace-wide pool and is judged only in a
  file's reach: the file's own pieces, the files it includes, and the `const` and `static` items
  it or an included file names, as a word or as a format placeholder. Pieces of two or more
  characters are pooled as before, so every spelling SPEC-324 refuses stays refused, and so does
  every join of a one-character piece through an include, a named item or a format string. A
  character carried to its join only through a function's return value or argument is no longer
  refused; that class is disclosed, pinned by a test, and tracked under #585. Rows `S33100` to
  `S33106` pin the reach, and a Lean entry proves that the new refusals are always within the old
  ones and that a lone character is never refused.
