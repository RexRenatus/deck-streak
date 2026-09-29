### Fixed

- A mutation-row band file that repeats a key in one object is refused at every read, in the
  working tree and in a revision, naming the file and the key (SPEC-122, ADR-122). A clean git
  merge of two added tables under one key no longer drops rows silently; `retired` exits 2 on a
  refusal instead of a traceback.
