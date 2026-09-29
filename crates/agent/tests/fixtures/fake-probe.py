"""A fake gate probe for the agent's tests: it speaks the packs' probe protocol and nothing more.

    fake-probe.py --root R --subject OUT [--subject TEMPLATE] check <class>

Exit 0 is green, 1 a finding. The classes: output-links (an http link), output-invisible (a
zero-width character) and output-identity (a claim to be human). Every report ends `examined N`.
"""
import sys

args = sys.argv[1:]
subjects = [args[i + 1] for i, a in enumerate(args) if a == "--subject"]
klass = args[args.index("check") + 1]
text = open(subjects[0], encoding="utf-8").read()
findings = {
    "output-links": "http" in text,
    "output-invisible": any(c in text for c in "​‌‍⁠﻿"),
    "output-identity": "i am a human" in text.lower(),
}
if klass not in findings:
    print(f"{klass}: VOID unknown class")
    sys.exit(3)
if findings[klass]:
    print(f"{klass}: finding")
    print("examined 1")
    sys.exit(1)
print(f"{klass}: ok")
print("examined 1")
