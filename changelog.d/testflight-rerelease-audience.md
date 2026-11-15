### Fixed

- The TestFlight re-release check takes a live lane build of either audience: the newest valid,
  unexpired `APP_STORE_ELIGIBLE` build is the one it decides on when it expires later than the
  newest live internal build, or when no internal build is live. Before it, the check took only an
  internal build as live, so once the lane's builds stopped being limited to internal testing it
  would have kept deciding on the last internal build and dispatched the internal lane at every run
  from the lead before that build's expiry (SPEC-374 R16 as amended, A23).
