# Red-first record: SPEC-605

The criteria tests run in the `hygiene` job of CI, which runs `scripts/tests`. The red commit adds
the Kani run budget to the test table and the reader field table and leaves the settings file
unchanged, so A1 reads red by name there. A2 is generated from the field table the same commit
extends and is recorded not red, with the mutation row that proves it can fail.
