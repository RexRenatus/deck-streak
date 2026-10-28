"""The formal obligations ledger's checker and its census (SPEC-496, ADR-514): the red-first stub.

It keeps every name the tests read and none of the behaviour: `check` refuses nothing, `declared`
reads no entry, and `census` counts the ledger's words and finds no name. The checker replaces it.
"""

import hashlib
from pathlib import Path
from typing import NamedTuple

SCHEMA = "formal-obligations.v1"
LEDGER = Path("formal") / "obligations.json"
CENSUS_SALT = "formal-ledger-census:"
# The private core names, each as the SHA-256 of the salt and the name in lower case.
PRIVATE_NAME_DIGESTS = frozenset(
    (
        "ef4b75756228b6a72d10811c10d3aa97fab4c1fb284ddfdd025841e48e833de1",
        "a59175f3edf5cffe4589038941ad7a6f47c651b3e1a85718f6fd3340d370c522",
        "611242a514f71c7955c1bf54729aca37c47be77d6aff46b261a2b68b6a11d9ba",
        "a0a587d54ce9147f901dcc5847791b755508b2542ed4191f95c9e00f07574419",
        "ff4701a06bc65fa26145e1a9599876919a015ecf5b14615ad94109d7ad94c20c",
        "5488b9a96d0ec6345956df83d201da3deed45a61a1814550954063cb4f3ba3f7",
        "29966b1c88d9bd1e7da215ed596b890ef4cc1303836c39cd76816f4605f27dd3",
        "7955ac0c98f54ad131059ea8d245adc8c1a11e4d573fdedbad6c13d81193df30",
        "3a69966521d9931e6737d50a84ee5d9ab0de316c70b45ed62252e2810318f682",
        "56d1f6affd34e94a251a1772d25b7a58bb699505611f9955d1b74682037ba0f7",
        "f368c9dff628c60191d4207445d05db8e714cbc01a380a10daf93aa41fae870b",
        "302c5b15e60e0baae5270e37cfeeec2a0dd64776959dde81a8d670b1b9c0793e",
        "ba7cbf7b7796b8c11d1fc598abaa3c02f120986c925b56af873ef51c5ff14919",
        "948d8c10a2bf9f76c37f911d9f36e4bfa0a8a444d613f529504a2dd71d6ecfbc",
        "6beca2fbfc460bef29dc3227d6c5594008382af4c4956ead1be7c13e5a568c7a",
        "b2b3432f3e1eea90944159dac7382b960373802fea595d4425bc3bf6c5853e4b",
        "7977bbd769b4620120085bcb7f011ab651306daf81c512c38b2b38392710031f",
        "ff144dc18423e3aca17872694c2285a1963dd444f102fdef98b781406eee9c7a",
        "f36ea2cfaa2b2e525b80a601dff266bd3e2475739b7535b8a18ab5580ba7cae2",
        "a7216f28f7ca8be9a6bee85579949a79c9a46fa4b3cc4350d838a62216bbbb32",
        "9687787c387352ff678828be06b24396fccbb7788a367f4ba6d4898334b23724",
        "9abe6f50b3793e78ff1d86c263fd977f6b9fc34cfedd5b5081e0a05383e5d16a",
        "9129d5b2dc452c9e7f5946841b12ad67cca60e1518ee6827327df8eb28d01db3",
        "1407c797b90d2291502d86283a81987e374e54b2220df06d127b69a0360f6aca",
        "f810059a67fcc729ec02253e70a7be7ccac17d3701aad489a55cc6a470e22e4b",
        "0c60c70d7d8a36f06bdccda2251ae0deb587ab089732bc40ad1f16dd53b087bf",
        "641bd31fb1521dca104263c835c24f2c9feea532ff2e6ef8d4d7924a2d8322ff",
        "2198fe69013cc84b07e2c2243a4e1a654024f3cff7c9aa630b3b2d8713297fde",
        "bb5c608a1d6864bc98cb65347b8e84c31bcb730f509e7edd4898b41a9d63d486",
        "6cf8c39272927d9d8b8297e41e0aa64b580f54b2ea17987c147f6fbd7711b2f0",
        "99b3cdb3717f78609c2e3855198d58b2b70d75e688df626a66e000b103dcd9b7",
        "73dcab871e5233e0a93fe0565251df623ae5ea19b42907e92175f459f352c227",
        "3cca1ad745ed71a47458d78400cc3717fa5034b810522b12d50245976639031b",
        "f85464f24ea5e500ed4be1a56f7923903a75725d1de12d9e72462307c316ab78",
        "c60e9729fcc28fd9bae7b32ef8a03c67cf25d9e0e410f6960fa52b87f95d16e8",
        "b1360ff56114b3cd77aca30be26881eb85821479c824c7c825995b6326f1a6ea",
        "1747fb1718b5bd84670d85112cfad7444c10899c82e5b32a9d262db810820fe7",
        "c234602672f5c72291872ac54d491a5f74caa61d10bab53a697c47b859e81eb0",
        "ada29d91ffba3cf2944d67659c8580b10d5c3cb701fea229c2305c22c2186377",
        "e2cf48866a4d4d11542d6ae27c5f14cb2c39fa32c464d70c8edc5e90b9a61bc7",
        "58037bc882f8620a42cfed4980b19a4e2625aca983e03910e1e86c228b0bedfe",
        "7bbd376428080be95813d5d8f2509c2e998a14c346dbf77f923ef25c9c2eeca0",
        "fe6694e2e1b6e461c66fee0770d6b87f63fe12373916bca0c59568bd043a4f89",
        "b65983d9e7923417e0e0a9082c3ccf936fa7af895b4fbeb010296745b9720563",
        "01a90ca995188038685d643cdbf39b9dd94610564b194ebb80c164fad5e3b05e",
        "41775e8196c1093790929c91924c35ad06a448e694a1b2af6cb699268b96f5b7",
        "172b9d17a4218f385f5442c9693a3e94a3e1fb2906d0640df7ca9d62ddca4879",
        "ef5a39e5b90528f49749243ecd400e7be9a4223e8985acb33ffc953b26eb428b",
        "033eedac7886944d02962ec6a1c33ca55bd1ad3fbcc9d4918eeaaca4c1dd8a62",
        "dc2cc651b4a16a7a50a945f6ea48953e9c84dd65ef077b5a4927a67f4e6e40c3",
    )
)


class Refusal(NamedTuple):
    """One refusal: the arm that refused, where in the ledger, and why."""

    arm: str
    where: str
    why: str


def declared(root):
    """The stub reads no entry."""
    return {}


def check(document, root):
    """The stub refuses nothing."""
    return []


def digest(name):
    """The census digest of `name`: the SHA-256 of the salt and the name in lower case."""
    return hashlib.sha256((CENSUS_SALT + name.lower()).encode("utf-8")).hexdigest()


def census(text, digests=PRIVATE_NAME_DIGESTS):
    """The stub counts the words and finds no name."""
    return len(text.split()), []
