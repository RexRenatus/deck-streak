### Fixed

- The FSRS-7 switch no longer writes a stock stability that the collection stores as 0. A card whose
  FSRS-7 memory would read 90% recall for less than 0.0001 days, such as one whose first review was
  rated Again, now takes the scheduler's own least stability, 0.0001 days, which the collection
  keeps; every other stability is unchanged (SPEC-413, ADR-427, #823).
- The FSRS-7 replay reads the gap between two reviews whole for any pair of review ids, so a
  corrupt review log can no longer stop a test build or be read as two reviews on one instant
  (SPEC-413, ADR-427, #823).
