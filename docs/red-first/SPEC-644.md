# Red-first record: SPEC-644

The criterion's test is committed before the change it judges. It runs in the `harness` job of
the Apple body, on the iPhone simulator and the iPad simulator, in the pull request's own run.

```red-first
A1: red at 60af062: failed - A1: the scripted form reached nothing from its reference after its first try was refused (job 128574656317, both simulators)
A1: green at 17239c3
```

A1 was committed with the scripted form card unchanged, one try and no second (60af062), and read
red by its assertion on both simulators in the pull request's own run. The card's five tries came
in 17239c3, with the test unchanged, and the run at this delivery's head reads it green on both
simulators. The commits after 17239c3 change only documents.

Correction: the run at 30b3c36 did not read A1. Its `harness` job (job 128944091848) failed at the
step that runs the tests of the Harness scheme, so the planted suite step was skipped and A1 ran
on neither simulator in that run.
