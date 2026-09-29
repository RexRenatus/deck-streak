### Added

- The data migration and the cutover (W8) are specified as seven planned SPECs, seven proposed
  ADRs and three schematics, each SPEC naming its issues, its prerequisites and its mutation band.
  Nothing in them stops the predecessor: every step after the owner's go names #164, and the
  predecessor stays disabled, not removed, until the owner approves its removal.
  - SPEC-140: each context imports its own v9 rows for the study record, and the recovered card
    state keeps its origin in its own column (ADR-140, ADR-141).
  - SPEC-141: the game, the economy, discipline and the messages import their own v9 rows, and the
    runtime settings split by owner (ADR-140).
  - SPEC-142: the import reads a verified copy, rehearses on a copy of DeckStreak's database, and
    applies once over a restored backup, with a rollback by rename (ADR-142).
  - SPEC-143: the cutover checklist moves each contract one at a time, and no contract has two
    writers at any step (ADR-143).
  - SPEC-144: the predecessor retires only after the owner's go, and a day of DeckStreak alone is
    judged by every SLO (ADR-144).
  - SPEC-145: v1.0.0 is released from main after the day alone, and a check that cannot write
    reads the tag, CI and the security settings (ADR-145).
  - SPEC-146: the nightly stats file keeps the predecessor's contract, and is written after the
    day's one sync by one writer (ADR-146).
