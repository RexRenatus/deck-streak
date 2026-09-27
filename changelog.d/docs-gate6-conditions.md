### Changed

- The plan now follows the owner's gate-6 conditions (ADR-037, ADR-038, ADR-054).
  - DeckStreak syncs the owner's collection once per study day plus on the owner's explicit
    `/sync`, and never uploads or writes a local change, which a recording fake server will prove.
  - Credentials reach each unit from the secret manager at its start through a root-only socket that
    serves only systemd's mapped requests, and never rest on the host.
  - The AI route is optional and off by default. With no route, every AI duty records
    `ai_route_absent`, the surfaces say readings are not enabled, and nothing alerts.
- The W0 and W1 planned SPECs, their ADRs and schematics, the charter's constraint 16, the
  architecture overview and the owner setup are amended to match.
