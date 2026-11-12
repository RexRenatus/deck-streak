"""The scope fixture's target (SPEC-420 A8 to A16).

`scope_checks.EXECUTES` names, for each function here, the checks whose processes execute its
line. `at_class_body` is executed at import and `at_set_up_class` by a class fixture, so no check
executes either; `unreached` is executed by nothing, and `textual` by one check while another
reads it as text.
"""

WIDTH = 2


def in_process(a, b):
    return a + b


def shared(a, b):
    return a * b


def in_child(a, b):
    return a - b


def at_class_body(a, b):
    return a % b


def at_set_up_class(a, b):
    return a // b


def unreached(a, b):
    return a**b


def textual(a, b):
    return a & b
