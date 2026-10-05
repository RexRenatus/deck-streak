---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The edge refuses every spelling of the sync route but its own, read from the request-target as received, and the ban's test takes its population from that rule

## Context and Problem Statement

#658 asks that the edge and the ban filter agree on one spelling of the sync route, so that both
judge the same request the same way. The edge must serve the route only under its canonical
spelling and refuse every other spelling of it. A test must then feed the filter each spelling of
a refused sync login that the edge would serve, and the filter must count every one. Both land
before #161's step that installs the filter.

ADR-347 D4 placed the server under its own path on the web app's origin. ADR-351 D3 bounded its
login with a filter and a jail over the edge's access log, and D4 made that log one record of the
sync route alone. The route is chosen by `handle /anki-sync/*` and its prefix is removed by
`uri strip_prefix /anki-sync`. Caddy documents both as comparing a normalised path: decoded,
cleaned of dot segments and doubled slashes, and case-insensitive. The log records the
request-target as received. This record decides where the one spelling is stated and what reads
it (D1), how the filter's test takes its population (D2), and how the rule is judged without
running the edge (D3).

## Decision Drivers

- The cure is the smallest that makes both acceptance lines true. No restructuring of the route
  re-indents the lines that SPEC-337's and SPEC-340's rows anchor on.
- The filter and the jail stay as SPEC-340 left them, and their install stays #161's step 4.
- Nothing runs on a host. A host check is a recorded step on #161.
- No new binary in CI, no new privilege, and no patch to the fork (ADR-336).
- The rule states what is served, so a test can derive the served population from it, never from a
  hand-kept list.

## Considered Options (the alternatives it was chosen against)

D1, where the one spelling is stated and what reads it:

- A guard inside `handle /anki-sync/*`: a named `expression` matcher on
  `{http.request.orig_uri}`, the request-target as the edge received it. Its RE2 pattern admits
  the prefix as written, then segments of ASCII letters and digits, then any query, in the origin
  form or the absolute form with a lower-case scheme and a host of letters, digits, dots, colons
  and hyphens. `respond @sync_respelled 404` answers what the pattern does not admit. That is six
  added lines, four of them a comment. Chosen, because the original request-target is what the
  log records and what no rewrite in the route changes, so the guard works wherever `respond`
  falls in Caddy's directive order. A named matcher on a second `respond` keeps the handle's
  existing lines byte for byte.
- A `path` or `path_regexp` matcher on the spelling: rejected because Caddy's `path` matcher
  compares a lower-cased, decoded and cleaned path, and `path_regexp` a decoded and cleaned one.
  Neither sees the spelling the request carried.
- `{http.request.uri}` under a `route` block placed before the strip: rejected because it
  re-indents the handle, so S33730 to S33735, which anchor on the handle's indented lines, lose
  their anchors and must move with no change in behaviour. It also reads a value that a later rewrite would change.
- Widening the filter to every spelling the handle's matcher accepts: rejected because that set
  has no bound (any escape, any case, any dot segment). #658's first acceptance line asks the edge
  to refuse those spellings in any case.
- A patch to the fork's router: rejected because ADR-336 keeps the fork's patches minimal, and the
  spelling is the edge's to state.
- A redirect of each other spelling to the one spelling: rejected because a redirect is not a
  refusal, and a client that follows it is served.
- Refusing only an escape and a prefix in another case: rejected because the strip also cleans
  empty and dot segments, so the route would still have more than one spelling.
- A list of the server's route names in the pattern: rejected because it couples the edge to the
  fork's route table. The class of letters and digits holds every route the server has, and the
  server answers anything else itself.
- Refusing the absolute form: rejected because RFC 9112 section 3.2.2 has a server accept it, and
  the filter already counts it.
- A status of 400 or 403, or `abort`: rejected. A 403 at the login route is the status the filter
  counts, so the edge's own refusal would feed the ban. A 400 and a dropped connection would differ
  from the 404 the hidden health routes answer (ADR-025). 404 is chosen.

D2, how the filter's test takes its population:

- The test reads the guard from the Caddy block with the existing Caddyfile reader. It forms each
  spelling of the login: the login itself and each of its respellings, crossed with three queries
  and five forms of the target. It keeps those the edge serves, which means those the handle's
  matcher places in the route and the guard admits; with no guard, it keeps every one the matcher
  places. It feeds each kept spelling as a refused-login log line to the filter, read through
  ConfigParser's basic interpolation as the ban service reads it, and asserts that each is counted
  against its address. Chosen, because the served set then follows the edge: a change to the guard
  that serves a new spelling enters the test's population without anyone editing it.
- A fixed list of spellings beside SPEC-340 A6's: rejected because it duplicates A6 and does not
  follow the edge. A guard that admitted more would pass it.
- Changing the filter as well: rejected because, under the guard, every login the edge serves is
  in the filter's existing shape. A change would move SPEC-340's rows and #161's step-4 check for
  no new behaviour.

D3, how the guard is judged without running the edge:

- The test reads the pattern from the block and judges it with Python's `re`. It refuses any
  pattern outside the part of RE2 that both engines read alike (literals, `^`, `$`, `.`, classes,
  `(?:` groups, single `?`, `+` and `*`), and planted patterns prove the check refuses each
  construct by name. It also asserts that no spelling it feeds holds a newline, the one input on
  which the two engines' `$` differ. The edge's own behaviour is checked once on the host, at
  #161's step, before the filter is installed. Chosen, because it decides the rule in CI at no new
  cost.
- Running Caddy in CI against the rendered block: rejected because it adds a binary to the
  toolchain, which is a rail question, for a rule a static read already decides. The host check
  covers the one fact a static read cannot: that the installed edge reads `orig_uri` as
  documented.
- A Lean or TLA+ proof that the two regular languages agree: rejected because the subset is small
  and finite enumeration with the subset check decides the property the acceptance states.

## Decision Outcome

The chosen option of each of D1 to D3. The edge gains one matcher and one refusal in the sync
route. The tests gain the guard's reader, A1 and A2, and SPEC-337 A6's list of the route's
children names the guard and its refusal. The filter, the jail and the render script are
unchanged.

### Consequences

- Good, because the edge and the filter now read one field, the request-target as received, and
  only the one spelling of the route reaches the server.
- Good, because the filter's test follows the edge: a guard that serves more grows A2's
  population.
- Bad, because a client whose endpoint is configured in another spelling is answered 404 rather
  than served. The runbook's endpoint and the staging login at #161 answer it.
- Bad, because the guard's pattern is one long line in the block. It holds no quote, brace, hash
  or backslash, so the Caddyfile's lexer, its placeholder pass and the test's reader all read it as
  one token.

### Confirmation

SPEC-351's acceptance tests A1 and A2, its rows in S35100-S35199, and #161's recorded
edge check before step 4.

## What would make this wrong

- Caddy's `{http.request.orig_uri}` stops being the request-target as received, or the access log
  stops recording that field. The host check at #161 catches the first; SPEC-340 A7 and the
  `started` check catch the second.
- The server gains a route whose name holds a character outside letters and digits. A1's route
  list and the staging rehearsal catch it.
- The `expression` matcher stops reading RE2. A1's subset check no longer proves agreement, and
  this record must be revisited.

## More Information

SPEC-351; ADR-347 D4; ADR-351 D3, D4; ADR-336; ADR-025; #658, #628, #161. Caddy's
documentation: the request matchers (`path`, `path_regexp`, `expression`), the `uri` directive,
the directive order, and the placeholders. The CEL language definition (`matches`, RE2, substring
semantics). RFC 9112 section 3.2.2.
