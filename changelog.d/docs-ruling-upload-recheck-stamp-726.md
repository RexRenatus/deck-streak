### Changed

- An upload's re-check compares the server copy's ids and one integer stamp, built from the greatest
  row usn over every synced table and the collection's schema stamp, by a ruling the owner signs in
  `docs/rulings/` (#726). It replaces the collection's modified stamp, which every download
  restamps. This pull request adds only that ruling and changes no code, test, row or assertion.
