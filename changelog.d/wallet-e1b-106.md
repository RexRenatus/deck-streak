### Added

- The coin wallet, part two of three (SPEC-082, #106): the recompute fold mints each day's coins in
  its own write, after the day's base XP is final, so a backfilled, a settled or a current day
  holds the mint of its final base, and a fold that fails leaves no mint behind (ADR-315).
- `GET /api/wallet`, for the owner only: the balance, today's loss limit and what is left of it,
  and the coin movements newest first, twenty to a page with a cursor to the next.
- The balance in the Mini App's header on every screen, linking to a new `/wallet` screen that
  lists the movements and loads older pages, in all seven locales.
- A model of the mint reading each day's final base, checked with a witness for each way it could
  read too early, and mutation rows for the mint's phase, a backfilled day's mint, the owner-only
  route, the movements' order and the page's size and boundary.
