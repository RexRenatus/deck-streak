"""The TestFlight lanes (SPEC-352 A14 to A20 and A25; ADR-363): `testflight-internal.yml`, which
a dispatch on dev or a push to dev that changes an app input starts, its filter held to the crates
the XCFramework links, and `testflight-release.yml`, which a SemVer tag starts. Each runs a plan job
on a versioned Ubuntu image, the one Apple job body as its framework job, and an app job on the
admitted macOS image that names the lane's environment and alone reads the credential, in its
preflight, signing and upload steps. These tests read both files through `test_ci_workflows.py`'s
reader and compare them with each other, with `release.yml`'s tag guard and with the harness job of
`xcframework.yml`. Nothing here runs a workflow. A secret's name is read from the live files and
compared, never printed: each read is named by its job, its step's id and its role-word variable."""

import re
import unittest

from _support import REPO, examined
from test_ci_workflows import (
    ADMITTED_RUNNERS,
    CACHE_BY_THEMSELVES,
    PINNED,
    SECRET,
    WORKFLOWS,
    action,
    cargo_commands,
    expressions_in,
    load,
    marked_uses,
    push,
    read_hardened,
    rendered,
    strings,
    workflow_file_text,
)
from test_one_static_library import umbrella_closure

# Each lane file and the lane its plan names (SPEC-352 R1).
LANES = {"testflight-internal.yml": "internal", "testflight-release.yml": "release"}
INTERNAL, RELEASE = LANES
# The internal lane's push on dev, filtered to the app's inputs (SPEC-352 R22): every crate the
# XCFramework links, the iOS tree, the lockfile, the workspace manifest and the toolchain pin, the
# two workflow files the lane runs, and the two scripts its steps call.
INTERNAL_PATHS = [
    "crates/ffi/**",
    "crates/engine-core/**",
    "ios/**",
    "Cargo.lock",
    "Cargo.toml",
    "rust-toolchain.toml",
    ".github/workflows/xcframework.yml",
    ".github/workflows/testflight-internal.yml",
    "scripts/ios_lane.py",
    "scripts/ios_icon.py",
]
# The internal lane's queue (SPEC-352 R16 as R22 amends it): a dispatch and a push wait in groups of
# their own, so neither replaces the other; no run in progress is ever cancelled (SPEC-190 R9); and
# a newer push replaces the push run still waiting, so the next build is dev's newest.
INTERNAL_QUEUE = {
    "group": "testflight-internal-${{ github.event_name }}-${{ github.ref }}",
    "cancel-in-progress": "false",
    "queue": "single",
}
# The one Apple job body, called as GitHub reads it from the caller's own commit (SPEC-344 R5).
BODY = "$/.github/workflows/xcframework.yml"
# The release tag filter, the release workflow's own (SPEC-352 R1).
TAGS = ["v[0-9]+.[0-9]+.[0-9]+"]
# The credential's six parts, each by the role word its step variable is named (SPEC-352 R14), and
# the parts each credential step reads, by the step's id.
PARTS = ("KEY", "KEYID", "ISSUER", "CERTIFICATE", "PASSWORD", "PROFILE")
READS = {
    "preflight": PARTS,
    "sign": ("CERTIFICATE", "PASSWORD", "PROFILE"),
    "upload": ("KEY", "KEYID", "ISSUER"),
}
# The verb each credential step runs, and the condition each step after the preflight runs under.
VERBS = {"preflight": "preflight", "sign": "sign", "upload": "upload-to-testflight"}
CONDITIONS = {
    "build-unsigned": "${{ steps.preflight.outputs.placed == 'none' }}",
    "sign": "${{ steps.preflight.outputs.placed == 'all' }}",
    "upload-to-testflight": "${{ steps.preflight.outputs.placed == 'all' }}",
    "clean": "${{ always() }}",
    "summary": "${{ always() }}",
}
# The release workflow's tag guard, which the release lane's plan job runs as its own (R4).
ANCESTRY = "the tag is SemVer, annotated, and its commit is on main"
# The harness job's steps the app job repeats, the first and the last of them (SPEC-352 R19, A20),
# and the harness job's variables those steps and the build read.
FIRST = "this run's XCFramework, bindings and synthetic collection"
LAST = "the project, generated"
HARNESS_ENV = ("REPORT", "RESULTS", "DEVELOPER_DIR", "XCODEGEN_VERSION", "XCODEGEN_DIGEST")
PLAN_OUTPUTS = ("lane", "number", "version")
# A step's own variable, placed as `strings` places it.
STEP_VARIABLE = re.compile(r"jobs\.([^.\[]+)\.steps\[(\d+)\]\.env\.([^.\[]+)")
# A `set` that traces a script's commands into the log.
TRACE = re.compile(r"(?m)^\s*set\s+(?:-[a-wyz]*x|-o\s+xtrace)")


