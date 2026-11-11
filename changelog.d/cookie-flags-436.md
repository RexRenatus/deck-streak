### Added

- A cookie flag census reads every `Set-Cookie` header the services set and checks that each
  value carries `Secure`, `HttpOnly` and `SameSite`. A value it cannot read counts as a site. It
  starts at warn with today's two sites listed, prints what it examined, and refuses an empty
  population (SPEC-436 amendment 1).
