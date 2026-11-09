### Added

- A media store of the app's own: a media file is kept as a file under the SHA-256 of its bytes in a per-user
  folder, three tables in the collection store index the files, their names and their pins, a put commits its row
  only after its file is in place, a file without its row is deleted at the next open, bytes are verified where they
  enter and size-checked where they are read, and a face reads its media by name through the store. The face
  rewrite reads the app's own reference grammar, and every face is unchanged byte for byte. Nothing live reads or
  writes the store yet (SPEC-601, ADR-621).
