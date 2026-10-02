### Added

- The coin wallet, in the economy crate (SPEC-082, part one of three): the coin constants and the
  pure rules, each equal to the predecessor's, for the day's mint, the daily loss cap, the scaled
  fine and the debit clip; and the wallet's seven ports over one coin ledger, a credit, a
  once-ever credit, the day's mint settle, a purchase, a floor-clipped debit, a refund and a capped
  debit. Each port reads the balance and writes its movement in one immediate transaction, so the
  balance never falls below zero, and a key holds at most one movement (ADR-308).
- Migration 008201, which creates the `coin_ledger` and `economy_state` tables, and the economy
  data-rights port, which exports and erases both with the learner's data.
- A model of concurrent callers over the coin ledger, checked for the zero floor, one movement per
  key, a once-ever credit and a settled mint that never falls, with a witness for each; and a Lean
  proof of the debit clip's bounds and of the mint's range and order, checked against
  `clip_debit` and `mint_for_base_xp` over 747 recorded vectors.
