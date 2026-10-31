### Added

- The daily recompute now grants a chest for each finished study session, sweeps a chest left
  unopened past its day and pays it on its own study day, and settles each double-XP window's
  bonus under its own source, all in the day's one write (SPEC-411, ADR-425; #102, #103).
- The owner can list the unopened chests, the tokens held and the open double-XP window at
  `GET /api/chests`, open a chest at `POST /api/chests/{id}/open`, pick an Epic chest's prize (a
  token or a streak freeze) at `POST /api/chests/{id}/pick`, and start the oldest held token's
  window at `POST /api/tokens/activate`. Each route answers only the owner's session, refuses a
  cross-site request, and pays a chest once: a repeated open or pick answers that it is already
  done and writes nothing.
- A chest's XP, an Epic prize's freeze and a token's bonus are written in the same write that
  resolves the chest, so no resolved chest is ever left unpaid.
