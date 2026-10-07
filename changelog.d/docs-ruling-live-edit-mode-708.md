### Changed

- A ruling the owner signs in `docs/rulings/` (#708) admits one declared write class, live edit, which
  may change a reviewed note's type and add fields to a note type only while the owner's live-edit
  preference is on. Every other never-list entry stays bound, a one-way sync stays the owner's own
  tap, and nothing builds or calls live edit before the ruling lands. This pull request adds only
  that ruling and its fragment, and changes no code, test, row or gate.
