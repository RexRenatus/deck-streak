### Added

- The MCP server's systemd unit (SPEC-119, part of #157): `deck-streak-mcp.service` runs the `mcp`
  role with the watchdog, the hardening every service carries and its two tokens loaded as
  credentials from the private rail, inside DeckStreak's existing share of the host (ADR-332). A
  deploy installs it and never starts it; its first start is the owner's, once the core token is
  stored, as `deploy/README.md` describes.
- `DECKSTREAK_MCP_LISTEN` in the committed settings example: the MCP server's loopback address.

### Changed

- The API's processor ceiling gives the MCP server its share; it stays a ceiling, not a
  reservation (ADR-332).
