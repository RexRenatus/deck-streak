### Changed

- A role refuses to start when a credential it loads is empty, naming the credential as it does for
  a missing one, so the unit fails and its alert quotes the refusal. The sync job records a run
  whose credential is empty as missing its credentials, as it does for a missing one. The alert
  unit refuses to page with an empty credential, before any request, and stays failed.
