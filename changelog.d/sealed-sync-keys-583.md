### Added

- The design of sealed sync: every sync op body (notes, media, deck names, reviews, palace and tags)
  is to be sealed on the device under one account key, which each device key, each passkey through
  its PRF output, and a printed recovery key unwrap, and which email sign-in never reaches. The
  server keeps only clear op heads with the fields its merge rule needs, padded sizes, wrapped keys
  and one daily record per study day (date, reviews, minutes and XP granted), and a census lists
  what each context and table becomes. Sealed sync goes live when DeckStreak's own sync replaces
  the stock-app path, and a test holds the privacy page from claiming it before then (SPEC-583,
  ADR-602, #1079).
