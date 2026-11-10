# Schematic: how the hygiene job's ShellCheck step and the shell script census read the tree

Kind: data flow. Decided by ADR-677; built by SPEC-655.

One step of the hygiene job runs ShellCheck over every tracked shell script and saves what it saw;
the python stage that follows runs the census, which lists the same scripts from the tree and
judges the saved output. The tool never decides the job: the census does, starting at warn.

```mermaid
flowchart LR
  s_tree["the tracked files, from git ls-files"]
  s_step["the hygiene job step: the pinned release archive, its digest checked, then ShellCheck"]
  s_saved["the shellcheck folder of the stage logs: files.txt, version.txt, status.txt, output.json"]
  s_upload["the uploaded stage logs of the hygiene job"]
  s_census["scripts/tests/test_shell_script_census.py, run by the python stage"]
  s_listed["LISTED: a path and a check code, each with a count and a reason"]
  s_out["WARN lines at warn; DRIFT, STALE, UNREASONED and VOID fail"]
  s_tree -->|the R1 rule, in shell| s_step
  s_step -->|writes| s_saved
  s_saved --> s_upload
  s_tree -->|the R1 rule, in Python| s_census
  s_census -->|reads| s_saved
  s_census -->|reads| s_listed
  s_census --> s_out
```

## What each part decides

- The step fails only on the download or the digest. It records the tool's exit status and never
  fails on a finding (SPEC-655 R3).
- The census refuses an empty population, and a saved output that is missing, off the pin or
  contradicted by its exit status, as VOID; a saved list other than its own population fails as
  DRIFT (R2, R4).
- Each finding is compared with LISTED by path and check code. At warn an unlisted finding prints a
  WARN line and passes; a listed entry found fewer times than listed fails as STALE, and an entry
  with an empty reason fails as UNREASONED, at both levels (R5, R6).
- Outside the hygiene job no output is saved, so the repository test reads VOID there (R7).
