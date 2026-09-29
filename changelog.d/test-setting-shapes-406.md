### Fixed

- The setting-shape guard's own tests now fail when its source reader is rewritten (nested or
  block comments, raw strings, character literals, escapes, the implementation's own constant, or other
  source files read whole), and the guard reads an out-of-line `#[cfg(test)] mod` file and ignores an
  implementation named inside a block comment.