def lane_jobs(name):
    """A lane file's jobs, read as the hardening tests read every workflow."""
    jobs = read_hardened(WORKFLOWS / name).get("jobs")
    return jobs if isinstance(jobs, dict) else {}


def steps_of(job):
    """A job's steps, or none when it holds no list of them."""
    steps = job.get("steps") if isinstance(job, dict) else None
    return steps if isinstance(steps, list) else []


def needs_of(job):
    """The jobs a job waits on, `needs` read as one name or a list."""
    needs = job.get("needs") if isinstance(job, dict) else None
    return [needs] if isinstance(needs, str) else list(needs or [])


def verb(step):
    """The `ios_lane.py` verb a step runs, or '' for any other step."""
    found = re.fullmatch(
        r"python3 scripts/ios_lane\.py ([a-z-]+)(?: --lane [a-z]+)?\n?", str(step.get("run", ""))
    )
    return found.group(1) if found else ""


def credential_reads(workflow):
    """Each secret a read workflow reads, the default token included, in its order: (where, name,
    expression, whole). `where` is (job, step id, variable) for a step's own variable and
    (place,) for any other place; `whole` is whether the value is that one expression."""
    jobs = workflow.get("jobs") if isinstance(workflow.get("jobs"), dict) else {}
    found = []
    for place, text in strings(workflow):
        for expression in expressions_in(text):
            for match in SECRET.finditer(expression):
                name = match.group(1) if match.group(1) is not None else match.group(2)
                at = STEP_VARIABLE.fullmatch(place)
                if at:
                    step = steps_of(jobs.get(at.group(1)))[int(at.group(2))]
                    where = (at.group(1), str(step.get("id")), at.group(3))
                else:
                    where = (place,)
                found.append((where, name, expression, text == f"${{{{ {expression} }}}}"))
    return found


def built_packages(text):
    """The packages a workflow's cargo commands name with `-p` or `--package`, sorted, and every
    cargo command that names none, whose build closure no manifest read can place."""
    named, unnamed = set(), []
    for command in cargo_commands(text):
        found = re.findall(r"(?:^|\s)(?:-p|--package)(?:\s+|=)(\S+)", command.split(" -- ")[0])
        if found:
            named.update(found)
        else:
            unnamed.append(command)
    return sorted(named), unnamed


def filter_problems(paths, closure):
    """Where the internal lane's push filter and the XCFramework's build closure differ (SPEC-352
    R22): a crate the XCFramework links that no `crates/` glob watches, and a `crates/` glob for a
    crate it does not link. `closure` is the `crates/<dir>` directories `umbrella_closure` reads."""
    globs = {str(path) for path in paths if str(path).startswith("crates/")}
    wanted = {f"{directory}/**" for directory in closure}
    unwatched = [
        f"{pattern}: the XCFramework links it, and the internal lane's push filter"
        " does not watch it"
        for pattern in sorted(wanted - globs)
    ]
    unlinked = [
        f"{pattern}: the internal lane's push filter watches it, and the XCFramework"
        " does not link it"
        for pattern in sorted(globs - wanted)
    ]
    return unwatched + unlinked


def push_paths(workflow):
    """The `paths` filter of a read workflow's push trigger, or none when it has no such filter."""
    on = workflow.get("on")
    push = on.get("push") if isinstance(on, dict) else None
    paths = push.get("paths") if isinstance(push, dict) else None
    return list(paths) if isinstance(paths, list) else []


