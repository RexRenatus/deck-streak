### Fixed

- The TestFlight lane's signing step re-wraps the certificate from a file in the lane that only
  the runner's user may read or write, named to openssl by `-in` and removed as soon as the
  re-wrap returns or fails. Before it, the re-wrap was given what the certificate read wrote on
  its standard input, where openssl's key read spent the input and its certificate read found
  none, so the step refused with `openssl pkcs12 failed with exit 1` after a read that had
  succeeded (SPEC-352 R9 as amended, A32).
