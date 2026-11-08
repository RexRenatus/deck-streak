### Changed

- Each card factory test of the iPhone and iPad card probe may load one ready card view in its
  own set-up, after the warm-up and before anything it measures, under the existing warm-up bound
  and with its own load asserted, by a ruling the owner signs in `docs/rulings/` (#796, #781). No
  wait, timeout or bound moves, and no test changes its wait, reading or assertion. This pull
  request adds only that ruling and changes no code, test, row or assertion; the build that
  applies it follows in its own pull request.
