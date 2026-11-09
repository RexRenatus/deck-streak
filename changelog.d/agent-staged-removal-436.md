### Fixed

- The agent's output gate and its runner remove the files they stage through tokio's file removal,
  which runs on the blocking pool, so an unlink never holds a runtime worker. The blocking census no
  longer lists the four removals, a census test refuses any blocking site in the agent's sources,
  and a gate test shows each staged file is removed once its check ends (SPEC-436, #908).