class TheTestflightLanes(unittest.TestCase):
    def test_each_lane_runs_on_its_one_trigger_and_nothing_a_pull_request_starts(self):
        # SPEC-352 A14 (R1, as R22 and SPEC-405 R1 amend it): a dispatch on dev and a push to dev
        # that changes an app input start the internal lane, and a SemVer tag's push or an
        # input-free dispatch at it the release lane, each on nothing else, so no pull request,
        # schedule, tag or other branch's push starts the internal lane.
        triggers = {
            INTERNAL: {
                "workflow_dispatch": None,
                "push": {"branches": ["dev"], "paths": INTERNAL_PATHS},
            },
            RELEASE: {"push": {"tags": TAGS}, "workflow_dispatch": None},
        }
        for name, on in examined("lane triggers", list(triggers.items())):
            self.assertEqual(read_hardened(WORKFLOWS / name).get("on"), on, name)
        self.assertEqual(load("release.yml")["on"]["push"]["tags"], TAGS)

    def test_the_internal_push_filter_watches_every_crate_the_xcframework_links(self):
        # SPEC-352 A25 (R22): the internal lane's push filter watches exactly the crates the
        # XCFramework links, read from the packages its cargo commands build and the workspace's
        # manifests, so a crate added to that closure without the filter goes red here by name.
        paths = push_paths(read_hardened(WORKFLOWS / INTERNAL))
        packages, unnamed = built_packages(workflow_file_text(WORKFLOWS / "xcframework.yml"))
        closure = set()
        for package in packages:
            closure |= set(umbrella_closure(REPO, package))
        self.assertEqual(filter_problems(paths, sorted(closure)), [])
        self.assertEqual(unnamed, [])
        examined("packages the XCFramework builds", packages)
        linked = examined("crates the XCFramework links", sorted(closure))
        watched = [f"{directory}/**" for directory in linked]
        self.assertEqual(
            filter_problems(watched[:-1], linked),
            [
                f"{watched[-1]}: the XCFramework links it, and the internal lane's push filter"
                " does not watch it"
            ],
        )
        self.assertEqual(
            filter_problems([*watched, "crates/not-linked/**"], linked),
            [
                "crates/not-linked/**: the internal lane's push filter watches it, and the"
                " XCFramework does not link it"
            ],
        )
        self.assertEqual(
            built_packages("    cargo run -p a --bin b -- -p c\n    cargo build --release\n"),
            (["a"], ["cargo build --release"]),
        )

    def test_the_job_graph_is_plan_then_framework_then_app(self):
        # SPEC-352 A15 (R2): plan, then the framework call, then the app; a refusal in plan starts
        # no macOS job, and the framework call is passed no secret.
        for name in examined("lane files", LANES):
            jobs = lane_jobs(name)
            self.assertEqual(list(jobs), ["plan", "framework", "app"], name)
            self.assertEqual(needs_of(jobs["plan"]), [], name)
            self.assertEqual(needs_of(jobs["framework"]), ["plan"], name)
            self.assertEqual(needs_of(jobs["app"]), ["plan", "framework"], name)
            self.assertNotIn("secrets", jobs["framework"], name)
            self.assertNotIn("environment", jobs["plan"], name)
            self.assertRegex(str(jobs["plan"].get("runs-on")), r"^ubuntu-\d\d\.\d\d$", name)
            self.assertEqual(jobs["app"].get("runs-on"), ADMITTED_RUNNERS["xcframework.yml"], name)

    def test_only_the_app_job_names_an_environment_and_reads_a_credential(self):
        # SPEC-352 A16 (R7, R14): the app job alone names the lane's environment, and it reads each
        # part only in the step that uses it, in that step's own variable: the preflight as a
        # presence boolean, the signing and upload steps as the value. Each part is one secret in
        # every step, and the six are six different secrets.
        for name in examined("lane files", LANES):
            workflow = read_hardened(WORKFLOWS / name)
            jobs = lane_jobs(name)
            environments = {
                job_id: job.get("environment")
                for job_id, job in jobs.items()
                if isinstance(job, dict) and "environment" in job
            }
            self.assertEqual(environments, {"app": name.removesuffix(".yml")}, name)
            reads = credential_reads(workflow)
            self.assertEqual(
                sorted(where for where, _name, _expression, _whole in reads),
                sorted(("app", step, part) for step, parts in READS.items() for part in parts),
                name,
            )
            named, forms = {}, {}
            for (_job, step, part), secret, expression, whole in reads:
                named.setdefault(part, set()).add(secret)
                form = f"secrets.{secret} != ''" if step == "preflight" else f"secrets.{secret}"
                forms[step, part] = whole and expression == form
            self.assertEqual(
                {part: len(found) for part, found in named.items()}, dict.fromkeys(PARTS, 1)
            )
            self.assertEqual(len(set().union(*named.values())), len(PARTS), name)
            self.assertEqual(forms, dict.fromkeys(forms, True), name)
            steps = steps_of(jobs["app"])
            ids = {step.get("id"): verb(step) for step in steps if step.get("id") in VERBS}
            self.assertEqual(ids, VERBS, name)
            conditions = {verb(step): step.get("if") for step in steps if verb(step) in CONDITIONS}
            self.assertEqual(conditions, CONDITIONS, name)

    def test_the_lanes_are_hardened_pinned_uncached_and_queued(self):
        # SPEC-352 A17 (R13, R16, as R22 amends R16): a read-only token, a full history in plan and
        # no persisted token anywhere, every action pinned, no cache and no artifact of its own, no
        # expression and no trace in a script, clean and summary always; the release lane keeps one
        # queue per ref that never cancels and never replaces, and the internal lane one queue per
        # event and ref that never cancels, where a newer waiting run replaces an older one.
        for name in examined("lane files", LANES):
            workflow = read_hardened(WORKFLOWS / name)
            jobs = lane_jobs(name)
            self.assertEqual(workflow.get("permissions"), {"contents": "read"}, name)
            stem = name.removesuffix(".yml")
            expected = {
                "group": f"{stem}-${{{{ github.ref }}}}",
                "cancel-in-progress": "false",
                "queue": "max",
            }
            if name == INTERNAL:
                expected = INTERNAL_QUEUE
            self.assertEqual(workflow.get("concurrency"), expected, name)
            for job_id, job in jobs.items():
                self.assertIn(job.get("permissions"), (None, {"contents": "read"}), job_id)
                self.assertNotIn("concurrency", job, job_id)
                if "uses" not in job:
                    self.assertIn("timeout-minutes", job, job_id)
            for ref, is_call in examined("action references", marked_uses(workflow)):
                if is_call:
                    self.assertEqual(ref, BODY, name)
                else:
                    self.assertRegex(str(ref), PINNED, name)
            steps = [(job_id, step) for job_id, job in jobs.items() for step in steps_of(job)]
            checkouts = [
                (job_id, step.get("with") or {})
                for job_id, step in steps
                if action(step) == "actions/checkout"
            ]
            for job_id, given in examined("checkouts", checkouts):
                self.assertEqual(given.get("persist-credentials"), "false", job_id)
            self.assertEqual(
                [given.get("fetch-depth") for job_id, given in checkouts if job_id == "plan"],
                ["0"],
                name,
            )
            for job_id, step in steps:
                uses = action(step)
                self.assertFalse(uses.startswith("actions/cache"), f"{name}: {job_id}: {uses}")
                self.assertNotIn(uses, CACHE_BY_THEMSELVES, f"{name}: {job_id}")
                self.assertNotEqual(uses, "actions/upload-artifact", f"{name}: {job_id}")
            always = {verb(step): step.get("if") for _job, step in steps if verb(step)}
            self.assertEqual(
                {each: always.get(each) for each in ("clean", "summary")},
                {"clean": "${{ always() }}", "summary": "${{ always() }}"},
                name,
            )
            for job_id, step in examined("run steps", [(j, s) for j, s in steps if "run" in s]):
                self.assertNotIn("${{", str(step["run"]), f"{name}: {job_id}: {step.get('name')}")
                self.assertNotRegex(str(step["run"]), TRACE, f"{name}: {job_id}")

    def test_the_two_app_jobs_differ_only_in_environment_and_lane(self):
        # SPEC-352 A18 (R4, R17): the app jobs are one job but for the environment, the lane
        # reaching them as the plan's output; the plan jobs are one job but for the lane argument
        # and the release lane's ancestry step, which is the release workflow's, byte for byte.
        internal, release = lane_jobs(INTERNAL), lane_jobs(RELEASE)
        self.assertIn("app", list(internal))
        self.assertIn("app", list(release))
        self.assertEqual(internal["app"].get("environment"), "testflight-internal")
        self.assertEqual(release["app"].get("environment"), "testflight-release")
        self.assertEqual({**release["app"], "environment": "testflight-internal"}, internal["app"])
        guard = [
            step
            for step in steps_of(load("release.yml")["jobs"]["release"])
            if step.get("name") == ANCESTRY
        ]
        planned = steps_of(release["plan"])
        self.assertEqual([step for step in planned if step.get("name") == ANCESTRY], guard)
        self.assertEqual([step.get("name") for step in planned].index(ANCESTRY), 1)
        relaned = [
            {**step, "run": step["run"].replace("--lane internal", "--lane release")}
            if "run" in step
            else step
            for step in steps_of(internal["plan"])
        ]
        rest = [step for step in planned if step.get("name") != ANCESTRY]
        self.assertEqual(rest, relaned)
        self.assertEqual({**release["plan"], "steps": rest}, {**internal["plan"], "steps": relaned})
        self.assertEqual(
            [verb(step) for step in examined("internal plan steps", steps_of(internal["plan"]))],
            ["", "plan"],
        )

    def test_the_framework_job_calls_the_apple_job_body_with_nothing_passed(self):
        # SPEC-352 A19 (R6): the lane calls the one Apple job body with no input and no secret, and
        # that body's triggers are a call and a dispatch, with no input, no secret and no queue.
        for name in examined("lane files", LANES):
            self.assertEqual(
                lane_jobs(name).get("framework"), {"needs": "plan", "uses": BODY}, name
            )
        callee = load("xcframework.yml")
        self.assertEqual(callee["on"], {"workflow_call": None, "workflow_dispatch": None})
        self.assertNotIn("concurrency", callee)

    def test_the_app_job_places_the_framework_as_the_harness_does(self):
        # SPEC-352 A20 (R2, R19): the app job downloads this run's framework once, with no run,
        # token or repository, and repeats the harness job's steps from the download through the
        # generated project; its variables are the harness's constants and the plan's outputs.
        harness = load("xcframework.yml")["jobs"]["harness"]
        named = [step.get("name") for step in steps_of(harness)]
        placed = steps_of(harness)[named.index(FIRST) : named.index(LAST) + 1]
        self.assertGreaterEqual(len(examined("harness steps placed", placed)), 4)
        for name in examined("lane files", LANES):
            jobs = lane_jobs(name)
            app = jobs.get("app") or {}
            steps = steps_of(app)
            self.assertEqual(needs_of(app), ["plan", "framework"], name)
            self.assertIn(FIRST, [step.get("name") for step in steps], name)
            start = [step.get("name") for step in steps].index(FIRST)
            self.assertEqual(steps[start : start + len(placed)], placed, name)
            self.assertEqual(steps[0], steps_of(harness)[0], name)
            downloads = [step for step in steps if action(step) == "actions/download-artifact"]
            self.assertEqual(len(downloads), 1, name)
            self.assertEqual(
                downloads[0].get("with"), {"name": "xcframework", "path": "engine-artifact"}
            )
            self.assertEqual(
                app.get("env"),
                {each: harness["env"][each] for each in HARNESS_ENV}
                | {each.upper(): f"${{{{ needs.plan.outputs.{each} }}}}" for each in PLAN_OUTPUTS},
                name,
            )
            self.assertEqual(
                jobs["plan"].get("outputs"),
                {each: f"${{{{ steps.plan.outputs.{each} }}}}" for each in PLAN_OUTPUTS},
                name,
            )
            plan = [step for step in steps_of(jobs["plan"]) if step.get("id") == "plan"]
            self.assertEqual(
                [step.get("run") for step in plan],
                [f"python3 scripts/ios_lane.py plan --lane {LANES[name]}"],
                name,
            )


