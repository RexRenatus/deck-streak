### Security

- The API's and the MCP server's units now allow loopback peers alone and deny every other address,
  as the sync server's unit already did, and a census holds the lists of every service that listens
  on loopback and every listen setting in the example configuration (SPEC-395, ADR-409, #679).
