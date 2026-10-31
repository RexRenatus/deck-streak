### Changed

- The box-pack runner's cards cleanup is held by its tests: a sibling `deckstreak-box-cards.*`
  directory and the temporary directory both sit in survive every run byte for byte, an empty
  `BOX_PACKS_OUT` reads as unset, and `BOX_PACKS_KEEP` keeps the made directory for any non-empty
  value, `0` included, and for no empty one (#542). The runner's lines fit 100 columns, and it
  reads both variables before it makes the directory, so nothing runs between the make and the
  cleanup.
