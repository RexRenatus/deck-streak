### Security

- The edge serves the sync route under its one spelling and answers every other spelling of it
  404 (SPEC-351, ADR-362). A test feeds the ban filter each spelling of a refused sync login
  that the edge serves, taken from the edge's own rule, and the filter counts every one.
