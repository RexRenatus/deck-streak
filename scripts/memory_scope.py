#!/usr/bin/env python3
"""Run one mutants command inside a memory scope, and record what the kernel did at the cap.

`python3 scripts/memory_scope.py --report <dir> -- cargo mutants ...` asks the systemd manager for
a transient scope that holds this process, so the command and every process it starts share one
memory limit: fifteen sixteenths of the machine, in whole pages, with swap forbidden. A mutant that
allocates without end is then stopped by the kernel at the cap, and the rest of the run goes on.
The cap is measured from the machine and is never an option, a variable or an input.

The scope is checked in force before the command runs (the group is the scope, the limits read
back as asked, the policy is `continue`, the counters start at zero). When any of it is not so the
command does not run: a limit that is not in force would look like a green run.

`<dir>/memory-scope.json` says what happened, and the verdict reads it (SPEC-196). It is written
whole each time: first `running`, then `done` with the kernel's event counts. It never holds a size
in bytes.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

REFUSED = 78
SCOPE_NAME = "memory-scope"
RECORD = "memory-scope.json"
COUNTER = re.compile(r"[0-9]{1,20}")
EVENT_LINE = re.compile(r"([a-z_]+) ([0-9]{1,20})")


class Refused(Exception):
    """The scope is not in force, so the command must not run."""


def build_parser() -> argparse.ArgumentParser:
    """The parser: `--report` and the command after `--`, and nothing that could set the cap."""
    parser = argparse.ArgumentParser(
        prog="memory_scope.py",
        description="Run a command inside a memory scope and record what the kernel did.",
        allow_abbrev=False,
    )
    parser.add_argument("--report", required=True, type=Path, help="directory for the record")
    parser.add_argument("command", nargs=argparse.REMAINDER, help="-- and the command to run")
    return parser


def cap_of(meminfo_text: str, page: int) -> int:
    """Fifteen sixteenths of MemTotal, in whole pages."""
    for line in meminfo_text.splitlines():
        key, _, rest = line.partition(":")
        if key.strip() == "MemTotal":
            return int(rest.split()[0]) * 1024 * 15 // 16 // page * page
    raise ValueError("MemTotal is absent from the meminfo")


def scope_call(unit: str, pid: int, cap: int, sudo: tuple[str, ...]) -> list[str]:
    """The manager call that starts `unit` holding `pid`, under the cap, with swap forbidden."""
    return [
        *sudo,
        "--non-interactive",
        "busctl",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "StartTransientUnit",
        "ssa(sv)a(sa(sv))",
        unit,
        "fail",
        "5",
        "PIDs",
        "au",
        "1",
        str(pid),
        "MemoryMax",
        "t",
        str(cap),
        "MemorySwapMax",
        "t",
        "0",
        "OOMPolicy",
        "s",
        "continue",
        "CollectMode",
        "s",
        "inactive-or-failed",
        "0",
    ]


def write_record(report: Path, record: dict[str, object]) -> None:
    """The record, whole: a temporary file beside it and then a rename."""
    report.mkdir(parents=True, exist_ok=True)
    temporary = report / f".{RECORD}.tmp"
    temporary.write_text(json.dumps(record, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary, report / RECORD)


def read_raw(path: Path) -> str | None:
    """The file's text exactly as written, or None when it cannot be read as UTF-8.

    Bytes are decoded strictly and no newline is translated, so a CRLF, a stray byte or a NUL
    reaches the parser as itself and is refused there, and never crashes it.
    """
    try:
        return path.read_bytes().decode("utf-8")
    except (OSError, UnicodeDecodeError):
        return None


def read_text(path: Path) -> str | None:
    raw = read_raw(path)
    return None if raw is None else raw.strip()


def counter_of(word: str) -> int | None:
    """A kernel counter: one to twenty ASCII decimal digits (an unsigned 64-bit value), else None."""
    return int(word) if COUNTER.fullmatch(word) else None


def lines_of(path: Path) -> list[str] | None:
    """The file's lines as the kernel writes them (each ends in a newline), or None if unreadable."""
    raw = read_raw(path)
    if raw is None:
        return None
    if not raw:
        return []
    return raw.removesuffix("\n").split("\n")


def events_of(group: Path) -> dict[str, int] | None:
    """The counts in `memory.events`, or None unless the file is whole.

    The kernel writes one `<key> <digits>` line per key, and each key once. A file that is
    unreadable, holds any other line, or names a key twice is not whole: taking the last of two
    lines would read a real kill as a clean run.
    """
    lines = lines_of(group / "memory.events")
    if lines is None:
        return None
    counts: dict[str, int] = {}
    for line in lines:
        match = EVENT_LINE.fullmatch(line)
        if match is None or match[1] in counts:
            return None
        counts[match[1]] = int(match[2])
    return counts


def peak_of(group: Path) -> int | None:
    """The count in `memory.peak`, or None unless the file is one whole ASCII number."""
    lines = lines_of(group / "memory.peak")
    if lines is None or len(lines) != 1:
        return None
    return counter_of(lines[0])


