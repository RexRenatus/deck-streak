### Changed

- A release tag's runs now queue in the tag's group, so a third run of one tag waits behind the
  running one and no longer replaces the waiting second. A running release is still never cancelled
  and two runs of one tag still never run at once; GitHub keeps up to a hundred waiting runs of one
  tag and cancels any beyond them.
