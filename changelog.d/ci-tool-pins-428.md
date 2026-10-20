### Changed

- Every CI tool the install action fetches names an exact version, the one each job already installed, so a bump of the
  action no longer moves a tool. Every job that runs steps sets its own time limit, and the workflow lint reads the whole
  `.github` directory, so the Dependabot configuration is audited beside the workflows. Seven checks in the workflow
  tests hold all three and print what they examined (SPEC-428).
