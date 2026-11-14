### Changed

- The speculative-connection measurement's record states the one reading it keeps unjudged: in
  Chromium's full build, a click on six planted cards' links in the card frame opens one connection,
  with no request, to the link's own origin. It is recorded as an accepted residual in SPEC-650 and
  ADR-672; no layer, test, control or reading of the card suite changes.
