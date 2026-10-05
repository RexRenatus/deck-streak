# SPEC-351: the edge serves the sync route under one spelling, and the ban filter counts every spelling of a refused login the edge serves

- **Issue:** #658, one of the preconditions of the sync server's first deploy (#628); it lands
  before step 4 of #161's SRV-1b host steps, which installs the ban filter. **Context(s):** none
  (the deploy templates and their tests, not a bounded context).
- **Decided by:** ADR-362 (D1 to D3), under ADR-347 D4 (the route on the web app's origin) and
  ADR-351 D3 and D4 (the ban and the access log).
- **Status:** this pull request delivers R1 to R5 and the rows of section 7.
  **Mutation band:** S35100-S35199. **Model:** none; `formal/tla/SyncSnapshotWindow` is
  unchanged.

## 1. The problem, measured

The edge and the ban filter should agree on one spelling of the sync route, so that both judge the
same request the same way. Measured at dev `02a8e458`:

- The sync route's handle states no spelling of the request-target. `grep -c 'orig_uri'
  deploy/caddy/deck-streak.caddy` prints 0. The handle (`handle /anki-sync/*`, line 60) holds five
  children: the strip, the health matcher, its refusal, the body bound and the proxy (SPEC-337 A6,
  `test_the_caddy_block_routes_the_sync_server_under_its_own_path`).
- Both of the handle's rules that read the path compare a normalised form of it. The handle's
  `path` matcher compares the path decoded, cleaned of dot segments and doubled slashes, and in any
  case; `uri strip_prefix` (line 63) removes the prefix from the same normalised form, in any case
  (Caddy's documentation of the `path` matcher and of the `uri` directive). Neither rule can state
  one spelling.
- The filter is held by SPEC-340 A6 to the origin and absolute forms, a query and an escaped
  method segment: `test_sync_ban.py` holds 2 tests, and A6 asserts 7 lines the filter counts and
  13 it does not (`examined 13 line(s) that are not a refused sync login`). No test reads the edge
  and the filter together: `grep -ci caddy scripts/tests/test_sync_ban.py` prints 1, the jail's
  journal match at line 155.
- Two mutation rows hold the filter, its status (S34014) and its method segment (S34015). No row
  holds its query arm or its absolute-form arm.

## 2. Requirements

R1. The edge serves the sync route under its one spelling. The request-target as the edge received
    it is the prefix /anki-sync as written, then one or more segments of ASCII letters and
    digits, then any query. It is in the origin form, or in the absolute form with a lower-case
    `http` or `https` scheme and a host of letters, digits, dots, colons and hyphens. Every route
    of the sync server, in the spelling its own client sends, has this spelling.
R2. The edge answers 404 to every other spelling of a request that the handle's matcher places in
    the route, and proxies nothing of it. Other spellings include an escape anywhere in the path,
    the prefix in another case, an empty, `.` or `..` segment, and a trailing slash.
R3. The guard reads the request-target as the edge received it (`{http.request.orig_uri}`). The
    access log records that same field, and no rewrite in the route changes it. Its pattern uses
    only the part of RE2 that Python's `re` reads the same way: literals, `^`, `$`, `.`, classes
    of literals and ranges, `(?:` groups, and single `?`, `+` and `*`. The test can then judge the
    pattern with the regular expression the edge runs.
R4. A test feeds the ban filter each spelling of a refused sync login that the edge would serve,
    and the filter counts every one against its address. The test derives the spellings from the
    edge's own guard and reads the filter through the reader the ban service uses.
R5. The filter, the jail and the render script are unchanged. The delivery lands before step 4 of
    #161's SRV-1b host steps; that order is a step recorded on #161, not a test.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | R1 to R3: the edge's guard refuses every respelling of the route that the handle places in it, serves every route of the server in its one spelling, in both forms and with or without a query, reads the target as received, answers 404, and is written in the shared subset | `test_deploy_templates.py` TheCaddyBlock |
| A2 | R4: every spelling of a refused sync login that the edge serves, derived from its guard, is counted by the filter against its address | `test_sync_ban.py` TheSyncBanJail |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_sync_route_is_served_under_its_one_spelling
A2: python3 -m unittest discover -s scripts/tests -p test_sync_ban.py -k test_the_filter_counts_every_spelling_of_a_refused_login_the_edge_serves
```

R5 has no test of its own. The diff shows that the filter, the jail and the render script are
unchanged, and SPEC-340 A6 still holds the filter's own population. The order before #161's step
is recorded on that issue.

## 4. File manifest

| file | context | change |
|---|---|---|
| `deploy/caddy/deck-streak.caddy` | the edge | changed: the guard and its refusal in the sync route's handle |
| `scripts/tests/test_deploy_templates.py` | the deploy templates' tests | changed: the guard's reader and the spelling helpers, A1, and SPEC-337 A6's list of the route's children |
| `scripts/tests/test_sync_ban.py` | the ban's tests | changed: A2 |
| `scripts/mutation-rows.d/S35100-S35199.json` | mutation rows | added |
| `docs/specs/SPEC-351-the-edge-serves-the-sync-route-under-one-spelling.md` | this SPEC | added |
| `docs/decisions/ADR-362-the-edge-serves-the-sync-route-under-one-spelling.md` | its decisions | added |
| `docs/schematics/the-edge-serves-the-sync-route-under-one-spelling.md` | the edge's refusal branch | added |
| `docs/red-first/SPEC-351.md` | the red-first record | added |
| `changelog.d/sync-route-spelling-351.md` | the changelog fragment | added |

## 5. What this does NOT do

- It changes no host. The new block reaches a host only through SRV-1's Caddy step, and the check
  that the edge refuses before the filter is installed is a recorded step on #161.
- It changes neither the ban filter nor the jail. A refused respelling is answered 404 and is not
  counted toward a ban; the filter still counts the refused login at status 403, as SPEC-340 left
  it (#628, #161).
- It leaves the bare path's redirect (`handle /anki-sync`) as it is. That handle serves nothing
  and points at the one spelling (#658).
- It holds no route but the sync route to one spelling; the API's and the SPA's routes keep their
  handles (#658).
- It runs no edge in CI and adds no binary to the toolchain. The guard is judged by reading the
  block, and its behaviour on a host is checked at #161's step.

## 6. Risks

- A client configured with the endpoint in another spelling, another case or a doubled slash, is
  answered 404. The staging login at the one spelling in #161's step detects it, and the cutover
  runbook names the endpoint as the web app's origin at /anki-sync/.
- A future route of the server with a character outside letters and digits would be refused. A1's
  list of the server's routes and the staging rehearsal (#161) detect it when the route is added.
- A change in the edge's meaning of `orig_uri` or of the `expression` matcher would change what the
  guard reads. Caddy's own validation at install and #161's edge check before the filter detect it.
- A pattern outside the shared subset could read differently at the edge than in the test. A1
  refuses any such pattern, and planted patterns prove that the check refuses each construct by
  name.

## 7. The mutation rows

S35100-S35199, in `scripts/mutation-rows.d/S35100-S35199.json`, table
`SCRIPT_MUTATIONS`. Each row's anchor and mutant are copied from the cured file.

- The guard (A1):
  - the target as rewritten, in place of as received (S35101);
  - the guard's negation dropped (S35102);
  - an escape admitted in a segment (S35103);
  - an empty segment admitted (S35104);
  - the prefix admitted in another case (S35105);
  - the refusal removed (S35106);
  - the absolute form refused (S35107);
  - a query refused (S35108);
  - a dot segment admitted (S35109).
- The edge and the filter together (A2):
  - the scheme admitted in another case (S35110);
  - the filter's query arm removed (S35111);
  - the filter's absolute-form arm removed (S35112).

## 8. References

ADR-362; ADR-347 D4; ADR-351 D3, D4; SPEC-337 A6; SPEC-340 R5, R6, A6, A7; #658, #628, #161.
Caddy's documentation: the `path` and `expression` request matchers, the `uri` directive, the
directive order, and the `http.request.orig_uri` placeholder. The CEL language definition's
`matches`. RFC 9112 section 3.2.2, the absolute form.
