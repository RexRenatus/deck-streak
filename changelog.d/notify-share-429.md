### Fixed

- The one-router census now refuses any hand-built Bot API send URL in the notifications
  transport, whatever form builds it, outside the two named send methods; a form the census
  cannot read is refused (SPEC-041, issue #429).
- A share caption over the Bot API limit of 1,024 UTF-16 units is now refused before any Bot API
  call, by the same bound function the photo path uses (SPEC-132, issue #430).
