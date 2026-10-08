### Changed

- The MCP server's grant is read or write (SPEC-369, ADR-380). The core credential grants read;
  an optional write credential, `mcp-write-token`, grants write, and a tool that changes data
  needs it. No such tool is served yet, so no client loses anything, and no new credential is
  needed until one is.
- The MCP server's roster no longer carries the predecessor's data-export or erase-everything
  tool (SPEC-369, ADR-380). Export and erase stay on the bot's `/export` and `/delete` and the
  host's `deckstreakd data` commands.