def counts_after(group: Path, cap: int) -> tuple[str | None, dict[str, int]]:
    """(why, counts): why a value read after the command cannot be trusted, or None.

    A count that is absent, unreadable or malformed must not read as a clean run: it is the
    absence of a measurement, so the record is not in force and says which value it lacks, and
    the counts it carries are none.
    """
    events = events_of(group)
    if events is None:
        return "memory.events is not whole after the command", {}
    for name in ("oom", "oom_kill", "max"):
        if name not in events:
            return f"memory.events holds no {name} count after the command", {}
    peak = peak_of(group)
    if peak is None:
        return "memory.peak holds no count after the command", {}
    return None, {**events, "peak_percent": peak * 100 // cap}


def unit_of(group: str) -> str:
    """The last component of a control-group path: the name of the scope it is."""
    return group.rpartition("/")[2]


def group_of(proc_cgroup: Path) -> str:
    text = read_text(proc_cgroup) or ""
    return text.splitlines()[-1].split("::", 1)[-1] if text else ""


def wait_for_scope(unit: str, proc_cgroup: Path, reads: int) -> str:
    """This process's control group once it is `unit`, read at most `reads` times, 0.1 s apart."""
    for _ in range(reads - 1):
        group = group_of(proc_cgroup)
        if unit_of(group) == unit:
            return group
        time.sleep(0.1)
    return group_of(proc_cgroup)


def check_in_force(
    unit: str,
    cap: int,
    group: str,
    cgroup_root: Path,
    systemctl: tuple[str, ...],
) -> None:
    """Raise Refused, naming the arm, unless the scope holds the process as asked."""
    if unit_of(group) != unit:
        raise Refused(f"the control group is not the scope {unit}")
    here = cgroup_root / group.lstrip("/")
    for name, wanted in (
        ("memory.max", str(cap)),
        ("memory.swap.max", "0"),
        ("memory.oom.group", "0"),
    ):
        found = read_text(here / name)
        if found != wanted:
            raise Refused(f"{name} is not what the scope was asked for")
    events = events_of(here)
    if events is None:
        raise Refused("memory.events is not whole")
    for name in ("oom", "oom_kill"):
        if name not in events:
            raise Refused(f"memory.events holds no {name} count")
    if events.get("oom") != 0 or events["oom_kill"] != 0:
        raise Refused("memory.events counts an out-of-memory event before the command ran")
    shown = subprocess.run(
        [*systemctl, "show", "--property=OOMPolicy", "--value", unit],
        capture_output=True,
        text=True,
        check=False,
    )
    if shown.stdout.strip() != "continue":
        raise Refused("the unit's OOMPolicy is not continue")


def refuse(report: Path, why: str) -> int:
    print(f"memory-scope: REFUSED: {why}", flush=True)
    write_record(
        report,
        {
            "in_force": False,
            "state": "done",
            "reason": why,
            "oom": 0,
            "oom_kill": 0,
            "max": 0,
            "peak_percent": 0,
        },
    )
    return REFUSED


def run(
    command: list[str],
    report: Path,
    *,
    meminfo: Path = Path("/proc/meminfo"),
    page: int | None = None,
    sudo: tuple[str, ...] = ("sudo",),
    systemctl: tuple[str, ...] = ("systemctl",),
    proc_cgroup: Path = Path("/proc/self/cgroup"),
    cgroup_root: Path = Path("/sys/fs/cgroup"),
    reads: int = 50,
) -> int:
    """Put this process in a scope, check it, run `command` in it, and record the outcome.

    The keyword-only parameters are the test seams; none is reachable from the command line.
    """
    page = os.sysconf("SC_PAGE_SIZE") if page is None else page
    try:
        cap = cap_of(meminfo.read_text(encoding="utf-8"), page)
    except (OSError, ValueError, IndexError) as error:
        return refuse(report, f"no cap can be measured: MemTotal is unreadable ({error})")
    pid = os.getpid()
    unit = f"{SCOPE_NAME}-{pid}.scope"
    called = subprocess.run(
        scope_call(unit, pid, cap, sudo), stdout=subprocess.DEVNULL, check=False
    )
    if called.returncode != 0:
        return refuse(report, f"the manager refused the call (exit {called.returncode})")
    group = wait_for_scope(unit, proc_cgroup, reads)
    try:
        check_in_force(unit, cap, group, cgroup_root, systemctl)
    except Refused as error:
        return refuse(report, str(error))
    here = cgroup_root / group.lstrip("/")
    write_record(
        report,
        {
            "in_force": True,
            "state": "running",
            "reason": None,
            "oom": 0,
            "oom_kill": 0,
            "max": 0,
            "peak_percent": 0,
        },
    )
    code = subprocess.run(command, check=False).returncode
    why, counts = counts_after(here, cap)
    percent = counts.get("peak_percent", 0)
    record = {
        "in_force": why is None,
        "state": "done",
        "reason": why,
        "oom": counts.get("oom", 0),
        "oom_kill": counts.get("oom_kill", 0),
        "max": counts.get("max", 0),
        "peak_percent": percent,
    }
    write_record(report, record)
    if why is None:
        print(
            f"memory-scope: peak {percent}% of the cap; the kernel stopped "
            f"{record['oom_kill']} process(es) at the cap",
            flush=True,
        )
    else:
        print(f"memory-scope: NOT IN FORCE after the command: {why}", flush=True)
    return 128 - code if code < 0 else code


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(sys.argv[1:] if argv is None else argv)
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        build_parser().error("no command after --")
    return run(command, args.report)


if __name__ == "__main__":
    sys.exit(main())
