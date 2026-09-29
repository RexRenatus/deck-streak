### Added

- The second brain and the AI duties (W6) are specified as twelve planned SPECs, twelve proposed
  ADRs and five schematics, one SPEC per bounded context or tight group, each naming its issues,
  its prerequisites and its mutation band. Every duty runs through SPEC-043's runner with the AI
  route absent by default (ADR-054), and each SPEC tests what the product does without it.
  - SPEC-110: a law drill is answered once through the vault contract, from the bot or the Mini
    App, and its grade is recorded once and paid as a `once` grant keyed by its drill (ADR-110).
  - SPEC-111: the drill coach mints law drills by the owner's cadence, archives the unanswered, and
    grades an answer only when the law gate and an integer score range accept it (ADR-111).
  - SPEC-112: the leech doctor prepares a remedy for a failing card after the sync, capped, against
    a confusable card the engine chose (ADR-112).
  - SPEC-113: the writing tutor corrects a sample and the conversation partner answers one turn,
    each at the owner's level; a requested duty answers as its command's reply (ADR-113).
  - SPEC-114: a practice set is generated on request, its keys stay on the server until it is
    submitted, and its completion pays once, never its score (ADR-114).
  - SPEC-115: the daily digest gains a coaching paragraph that quotes only its own numbers, and goes
    out degraded when coaching cannot be had (ADR-115).
  - SPEC-116: the inbox curator files captures byte for byte from a plan it returns, the daily note
    links the day, and the weekly synthesis cites every claim (ADR-116).
  - SPEC-117: the vault pass runs nightly and on `/vaultops`, one at a time, bounded (ADR-117).
  - SPEC-118: a photo, voice note or document the owner sends lands in the vault inbox once, and a
    quick capture from the Mini App writes the same stub (ADR-118).
  - SPEC-119: the MCP server serves the predecessor's tools in Rust on loopback, and its guard
    refuses every request without a granted bearer, comparing digests in constant time (ADR-119,
    ADR-121).
  - SPEC-120: the collection atlas is a series refreshed from the cards whose stamp changed after
    each sync, and the agent reads it in pages (ADR-120).
  - SPEC-121: the drill workspace answers a drill section by section, and the law progress screen
    reads the ledger, never the vault's dashboards.
