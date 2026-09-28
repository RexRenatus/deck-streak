### Added

- `scripts/vendor-packs.py` re-vendors the pack probes from the packs' checkout in one command. It
  drops every file an exclusion names before reading it, scans every other file with the public
  scrub's rules and the maintainer's private list, and writes nothing unless every file passed.

### Changed

- The vendored-packs manifest records each exclusion as globs a tool applies, with its reason.
