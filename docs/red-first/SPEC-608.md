# Red-first record: SPEC-608

Each criterion's test is committed before the change it judges. They run in CI's `hygiene` job, which
runs `scripts/tests`; each was read red there with the report unchanged, and green once the report
reads the planted suite bundle.

