### Changed

- A release tag's runs now queue in the tag's group, so a third run of one tag waits behind the
  running one and no longer replaces the waiting second. A running release is still never cancelled
  and two runs of one tag still never run at once; GitHub keeps up to a hundred waiting runs of one
  tag and cancels any beyond them.
- The check that holds a release workflow to its queue now reads every workflow as GitHub's parser
  does: a quoted false is text and a tab in the indentation is refused. It reads only the YAML
  forms it names and refuses every other form by its line, so a form YAML reads otherwise, or
  refuses, never passes as text. Every value a release
  workflow holds, down to a job's own keys, is one the parser defines there, and every workflow it
  calls is one of this repository's, held to the same queue. Its group reads the tag's ref alone,
  its cancellation is absent or the boolean false, and no other workflow's or job's block can render
  as its group.
