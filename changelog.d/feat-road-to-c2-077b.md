### Added

- The Road to C2 in the Mini App and the bot (SPEC-077, part two of two): `GET /api/progress`
  answers each configured course's mastery, band, current unit and six band cells, and
  `GET /api/law` answers the law block with each pending count as pending; both serve the owner
  only.
- The bot's `/progress` states each course's band, mastery and current unit.
- The Mini App's progress screen draws each course's ladder as six band cells, each with its
  mastery and its mature cards, and marks the current band and unit; the law tab shows the law
  block's lines in the server's order, lists a count not yet counted as pending rather than as
  zero, and shows the law cards and today's law XP by tier. Both screens are audited for
  accessibility in both of Telegram's colour schemes and carry their strings in all seven
  locales.

### Changed

- The recompute registers Road to C2's progress step and its band badge step, so a band-up is
  stored, paid and celebrated in production, once ever, even after the learner's data is erased.
- The persona engine's band adapter reads a course's live band from its stored progress.
