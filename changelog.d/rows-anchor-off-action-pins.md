### Changed

- The five mutation rows that anchored on a workflow action's pin or its version comment now anchor
  on text an update of that action never rewrites: the two benchmark upload rows on the step's own
  inputs, and the two install rows and the sibling checkout row on a comment line placed above the
  line each mutant edits. A new test refuses any row whose find holds an action pin or a version
  comment and names it, so the next action update leaves every row's anchor in place (SPEC-678).
