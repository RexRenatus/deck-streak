### Fixed

- The mutation runner's `bin` killer kind reads a `[[bin]]` table with no `path`, whose name no
  inferred binary holds, as cargo does: under edition 2015 from its older paths, the name's own
  `src/<name>.rs` first when the package has no library target, and under a later edition, or
  with no file on those paths, it refuses the manifest by name (#527).
