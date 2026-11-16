### Fixed

- The TestFlight lane's signing step gives the archive the identity its include file names, the
  one the job keychain pairs with the profile's certificate, as one `CODE_SIGN_IDENTITY` build
  setting on the archive's command line. Before it, the app target kept the iOS application
  preset's `iPhone Developer` identity, which outranks the project-level include, so the archive
  stopped with `No signing certificate "iOS Development" found` after the profile check had
  passed (SPEC-352 R9 as amended, A33).
