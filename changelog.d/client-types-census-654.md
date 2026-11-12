### Added

- A census in the guard tests compares each request handler's answer keys with the keys the web
  client's parser reads and with any Swift type that mirrors the route. It names every drifted key,
  every client type with no route and every route with no client type, refuses an empty population,
  and starts at warn with today's findings listed (SPEC-654).
