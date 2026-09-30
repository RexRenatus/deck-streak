### Fixed

- The one-router census now refuses, in the bot crate's transport and the bot's other sources, a
  hand-built Bot API send URL that names the bot's base URL, and, by the mention of a name it holds, a
  request whose Bot API method it cannot read (a generic request, the client's own HTTP client or the
  HTTP crate) outside its named site (SPEC-041, issue #429).
- A use of one of the four reqwest paths in `clippy.toml` is now named by the compiler's resolved
  path: the workspace's clippy stage refuses a use of the client type or of `get`, `new` and
  `builder` under any name a source binds it to, outside the bot transport's one annotated site, and
  a test refuses any other suppression of the rule (SPEC-041 A18, issue #429). A request over a raw
  socket or through another HTTP crate is not read.
- A share caption over the Bot API limit of 1,024 UTF-16 units is now refused before any Bot API
  call, by the same bound function the photo path uses (SPEC-132, issue #430).
