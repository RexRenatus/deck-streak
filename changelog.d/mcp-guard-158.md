### Added

- The MCP server's bearer guard, part one of SPEC-119 (#158), as a library of the new adapter
  crate `deck-streak-mcp`, whose only dependency edge is the kernel (ADR-320): the core and
  law-track credentials read only through the credential loader, each at least 32 characters, no
  two sharing a value, and one holding a character no request can present refused at start by its
  id; the bearer parsed from one `Authorization` header and matched by comparing SHA-256 digests in
  constant time over every grant; one `401 unauthorized` with `WWW-Authenticate: Bearer` for every
  refusal; and the predecessor's limiter of five fresh failures a minute over at most 512 buckets,
  proved against its goldens.
- A model of the guard's shared limiter, checked with a witness for each way a granted token could
  be refused, a bucket could pass its count, the buckets could pass their cap, or a refusal could
  say why, and mutation rows for the credentials, the bearer, the match, the refusal and the
  limiter.
