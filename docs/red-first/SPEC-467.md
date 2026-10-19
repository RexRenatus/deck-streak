# Red-first record: SPEC-467

The tests of A1 to A5 were committed alone (9d586b5d) against the unchanged scripts, so every test
ran and each red failed by assertion. The script (d8779375) turned A1 to A5 green, and the rows
commit (8814060f) is the tip CI read as green. A6 pins the clean run the base already recorded.
The five reds are the only failures of the `hygiene` job at 9d586b5d, read from its log.
No commit between 9d586b5d and 8814060f changes a test file: d8779375, a21d3c26, 1b18b35e and
8814060f were listed with `git log 9d586b5d..HEAD -- scripts/tests`, which printed nothing.

```red-first
A1: red at 9d586b5d: Tuples differ: (3, {'in_force': True, 'max': 0, 'oom': 0, 'oom_kill': 0, [47 chars]ne'}) != (3, {'in_force': False, 'state': 'done', 'reason': 'cgroup[102 chars]: 0})
A1: green at 8814060f
A2: red at 9d586b5d: Tuples differ: (3, {'in_force': True, 'max': 0, 'oom': 0, 'oom_kill': 0, [47 chars]ne'}) != (3, {'in_force': False, 'state': 'done', 'reason': 'cgroup[102 chars]: 0})
A2: green at 8814060f
A3: red at 9d586b5d: Tuples differ: (3, {[14 chars]ue, 'max': 0, 'oom': 0, 'oom_kill': 0, 'peak_p[40 chars]ne'}) != (3, {[14 chars]ue, 'state': 'done', 'reason': None, 'oom': 1,[41 chars] 50})
A3: green at 8814060f
A4: red at 9d586b5d: Tuples differ: (0, [], True) != (1, [], True)
A4: green at 8814060f
A5: red at 9d586b5d: Tuples differ: (0, [], {'in_force': True, 'max': 0, 'oom'[150 chars]p\n') != (1, [], {'in_force': False, 'state': 'done[204 chars]d\n')
A5: green at 8814060f
A6: not red: the clean run, whose processes end with the command, was already recorded as the criterion says before the change; the criterion pins that it stays so, and row S46706 reads KILLED
```

## The model's red-first

The formal entry `formal/tla/LateKillCounted` was checked by the repository's formal checker and by
hand in scratch copies. No CI reads the entry (#806): DeckStreak's CI has no formal job.

- Witness EveryKillIsCountedOrTheRecordIsVoid reads `Invariant EveryKillIsCountedOrTheRecordIsVoid is violated`, 279 distinct states.
- Witness NoKillIsReadAsACleanRun reads `Invariant NoKillIsReadAsACleanRun is violated`, 279 distinct states.
- Witness AKillBetweenTheTwoReadsIsCountedOrTheRecordIsVoid reads `Invariant AKillBetweenTheTwoReadsIsCountedOrTheRecordIsVoid is violated`, 279 distinct states.
- Witness AKillAfterBothReadsIsCountedOrTheRecordIsVoid reads `Invariant AKillAfterBothReadsIsCountedOrTheRecordIsVoid is violated`, 281 distinct states.
- The first three traces: Initial, Start(p1), End(cmd), Return, ReadEvents, Kill(p1), ReadPeak, Write.
- The fourth trace: Initial, Start(p1), End(cmd), Return, ReadEvents, ReadPeak, Kill(p1), Write.
- The main configuration `MCLateKillCounted.cfg` reads `Model checking completed. No error has been found.`, 468 distinct states.
- The checker's card at the stamped entry: `FORMAL EXAMINED entries=1 properties=4 witnesses=4 configs=1`, `FORMAL OK exit=0`, no finding.
- With a third process the main configuration reads no error, 1502 distinct states; with `ReadBeforeAlone = TRUE` it reads `Invariant EveryKillIsCountedOrTheRecordIsVoid is violated`, 693 distinct states.
