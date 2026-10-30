"""The leg control's target (SPEC-420 R9): three lines, one listed mutant on each.

`direct` is executed by a check in the runner's own process, `in_child` only by a child process a
check starts, and `unobserved` by a check that asserts nothing of it. So a runner that judges the
three reads KILLED, KILLED and SURVIVED, in that order, or it is not judging what it says.
"""


def direct(box, a, b):
    box.append(a + b)


def in_child(box, a, b):
    box.append(a * b)


def unobserved(box, a, b):
    box.append(a - b)
