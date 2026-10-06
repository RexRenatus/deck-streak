### Security

- The Mini App's development tools no longer resolve `katex` or `source-map-js` releases that carry
  GHSA-238p-pmpm-9mq7 and GHSA-68fv-2mgg-jv7q (#687). `source-map-js` moved in the lockfile alone,
  and one scoped pnpm override moves `katex` under Mermaid 11, because no update inside Mermaid's
  range reaches a fixed release (ADR-005). Neither package reaches the built Mini App, whose output
  is unchanged.
