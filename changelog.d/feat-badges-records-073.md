### Added

- Badges, records and the next milestone, in the progression crate (SPEC-073, part one of three): the
  40-badge catalog with its course-named descriptions; the study conditions and thresholds,
  equal to the predecessor's; a badge tier awarded at most once, a band key accepted only for a
  configured course and a band from A1 to C2, and an unknown key refused before any write; record
  detection, which counts only a value above the stored best and whole minutes; the record to
  chase; and the next milestone across the reviews, streak and mature-card ladders, with a tie
  kept by the earlier ladder.
- The `badges_earned` and `records` tables, exported and erased with the learner's data.
- A Lean proof that the next milestone is the least remaining fraction with its tie order, checked
  against `next_milestone` over 1,287 recorded vectors.
