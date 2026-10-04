### Added

- The MCP server's first slice, part a of SPEC-119 (#157, ADR-329): `deckstreakd mcp` serves the
  streamable HTTP transport at `/mcp` alone, keeping no session and answering JSON, on the loopback
  address `DECKSTREAK_MCP_LISTEN` names. Any other address refuses start by the setting's name, and
  a request whose `Host` is not a loopback name is refused. Every request passes the house layers
  and then the bearer guard, ahead of a bound on the requests in flight, so a refused request never
  takes a slot and a request past the bound is shed at once with 503; the body is capped.
- One tool, `get_law_track`, matching its entry of the predecessor's roster golden less the `token`
  parameter. It asks the guard for the `law_track` scope before it reads the law track through
  `coordination`'s use case, and answers the law streak, the XP and the level with the dues, the
  active leeches and the mastery. A pending number answers null, never 0, and the output schema
  declares each pending number nullable and required.
- The role reads its tokens once, through the credential loader, and drains on SIGTERM. Mutation
  rows pin the loopback rule, the scope check, the body cap, the shed, the hosts, the stateless JSON
  transport, the pending numbers, the mastery's rounding, the guard's place in the stack and the
  role's name.