class TheReleaseLaneHasASecondPath(unittest.TestCase):
    """SPEC-405 R1, R2 and R5 (ADR-419 D2a): a release tag whose push started no lane run is built
    by a manual dispatch at the tag's own ref, which takes no input, skips nothing the push runs
    and joins the tag's group."""

    def test_the_release_lane_runs_on_a_tag_push_or_an_input_free_dispatch_and_nothing_skips_either(
        self,
    ):
        workflow = read_hardened(WORKFLOWS / RELEASE)
        events = workflow.get("on")
        self.assertEqual(
            list(events),
            ["push", "workflow_dispatch"],
            "the lane runs on a tag push or a manual dispatch, and on nothing else",
        )
        self.assertEqual(events["push"], {"tags": TAGS})
        self.assertIsNone(events["workflow_dispatch"], "the dispatch takes no input")
        jobs = lane_jobs(RELEASE)
        self.assertEqual(sorted(jobs), ["app", "framework", "plan"])
        for name in examined("lane jobs", sorted(jobs)):
            self.assertNotIn("if", jobs[name], f"the job {name} runs on every path")
        guard = [step for step in steps_of(jobs["plan"]) if step.get("name") == ANCESTRY]
        self.assertEqual(len(guard), 1)
        self.assertNotIn("if", guard[0], "the guard step runs on every path")

    def test_a_push_and_a_dispatch_of_one_tag_render_one_lane_group(self):
        group = read_hardened(WORKFLOWS / RELEASE)["concurrency"]["group"]
        contexts = {
            "push": push("refs/tags/v1.0.0", run_id="301"),
            "workflow_dispatch": {
                **push("refs/tags/v1.0.0", run_id="302"),
                "github.event_name": "workflow_dispatch",
            },
        }
        self.assertEqual(
            {event: rendered(group, contexts[event]) for event in contexts},
            {
                "push": "testflight-release-refs/tags/v1.0.0",
                "workflow_dispatch": "testflight-release-refs/tags/v1.0.0",
            },
        )
        declared = list(read_hardened(WORKFLOWS / RELEASE)["on"])
        for event in examined("events", contexts):
            self.assertIn(event, declared, f"the lane declares no {event} trigger")


if __name__ == "__main__":
    unittest.main()
