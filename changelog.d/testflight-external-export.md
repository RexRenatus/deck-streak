### Changed

- The TestFlight lane's upload step exports the build without TestFlight's internal-only testing
  option, so a build the lane uploads is no longer limited to internal testing. The lane still
  opens no external testing group, no public link and no App Store submission, and the upload's
  key keeps its Developer role (SPEC-352 R10 as amended, A34).
