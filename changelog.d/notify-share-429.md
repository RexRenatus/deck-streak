### Fixed

- The one-router census now refuses, in the bot crate's transport and the bot's other sources, a
  hand-built Bot API send URL that names the bot's base URL, and a request whose Bot API method it
  cannot read (a generic request, the client's own HTTP client or the HTTP crate) outside its named
  site; a form the census cannot read is refused (SPEC-041, issue #429).
- A share caption over the Bot API limit of 1,024 UTF-16 units is now refused before any Bot API
  call, by the same bound function the photo path uses (SPEC-132, issue #430).
