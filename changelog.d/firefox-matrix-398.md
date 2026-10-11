### Added

- The planted card suite runs in Firefox as a third engine of the `card-sandbox` job, beside
  Chromium and WebKit, so the card frame's layers are proved in the three engines a desktop learner
  is likely to use (SPEC-398, ADR-412; #652). The job installs Firefox in a step of its own and its
  bound grows from 20 to 30 minutes; its check-run name does not change. The engine, study and
  end-to-end suites keep their engines (#764).
