### Added

- The vault to Anki flashcard bridge (W9) is specified as three planned SPECs, three proposed ADRs
  and two schematics, one SPEC per surface of the flow, each naming its issue, its prerequisites and
  its mutation band. A card reaches Anki only as a package the owner imports, and only after the
  owner's tap; DeckStreak reads the vault for it and writes nothing there.
  - SPEC-150: the flashcards in a note the owner tagged become pending candidates on request, each
    with a GUID from its note's identity and its block key, and only the owner's approve, edit or
    reject moves one, from the bot or the Mini App through one use case (ADR-150, ADR-152).
  - SPEC-151: `/vaultpack` builds a package of the approved cards alone with the engine's own
    export, each note keeping its GUID and its decision's time, and sends it into the owner's chat
    (ADR-151, ADR-152).
  - SPEC-152: the Mini App's vault card screen lists the pending cards and lets the owner approve,
    edit or reject each, and scan again.
