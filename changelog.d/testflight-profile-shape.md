### Changed

- The TestFlight lane's signing step now prints one line naming the placed profile's shape before
  it reads the profile, as one of six fixed classes with the decoded byte count and the count of
  stored characters outside the base64 alphabet, and refuses a profile with no XML property list
  by its class, before any tool runs, where it stopped before with a Python error. Nothing of the
  profile is printed (SPEC-704).
