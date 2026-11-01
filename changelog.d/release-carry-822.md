### Changed

- A release run carries each Python mutant outcome that one recorded release run of this repository judged, for a file whose
  whole read set is byte-identical at that run and here, and runs only the rest; the verdict holds the listing to the re-run
  outcomes plus the carried ones, and every bound stays as it was.
