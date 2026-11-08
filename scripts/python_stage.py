"""The python stage's module runner (SPEC-633, ADR-656).

    python3 scripts/python_stage.py <suite>...

Runs each `test_*.py` module of each suite as `python3 -m unittest discover -s <suite> -p
<module>`, each in a process of its own, at most PYTHON_STAGE_JOBS at once (by default the CPU
count; 1 runs them one after another), then the modules of SERIAL_MODULES one by one. It prints
each module's output in module-name order, each followed by its own line `module: <suite>/<name>
ran N test(s), OK` (`FAILED (exit c)` for a module that exits c, not 0), then the stage's last
line, `python: <suite> ran N test(s), exit c; ...`. A suite that ran no test, or one whose module
exited non-zero, fails the stage (SPEC-054 R7). `python3` is the one on PATH, so a stand-in there
answers every module run.
"""

import fnmatch
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

# The modules that never share a run, each with its reason (SPEC-633 R3, SPEC-635 R1).
SERIAL_MODULES = (
    ("test_deploy_*.py", "unmeasured on the box"),
    ("test_engine_pin.py", "spawns cargo"),
)
RAN = re.compile(r"^Ran (\d+) tests?\b", re.M)


def reason(name):
    return next((why for pattern, why in SERIAL_MODULES if fnmatch.fnmatch(name, pattern)), None)


def run(module):
    suite, name = module
    done = subprocess.run(
        ["python3", "-m", "unittest", "discover", "-s", suite, "-p", name],
        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
        capture_output=True,
        text=True,
        check=False,
    )
    output = done.stdout + done.stderr
    found = RAN.findall(output)
    return output, done.returncode, int(found[-1]) if found else 0


def main(suites):
    jobs = int(os.environ.get("PYTHON_STAGE_JOBS") or os.cpu_count())
    modules = [
        (suite, path.name) for suite in suites for path in sorted(Path(suite).glob("test_*.py"))
    ]
    tail = [module for module in modules if reason(module[1])]
    for suite, name in tail:
        print(f"serial: {suite}/{name} ({reason(name)})")
    pooled = [module for module in modules if module not in tail]
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        futures = {module: pool.submit(run, module) for module in pooled}
    results = {module: future.result() for module, future in futures.items()}
    results.update((module, run(module)) for module in tail)
    verdicts, failed = [], 0
    for suite in suites:
        ran = code = 0
        for module in modules:
            if module[0] == suite:
                output, returned, count = results[module]
                sys.stdout.write(output)
                outcome = f"FAILED (exit {returned})" if returned else "OK"
                print(f"module: {module[0]}/{module[1]} ran {count} test(s), {outcome}")
                ran += count
                code = code or returned
        failed = failed or code != 0 or ran == 0
        verdicts.append(f"{suite} ran {ran} test(s), exit {code}")
    print("python: " + "; ".join(verdicts))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
