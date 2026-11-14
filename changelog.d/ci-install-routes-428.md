### Changed

- CI holds every tool it installs outside the install action to an exact version: each download is checked against its
  digest, each cargo install is locked at an exact version or revision, each pip install requires hashes, Node comes from
  an exact `.nvmrc` and Rust from the exact toolchain file, and any other install route is refused. Every action pin
  carries its version comment, one commit per action. The workflow tests hold each rule and print what they examined
  (SPEC-428).
