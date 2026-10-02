### Added

- The kernel now holds CPython's numbers once (SPEC-302, issue #540): the mean, median, sum,
  round, the nearest-rank percentile, `lgamma` and the Mersenne Twister generator with its
  `choices`, each equal to CPython's result to the last bit on the parity goldens and on an
  edge-value fixture. The analytics crate calls the kernel's sum instead of keeping its own.
