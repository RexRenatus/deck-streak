"""The iPhone and iPad shell hands each scene phase and the review's close to its model, and each
sync moment has one call site (SPEC-600 R1, R5, R8 and A10; ADR-620 D7).

The census reads every Swift file git tracks under `ios/App/Sources`, with its comments and the
text of its strings removed by the thin-Swift census's own reader and every blank dropped, so a
sentence about a call is not a call and a call split over lines is still one. It holds each site
to the files and the counts the moments' one path gives it:

- the shell (`DeckListView.swift`) reads the scene phase once and hands each phase to the model's
  one entry, `model.phaseChanged(phase)`, and routes the review screen's close through
  `model.closeReview()`; no view clears the chosen deck itself;
- the shell's model (`AppModel.swift`) calls each moment once: `moments.opened(` after the launch's
  list, `moments.entered(` on a phase, `moments.stored(` after a sign-in, `moments.closed(` at a
  review's close, and `moments.kept(` when it lists the decks again; it hands the sync section its
  completion once, `sync.completed =`;
- the sync section's model (`SyncModel.swift`) reports a completed sync once, `await completed()`;
- the moments (`SyncMoments.swift`) run the sync section's sync once, `await sync.sync()`.

Each census is a function of the data it is handed, so every planted control is a dict: the test
hands it the live tree, then planted trees that each break one rule and must be refused by that
rule's name.
"""

import unittest

from _support import examined
from test_ios_thin_swift import code_of, live_tree

APP = "ios/App/Sources/"
SHELL = APP + "DeckListView.swift"
MODEL = APP + "AppModel.swift"
SYNC = APP + "SyncModel.swift"
MOMENTS = APP + "SyncMoments.swift"
# The shell's hook, written without blanks: it reads the phase and hands each one to the model.
HOOK = ".onChange(of:scenePhase){_,phaseinTask{awaitmodel.phaseChanged(phase)}}"
# Each site, written without blanks, and the files that may hold it with how many times each.
SITES = {
    "@Environment(\\.scenePhase)": {SHELL: 1},
    HOOK: {SHELL: 1},
    "model.phaseChanged(": {SHELL: 1},
    "Task{awaitmodel.closeReview()}": {SHELL: 1},
    "model.chosenDeck=nil": {},
    "moments.opened(": {MODEL: 1},
    "moments.entered(": {MODEL: 1},
    "moments.stored(": {MODEL: 1},
    "moments.closed(": {MODEL: 1},
    "moments.kept(": {MODEL: 1},
    "sync.completed=": {MODEL: 1},
    "awaitcompleted()": {SYNC: 1},
    "awaitsync.sync()": {MOMENTS: 1},
}


def bare(source):
    """A Swift file's code with comments, string text and every blank removed."""
    return "".join(code_of(source).split())


def moment_site_problems(sources):
    """Every rule the tree breaks, each named, and what was examined: (problems, judged)."""
    problems = []
    judged = {"files": [], "sites": []}
    codes = {}
    for path in sorted(sources):
        if not path.startswith(APP):
            continue
        judged["files"].append(path)
        codes[path] = bare(sources[path])
    for site, held in SITES.items():
        for path in sorted(set(codes) | set(held)):
            found = codes.get(path, "").count(site)
            if found:
                judged["sites"].append(f"{path}: {site} {found}")
            if found != held.get(path, 0):
                problems.append(f"{path}: `{site}` found {found} time(s), held {held.get(path, 0)}")
    return problems, judged


def without_hook(sources):
    """The live tree with the shell's scene-phase hook taken out: the planted source A10 names."""
    planted = dict(sources)
    planted[SHELL] = planted[SHELL].replace("Task { await model.phaseChanged(phase) }", "")
    return planted


class TheMomentsHaveOneSiteEach(unittest.TestCase):
    def test_the_shell_hands_each_scene_phase_and_the_review_close_to_the_model(self):
        sources, _ = live_tree()
        problems, judged = moment_site_problems(sources)
        examined("Swift file(s) under ios/App/Sources", judged["files"])
        for site in examined("moment site(s)", judged["sites"]):
            print(site)
        self.assertEqual(problems, [], "each moment is handed over and called from its one site")

        # The planted member: a shell with no hook is refused, so this census can fail.
        planted, _ = moment_site_problems(without_hook(sources))
        self.assertIn(f"{SHELL}: `{HOOK}` found 0 time(s), held 1", planted)
        examined("planted refusal(s)", planted)

    def test_each_planted_moment_site_is_refused_by_name(self):
        sources, _ = live_tree()
        plants = {
            "a view that clears the deck itself": (
                APP + "AccountView.swift",
                "\nfunc clear() { model.chosenDeck = nil }\n",
                f"{APP}AccountView.swift: `model.chosenDeck=nil` found 1 time(s), held 0",
            ),
            "a second launch moment": (
                MODEL,
                "\nawait moments.opened(signedIn: a, reviewOpen: b)\n",
                f"{MODEL}: `moments.opened(` found 2 time(s), held 1",
            ),
            "a sync from the moments outside their one site": (
                APP + "DataModel.swift",
                "\nfunc more() async { await sync.sync() }\n",
                f"{APP}DataModel.swift: `awaitsync.sync()` found 1 time(s), held 0",
            ),
        }
        refused = []
        for name, (path, text, expected) in plants.items():
            planted = dict(sources)
            planted[path] = planted.get(path, "") + text
            problems, _ = moment_site_problems(planted)
            self.assertEqual(problems, [expected], f"the plant {name!r} is refused by name")
            refused.append(expected)

        # A sentence or a comment that names a site is not one.
        quiet = dict(sources)
        quiet[APP + "AccountView.swift"] += (
            '\n// model.chosenDeck = nil in a comment\nlet line = "await sync.sync()"\n'
        )
        self.assertEqual(moment_site_problems(quiet)[0], [], "prose and strings are not sites")
        examined("planted refusal(s)", refused)


if __name__ == "__main__":
    unittest.main()
