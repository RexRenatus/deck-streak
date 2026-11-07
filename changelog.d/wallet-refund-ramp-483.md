### Changed

- The wallet model's later-day refund property, `RefundWritesItsOwnMovement`, now blocks: its header
  line in `formal/tla/WalletFloor/WalletFloor.tla` reads `ramp=block`, so a violation of it fails the
  entry's formal check where it was only reported. The ramp moved on the formal checker's own reading
  of the entry's streak, and a Python test reads the line and refuses a report, a missing or a
  doubled one. The wallet's code and its port tests are unchanged (SPEC-483, ADR-500, #549).
