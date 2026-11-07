"""The path gate: which gated Rust job a pull request into dev may skip, and which results the ci
aggregate admits (SPEC-634, ADR-658).

The map, `scripts/read-map.json`, names the paths each gated job reads:

    {"schema": "read-map.v1", "always": [...], "jobs": {"rust": {"reads": [...]}, ...}}

An entry ending in `/` is a directory prefix, `*` is every path, and any other entry is an exact
repository path. `always` joins every job's reads, and a job with no entry reads every path.

`read_map` admits a map or refuses it, and a refused map is None. `prefix_of`, `reads_entry`,
`reads`, `decide` and `admit` are pure functions of their arguments; the Lean entry `PathGate`
ports each one and covers it by name, and its vectors tie the port to this file. The module uses
the standard library alone, has no command line, opens no file and starts no program.

`decide` is a stand-in that answers "run" on every input, the base's behaviour, until the decision
lands (SPEC-634 R4).
"""

import json

SCHEMA = "read-map.v1"
GATED = ("rust", "engine", "release", "census")


class TwiceError(ValueError):
    """A JSON object that holds one key twice, which a reader would take as its last value."""


def _once(pairs):
    """An object hook that refuses a key held twice in one object."""
    keys = [key for key, _ in pairs]
    if len(set(keys)) != len(keys):
        raise TwiceError("a key twice")
    return dict(pairs)


def _strings(value):
    """True when value is a list of strings."""
    return isinstance(value, list) and all(isinstance(item, str) for item in value)


def read_map(text, tracked):
    """The map in `text`, or None with why it is refused.

    `tracked` is every path of the tree the map is read in, as `git ls-files` lists them. An entry
    that is not `*` and does not end in `/` names a directory, a prefix written without its `/`,
    exactly when some tracked path begins with the entry followed by `/`.
    """
    try:
        document = json.loads(text, object_pairs_hook=_once)
    except TwiceError:
        return None, "the map holds a key twice in one object"
    except ValueError:
        return None, "the map is not JSON"
    if not isinstance(document, dict) or set(document) != {"schema", "always", "jobs"}:
        return None, "the map is not an object of schema, always and jobs"
    if document["schema"] != SCHEMA:
        return None, f"the map's schema is not {SCHEMA}"
    jobs = document["jobs"]
    if not isinstance(jobs, dict):
        return None, "the map's jobs is not an object"
    lists = [("always", document["always"])]
    for job, entry in jobs.items():
        if not isinstance(entry, dict) or set(entry) != {"reads"}:
            return None, f"the job {job} is not an object of reads"
        lists.append((job, entry["reads"]))
    directories = {path[:at] for path in tracked for at, char in enumerate(path) if char == "/"}
    for owner, entries in lists:
        if not _strings(entries):
            return None, f"{owner} is not a list of strings"
        for entry in entries:
            if entry == "":
                return None, f"{owner} holds an empty entry"
            if entry != "*" and not entry.endswith("/") and entry in directories:
                return None, f"{owner} names the directory {entry} without its trailing /"
    return document, None


def prefix_of(d, q):
    """True when the entry d is a directory prefix, ending in `/`, and the path q begins with it."""
    return d.endswith("/") and q.startswith(d)


def reads_entry(e, q):
    """True when the entry e reads the path q: `*` reads every path, an exact entry its own path
    and a directory prefix every path under it."""
    return e == "*" or e == q or prefix_of(e, q)


def reads(m, job, q):
    """True when the job reads the path q in the map m: never in a refused map (None), always for
    a job with no entry, and otherwise when an entry of `always` or of the job's reads reads q."""
    if m is None:
        return False
    jobs = m["jobs"]
    if job not in jobs:
        return True
    return any(reads_entry(e, q) for e in m["always"] + jobs[job]["reads"])


def decide(event, job, changed, m):
    """The path gate's answer for one job, "skip" or "run".

    `event` is the triple of the event name, the pull request's base ref and base-is-dev's release
    flag; `changed` is the change list, or None when it cannot be read; `m` is the map, or None
    when it is refused. This stand-in keeps every input and answers "run" on each, as the base
    runs every job on every change.
    """
    kind, base_ref, release = event
    return "run"


def admit(needs, gate_ok, gate_out):
    """The gated jobs whose results do not pass, and every other job with its result.

    `needs` is each job and its result, in order; `gate_ok` is true when the path-gate job
    succeeded, and `gate_out` maps a gated job to the gate's output for it. A gated job's result
    passes when it is "success", or "skipped" while the gate succeeded and its output for the job
    is "skip". Every job that is not gated is returned with its result for the aggregate's existing
    loop.
    """
    refused = []
    rest = []
    for job, result in needs:
        if job not in GATED:
            rest.append((job, result))
        elif result == "success":
            continue
        elif result == "skipped" and gate_ok and gate_out.get(job) == "skip":
            continue
        else:
            refused.append(job)
    return refused, rest
