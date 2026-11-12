### Added

- Every tracked shell script, by its extension or by a shell `#!` line on a file with no
  extension, is checked by ShellCheck in the hygiene job, from a release archive pinned by its
  digest. A census refuses an empty population and an output that is missing, off the pin or over
  another list, and judges each finding against a listing of today's findings with a reason each;
  it starts at warn, so an unlisted finding prints and a listed one no longer found fails
  (SPEC-655).
