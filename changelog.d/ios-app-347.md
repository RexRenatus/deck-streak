### Added

- The native sync login (SPEC-347, part 1 of 2, #625): the engine's login call (1,3) joins the
  native adapter's allow-list and the engine core's native column, and the core guards it before
  the engine sees it. A login whose endpoint is absent, empty, unparseable, carries a user or a
  password, or is not `https` (plain `http` only to a loopback IP literal, the engine's own test
  server) is refused in the engine's own error shape, a `BackendError` of kind `INVALID_INPUT`
  whose text names neither the endpoint nor the credential. A login through the adapter round-trips
  against the engine's own sync server on loopback. ADR-358 records the choices and what each was
  chosen against.
