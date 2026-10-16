### Changed

- The real tree's settle census compiles in a store kept under the test process's temporary
  directory, outside `target/`, named by a key over the lock file, the toolchain pin, the census's
  command and the tree root (#692). Between censuses the store keeps only units of packages the lock
  file pins; before its first pass the census takes the store's lock and removes every path
  package's units, and it refuses by name any member unit it did not compile itself. The census
  reports the passes it ran and the member units it compiled, read and reused, and every planted
  census still compiles in an empty target of its own.
