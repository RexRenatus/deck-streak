### Fixed

- The TestFlight lane's signing step reads the placed certificate once more with `-legacy` when its
  first read fails, and each failed read prints one line naming one class of a closed set,
  `wrong-password`, `legacy-algorithm`, `option-refused` or `unrecognised`, chosen from openssl's
  standard error in memory and never printing any of it. Before it, a failed read printed only
  `openssl pkcs12 failed with exit 1`, so the log could not say whether the password or the
  runner's openssl was the cause. A `-legacy` read that fails too refuses naming both classes and
  runs no further tool (SPEC-352 R9 and R13 as amended, A26 to A28; ADR-363).
