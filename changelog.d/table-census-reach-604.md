### Fixed

- The three table censuses no longer refuse files that hold one unrelated character (SPEC-331,
  #604). A literal piece of one character leaves the workspace-wide pool and is judged only in a
  file's reach: the file's own pieces, the files it includes, and the `const` and `static` items
  it or an included file names, as a word or as a format placeholder. Pieces of two or more
  characters are pooled as before, so every spelling SPEC-324 refuses stays refused, and so does
  a join of a one-character piece held in the joining file, a file it includes, or an item it names. A
  character carried to its join only through a function's return value or argument is no longer
  refused; that class is disclosed, pinned by a test, and tracked under #585. A character reached
  only through a second step (a const naming another const, a renamed re-export, or an include
  inside a named item's initialiser) is not refused either. Rows `S33100` to
  `S33107` pin the reach, and a Lean entry proves that the new refusals are always within the old
  ones and that a lone character is never refused.
