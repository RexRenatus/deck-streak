#!/usr/bin/env python3
"""durable-unit-lint.py -- a repository's systemd units, judged offline against full practice.

SPEC-V2-2195 / ADR-V2-2195. The portable half of `skills/packs/durable-services`: every check reads
one repository's `deploy/` tree through `--root` and nothing else -- no box, no `systemctl`, no
network -- so it judges any project (DeckStreak first) the same way it judges a fixture. It is ONE
file with the standard library only, so a public project may vendor it into its own CI; phoenix's
`phxd pack probe` runs this copy through the pack's `{skills}` placeholder.

  durable-unit-lint.py check --id CHECK --root PATH [--format text|json]
      Run one check. Exit 0: it holds. Exit 1: it found something, or the subject holds no unit
      at all (`units-none`: a guard that examined nothing refuses). The pack row's severity is what
      decides whether that refuses: a `block` row goes red, an `advisory` row prints the verdict
      `advisory` and refuses nothing (`phxd pack probe`, SPEC-V2-2195). Exit 2: usage, or an id
      outside the catalog.

  durable-unit-lint.py lint --root PATH [--format text|json]
      Run every check; exit 1 when any blocking check found anything, else 0.

  durable-unit-lint.py catalog [--format text|json]
      Print the check catalog: id, stage, severity, reason and the source that makes it a check.

The subject (SKILL.md "Subject layout"): `deploy/**/*.service`, `*.timer` and `*.slice`, with
drop-ins in `deploy/**/<unit>.d/*.conf`; a unit under a directory named `user` is a user unit, where
the sandboxing that needs a mount namespace is not available (systemd.exec(5), SANDBOXING) and so
is not judged. Beside the units: `deploy/**/litestream*.yml`, `deploy/**/*.env*`,
`deploy/host-budget.json` (`{"memory": "1400M", "cpus": 2}`, the share of the host this stack may
use) and `deploy/**/journald.conf.d/*.conf`.

The directive tables are systemd 255's own (`src/core/load-fragment-gperf.gperf.in` at v255, the
version Ubuntu 24.04 ships), so a key systemd would ignore with "Unknown key name" is found here
before a deploy does. An advisory finding may be waived per unit with an `X-` key, which systemd
itself ignores: `X-DurableServices-Waive=<reason> <why>` in `[Unit]`. A waiver needs its why, and a
blocking check cannot be waived.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import re
import shlex
import sys
import zoneinfo
from collections.abc import Callable, Iterable
from pathlib import Path

SCHEMA = "durable-unit-lint.v1"
DEPLOY = "deploy"
UNIT_KINDS = {".service": "service", ".timer": "timer", ".slice": "slice"}
WAIVE_KEY = "X-DurableServices-Waive"
UNIT_NAME = re.compile(
    r"^[A-Za-z0-9:_.\\-]+(?:@[A-Za-z0-9:_.\\-]*)?\.(?:service|timer|slice)$"
)
KEY_NAME = re.compile(r"^[A-Za-z0-9-]+$")
SECTIONS = {
    "service": {"Unit", "Service", "Install"},
    "timer": {"Unit", "Timer", "Install"},
    "slice": {"Unit", "Slice", "Install"},
}

# systemd.unit(5), [INSTALL] SECTION OPTIONS; the rest are systemd v255's load-fragment table.
INSTALL_KEYS = frozenset(
    "Alias WantedBy RequiredBy UpheldBy Also DefaultInstance".split()
)
UNIT_KEYS = frozenset(
    """
    After AllowIsolate AssertACPower AssertArchitecture AssertCPUFeature AssertCPUPressure
    AssertCPUs AssertCapability AssertControlGroupController AssertCredential
    AssertDirectoryNotEmpty AssertEnvironment AssertFileIsExecutable AssertFileNotEmpty
    AssertFirstBoot AssertGroup AssertHost AssertIOPressure AssertKernelCommandLine
    AssertKernelVersion AssertMemory AssertMemoryPressure AssertNeedsUpdate AssertOSRelease
    AssertPathExists AssertPathExistsGlob AssertPathIsDirectory AssertPathIsEncrypted
    AssertPathIsMountPoint AssertPathIsReadWrite AssertPathIsSymbolicLink AssertSecurity
    AssertUser AssertVirtualization Before BindTo BindsTo CollectMode ConditionACPower
    ConditionArchitecture ConditionCPUFeature ConditionCPUPressure ConditionCPUs
    ConditionCapability ConditionControlGroupController ConditionCredential
    ConditionDirectoryNotEmpty ConditionEnvironment ConditionFileIsExecutable
    ConditionFileNotEmpty ConditionFirmware ConditionFirstBoot ConditionGroup ConditionHost
    ConditionIOPressure ConditionKernelCommandLine ConditionKernelVersion ConditionMemory
    ConditionMemoryPressure ConditionNeedsUpdate ConditionOSRelease ConditionPathExists
    ConditionPathExistsGlob ConditionPathIsDirectory ConditionPathIsEncrypted
    ConditionPathIsMountPoint ConditionPathIsReadWrite ConditionPathIsSymbolicLink
    ConditionSecurity ConditionUser ConditionVirtualization Conflicts DefaultDependencies
    Description Documentation FailureAction FailureActionExitStatus IgnoreOnIsolate
    IgnoreOnSnapshot JobRunningTimeoutSec JobTimeoutAction JobTimeoutRebootArgument JobTimeoutSec
    JoinsNamespaceOf OnFailure OnFailureIsolate OnFailureJobMode OnSuccess OnSuccessJobMode PartOf
    PropagateReloadFrom PropagateReloadTo PropagatesReloadTo PropagatesStopTo RebootArgument
    RefuseManualStart RefuseManualStop ReloadPropagatedFrom Requires RequiresMountsFor
    RequiresOverridable Requisite RequisiteOverridable SourcePath StartLimitAction StartLimitBurst
    StartLimitInterval StartLimitIntervalSec StopPropagatedFrom StopWhenUnneeded SuccessAction
    SuccessActionExitStatus SurviveFinalKillSignal Upholds Wants
    """.split()
)
SERVICE_ONLY_KEYS = frozenset(
    """
    BusName BusPolicy ExecCondition ExecReload ExecStart ExecStartPost ExecStartPre ExecStop
    ExecStopPost ExitType FailureAction FileDescriptorStoreMax FileDescriptorStorePreserve
    GuessMainPID NonBlocking NotifyAccess OOMPolicy OpenFile PIDFile PermissionsStartOnly
    RebootArgument ReloadSignal RemainAfterExit Restart RestartForceExitStatus RestartMaxDelaySec
    RestartMode RestartPreventExitStatus RestartSec RestartSteps RootDirectoryStartOnly
    RuntimeMaxSec RuntimeRandomizedExtraSec Sockets StartLimitAction StartLimitBurst
    StartLimitInterval SuccessExitStatus SysVStartPriority TimeoutAbortSec TimeoutSec
    TimeoutStartFailureMode TimeoutStartSec TimeoutStopFailureMode TimeoutStopSec Type
    USBFunctionDescriptors USBFunctionStrings WatchdogSec
    """.split()
)
EXEC_KEYS = frozenset(
    """
    AmbientCapabilities AppArmorProfile BindPaths BindReadOnlyPaths CPUAffinity
    CPUSchedulingPolicy CPUSchedulingPriority CPUSchedulingResetOnFork CacheDirectory
    CacheDirectoryMode Capabilities CapabilityBoundingSet ConfigurationDirectory
    ConfigurationDirectoryMode CoredumpFilter DynamicUser Environment EnvironmentFile ExecPaths
    ExecSearchPath ExtensionDirectories ExtensionImagePolicy ExtensionImages Group
    IOSchedulingClass IOSchedulingPriority IPCNamespacePath IgnoreSIGPIPE ImportCredential
    InaccessibleDirectories InaccessiblePaths KeyringMode LimitAS LimitCORE LimitCPU LimitDATA
    LimitFSIZE LimitLOCKS LimitMEMLOCK LimitMSGQUEUE LimitNICE LimitNOFILE LimitNPROC LimitRSS
    LimitRTPRIO LimitRTTIME LimitSIGPENDING LimitSTACK LoadCredential LoadCredentialEncrypted
    LockPersonality LogExtraFields LogFilterPatterns LogLevelMax LogNamespace LogRateLimitBurst
    LogRateLimitIntervalSec LogsDirectory LogsDirectoryMode MemoryDenyWriteExecute MemoryKSM
    MountAPIVFS MountFlags MountImagePolicy MountImages NUMAMask NUMAPolicy NetworkNamespacePath
    Nice NoExecPaths NoNewPrivileges OOMScoreAdjust PAMName PassEnvironment Personality
    PrivateDevices PrivateIPC PrivateMounts PrivateNetwork PrivateTmp PrivateUsers ProcSubset
    ProtectClock ProtectControlGroups ProtectHome ProtectHostname ProtectKernelLogs
    ProtectKernelModules ProtectKernelTunables ProtectProc ProtectSystem ReadOnlyDirectories
    ReadOnlyPaths ReadWriteDirectories ReadWritePaths RemoveIPC RestrictAddressFamilies
    RestrictFileSystems RestrictNamespaces RestrictRealtime RestrictSUIDSGID RootDirectory
    RootEphemeral RootHash RootHashSignature RootImage RootImageOptions RootImagePolicy RootVerity
    RuntimeDirectory RuntimeDirectoryMode RuntimeDirectoryPreserve SELinuxContext SecureBits
    SetCredential SetCredentialEncrypted SetLoginEnvironment SmackProcessLabel StandardError
    StandardInput StandardInputData StandardInputText StandardOutput StateDirectory
    StateDirectoryMode SupplementaryGroups SyslogFacility SyslogIdentifier SyslogLevel
    SyslogLevelPrefix SystemCallArchitectures SystemCallErrorNumber SystemCallFilter SystemCallLog
    TTYColumns TTYPath TTYReset TTYRows TTYVHangup TTYVTDisallocate TemporaryFileSystem
    TimeoutCleanSec TimerSlackNSec UMask UnsetEnvironment User UtmpIdentifier UtmpMode
    WorkingDirectory
    """.split()
)
KILL_KEYS = frozenset(
    """
    FinalKillSignal KillMode KillSignal RestartKillSignal SendSIGHUP SendSIGKILL WatchdogSignal
    """.split()
)
CGROUP_KEYS = frozenset(
    """
    AllowedCPUs AllowedMemoryNodes BPFProgram BlockIOAccounting BlockIODeviceWeight
    BlockIOReadBandwidth BlockIOWeight BlockIOWriteBandwidth CPUAccounting CPUQuota
    CPUQuotaPeriodSec CPUShares CPUWeight CoredumpReceive DefaultMemoryLow DefaultMemoryMin
    DefaultStartupMemoryLow Delegate DelegateSubgroup DeviceAllow DevicePolicy DisableControllers
    IOAccounting IODeviceLatencyTargetSec IODeviceWeight IOReadBandwidthMax IOReadIOPSMax IOWeight
    IOWriteBandwidthMax IOWriteIOPSMax IPAccounting IPAddressAllow IPAddressDeny
    IPEgressFilterPath IPIngressFilterPath ManagedOOMMemoryPressure ManagedOOMMemoryPressureLimit
    ManagedOOMPreference ManagedOOMSwap MemoryAccounting MemoryHigh MemoryLimit MemoryLow
    MemoryMax MemoryMin MemoryPressureThresholdSec MemoryPressureWatch MemorySwapMax
    MemoryZSwapMax NFTSet NetClass RestrictNetworkInterfaces Slice SocketBindAllow SocketBindDeny
    StartupAllowedCPUs StartupAllowedMemoryNodes StartupBlockIOWeight StartupCPUShares
    StartupCPUWeight StartupIOWeight StartupMemoryHigh StartupMemoryLow StartupMemoryMax
    StartupMemorySwapMax StartupMemoryZSwapMax TasksAccounting TasksMax
    """.split()
)
TIMER_KEYS = frozenset(
    """
    AccuracySec FixedRandomDelay OnActiveSec OnBootSec OnCalendar OnClockChange OnStartupSec
    OnTimezoneChange OnUnitActiveSec OnUnitInactiveSec Persistent RandomizedDelaySec
    RemainAfterElapse Unit WakeSystem
    """.split()
)
SECTION_KEYS = {
    "Unit": UNIT_KEYS,
    "Service": SERVICE_ONLY_KEYS | EXEC_KEYS | KILL_KEYS | CGROUP_KEYS,
    "Timer": TIMER_KEYS,
    "Slice": CGROUP_KEYS,
    "Install": INSTALL_KEYS,
}

# Deprecated or removed, with where and when: every one still loads on systemd 255, and every one
# is a setting systemd has asked units to stop using (NEWS; systemd-analyze verify; v258's
# load-fragment table ignores the cgroup-v1 ones outright).
CGROUP_V1 = "deprecated in systemd 252, ignored from 258 (cgroup v1 removed)"
DEPRECATED = {
    ("Service", "MemoryLimit"): f"{CGROUP_V1}; use MemoryMax=",
    ("Service", "CPUShares"): f"{CGROUP_V1}; use CPUWeight=",
    ("Service", "StartupCPUShares"): f"{CGROUP_V1}; use StartupCPUWeight=",
    ("Service", "BlockIOAccounting"): f"{CGROUP_V1}; use IOAccounting=",
    ("Service", "BlockIOWeight"): f"{CGROUP_V1}; use IOWeight=",
    ("Service", "StartupBlockIOWeight"): f"{CGROUP_V1}; use StartupIOWeight=",
    ("Service", "BlockIODeviceWeight"): f"{CGROUP_V1}; use IODeviceWeight=",
    ("Service", "BlockIOReadBandwidth"): f"{CGROUP_V1}; use IOReadBandwidthMax=",
    ("Service", "BlockIOWriteBandwidth"): f"{CGROUP_V1}; use IOWriteBandwidthMax=",
    ("Slice", "MemoryLimit"): f"{CGROUP_V1}; use MemoryMax=",
    ("Slice", "CPUShares"): f"{CGROUP_V1}; use CPUWeight=",
    (
        "Service",
        "PermissionsStartOnly",
    ): "deprecated in systemd 240; prefix the command with +",
    (
        "Service",
        "StartLimitInterval",
    ): "moved to [Unit] StartLimitIntervalSec= in systemd 229",
    ("Service", "StartLimitBurst"): "moved to [Unit] in systemd 229",
    ("Service", "StartLimitAction"): "moved to [Unit] in systemd 229",
    ("Service", "FailureAction"): "moved to [Unit] in systemd 236",
    ("Service", "RebootArgument"): "moved to [Unit] in systemd 229",
    ("Unit", "StartLimitInterval"): "renamed StartLimitIntervalSec= in systemd 230",
    ("Unit", "IgnoreOnSnapshot"): "removed; systemd ignores it",
    (
        "Service",
        "Capabilities",
    ): "removed; systemd ignores it (use CapabilityBoundingSet=)",
    ("Service", "NetClass"): "removed; systemd ignores it",
    ("Service", "SysVStartPriority"): "removed; systemd ignores it",
    ("Service", "BusPolicy"): "removed; systemd ignores it",
}
OBSOLETE_OUTPUT = {"syslog", "syslog+console"}

LONG_RUNNING_KINDS = {
    "simple",
    "exec",
    "notify",
    "notify-reload",
    "forking",
    "dbus",
    "idle",
}
RESTART_RECOVERS = {"on-failure", "on-abnormal", "on-watchdog", "on-abort", "always"}
NOTIFY_TYPES = {"notify", "notify-reload"}
DEFAULT_START_LIMIT_INTERVAL = 10.0  # systemd-system.conf DefaultStartLimitIntervalSec=
DEFAULT_START_LIMIT_BURST = 5  # systemd-system.conf DefaultStartLimitBurst=
DEFAULT_RESTART_SEC = 0.1  # systemd.service(5) RestartSec= "Defaults to 100ms"
TRUE = {"1", "yes", "y", "true", "t", "on"}
FALSE = {"0", "no", "n", "false", "f", "off"}

SECRET_NAME = re.compile(
    r"(?:^|_)(?:TOKEN|SECRET|PASSWORD|PASSWD|APIKEY|API_KEY|PRIVATE_KEY|ACCESS_KEY|SIGNING_KEY|"
    r"CREDENTIALS?|DSN)(?:$|_)",
    re.IGNORECASE,
)
PLACEHOLDER = re.compile(
    r"change[-_]?me|placeholder|example|replace[-_]?me|redacted|dummy|\bfake\b|\bunset\b|"
    r"\bnone\b|^your[-_]|secret[-_]?manager|set[-_]in[-_]",
    re.IGNORECASE,
)
# A name that says its value NAMES a secret in a secret store (the house `*_SECRET=<secret id>`
# convention), and the shape such an id has: readable words joined by - _ or ., few digits.
REFERENCE_NAME = re.compile(r"_SECRET(?:_NAME|_ID|_REF)?$", re.IGNORECASE)
REFERENCE_VALUE = re.compile(r"^[A-Za-z][A-Za-z0-9]*(?:[-_.][A-Za-z0-9]+)*$")
LITESTREAM_SECRET_KEYS = {
    "access-key-id", "secret-access-key", "password", "account-key", "key", "passphrase",
}  # fmt: skip
REMOTE_SCHEMES = {
    "s3",
    "gs",
    "gcs",
    "abs",
    "sftp",
    "nats",
    "webdav",
    "oss",
    "alioss",
    "wasabi",
}


# ---------------------------------------------------------------------------
# Values


def truthy(value: str | None) -> bool | None:
    """systemd's boolean spelling (systemd.syntax(7)), or None for anything else."""
    if value is None:
        return None
    lowered = value.strip().lower()
    if lowered in TRUE:
        return True
    if lowered in FALSE:
        return False
    return None


TIME_UNITS = {
    "us": 1e-6, "usec": 1e-6, "ms": 1e-3, "msec": 1e-3, "s": 1.0, "sec": 1.0, "second": 1.0,
    "seconds": 1.0, "m": 60.0, "min": 60.0, "minute": 60.0, "minutes": 60.0, "h": 3600.0,
    "hr": 3600.0, "hour": 3600.0, "hours": 3600.0, "d": 86400.0, "day": 86400.0,
    "days": 86400.0, "w": 604800.0, "week": 604800.0, "weeks": 604800.0, "M": 2629800.0,
    "month": 2629800.0, "months": 2629800.0, "y": 31557600.0, "year": 31557600.0,
    "years": 31557600.0,
}  # fmt: skip
TIME_PART = re.compile(r"(\d+(?:\.\d+)?)\s*([A-Za-z]*)")


def seconds(value: str | None) -> float | None:
    """A systemd time span (systemd.time(7)) in seconds; inf for `infinity`; None when unreadable."""
    if value is None:
        return None
    text = value.strip()
    if text == "infinity":
        return float("inf")
    if not text:
        return None
    total = 0.0
    position = 0
    for match in TIME_PART.finditer(text):
        if text[position : match.start()].strip():
            return None
        number, unit = match.groups()
        factor = (
            TIME_UNITS.get(unit if unit == "M" else unit.lower(), None) if unit else 1.0
        )
        if factor is None:
            return None
        total += float(number) * factor
        position = match.end()
    if text[position:].strip():
        return None
    return total


SIZE = re.compile(r"^(\d+(?:\.\d+)?)\s*([KMGTPE]?)$", re.IGNORECASE)


def size_bytes(value: str | None) -> int | None:
    """A byte size with a base-1024 suffix (systemd.resource-control(5)); None for a percentage,
    `infinity` or anything unreadable."""
    if value is None:
        return None
    match = SIZE.match(value.strip())
    if not match:
        return None
    number, suffix = match.groups()
    power = " KMGTPE".index(suffix.upper() or " ")
    return int(float(number) * (1024**power))


def is_placeholder(value: str) -> bool:
    """Whether an assigned value is a stand-in rather than a credential: empty, `<...>`, `$VAR`,
    only filler characters, or a placeholder word."""
    text = value.strip().strip("'\"")
    if not text:
        return True
    if (text.startswith("<") and text.endswith(">")) or text.startswith("$"):
        return True
    if set(text) <= set("xX*.-_ "):
        return True
    return bool(PLACEHOLDER.search(text))


def is_reference(name: str, value: str) -> bool:
    """Whether `name=value` names a secret held elsewhere rather than carrying it: the name says so
    (`*_SECRET`, `*_SECRET_NAME|_ID|_REF`) and the value is a readable id, few digits in it."""
    text = value.strip().strip("'\"")
    if not REFERENCE_NAME.search(name) or not REFERENCE_VALUE.match(text):
        return False
    return sum(char.isdigit() for char in text) * 10 < len(text) * 3


def literal_secret(name: str, value: str) -> bool:
    """A secret-named key carrying a value that is neither a placeholder nor a reference."""
    return (
        bool(SECRET_NAME.search(name))
        and not is_placeholder(value)
        and not is_reference(name, value)
    )


# ---------------------------------------------------------------------------
# Units


@dataclasses.dataclass(frozen=True, slots=True)
class Assignment:
    section: str
    key: str
    value: str
    line: int
    source: str


@dataclasses.dataclass(slots=True)
class Unit:
    """One unit file and its drop-ins, as systemd would read them."""

    name: str
    rel: str
    kind: str
    user: bool
    assignments: list[Assignment]
    sections: list[str]
    syntax: list[str]

    def assigned(self, section: str, key: str) -> bool:
        return any(a.section == section and a.key == key for a in self.assignments)

    def values(self, section: str, key: str) -> list[str]:
        """Every value in order, with systemd's reset rule: an empty assignment clears the list."""
        out: list[str] = []
        for assignment in self.assignments:
            if assignment.section == section and assignment.key == key:
                if assignment.value == "":
                    out = []
                else:
                    out.append(assignment.value)
        return out

    def last(self, section: str, key: str) -> str | None:
        found = [
            a.value for a in self.assignments if a.section == section and a.key == key
        ]
        return found[-1] if found else None

    def words(self, section: str, key: str) -> list[str]:
        return [word for value in self.values(section, key) for word in value.split()]

    def waived(self) -> dict[str, str]:
        """`X-DurableServices-Waive=<reason> <why>` assignments with a non-empty why."""
        out: dict[str, str] = {}
        for value in self.values("Unit", WAIVE_KEY):
            reason, _, why = value.strip().partition(" ")
            if reason and why.strip():
                out[reason] = why.strip()
        return out


def _logical_lines(text: str) -> Iterable[tuple[int, str]]:
    """systemd.syntax(7): comments start `#` or `;`, a trailing backslash joins the next line (a
    comment line inside the join is skipped), and every line keeps the number it started on."""
    pending: list[str] = []
    start = 0
    for number, raw in enumerate(text.splitlines(), start=1):
        stripped = raw.strip()
        if pending and stripped.startswith(("#", ";")):
            continue
        if not pending:
            start = number
        if stripped.endswith("\\"):
            pending.append(stripped[:-1])
            continue
        pending.append(stripped)
        yield start, " ".join(part for part in pending if part).strip()
        pending = []
    if pending:
        yield start, " ".join(pending).strip()


def _read_into(unit: Unit, path: Path, source: str) -> None:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        unit.syntax.append(f"{source}: unreadable ({error})")
        return
    section: str | None = None
    for number, line in _logical_lines(text):
        if not line or line.startswith(("#", ";")):
            continue
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1].strip()
            if section not in SECTIONS[unit.kind] and not section.startswith("X-"):
                unit.syntax.append(f"{source}:{number}: unknown section [{section}]")
            if section not in unit.sections:
                unit.sections.append(section)
            continue
        key, equals, value = line.partition("=")
        key = key.strip()
        if not equals or not KEY_NAME.match(key):
            unit.syntax.append(
                f"{source}:{number}: not a Key=Value line: {line[:60]!r}"
            )
            continue
        if section is None:
            unit.syntax.append(f"{source}:{number}: {key}= before any [section]")
            continue
        unit.assignments.append(Assignment(section, key, value.strip(), number, source))


def parse_unit(root: Path, path: Path) -> Unit:
    rel = path.relative_to(root).as_posix()
    unit = Unit(
        name=path.name,
        rel=rel,
        kind=UNIT_KINDS[path.suffix],
        user="user" in path.relative_to(root / DEPLOY).parts[:-1],
        assignments=[],
        sections=[],
        syntax=[],
    )
    if not UNIT_NAME.match(path.name) or len(path.name) > 255:
        unit.syntax.append(f"{rel}: {path.name!r} is not a valid unit name")
    _read_into(unit, path, rel)
    for dropin in sorted((path.parent / f"{path.name}.d").glob("*.conf")):
        _read_into(unit, dropin, dropin.relative_to(root).as_posix())
    return unit


def service_type(unit: Unit) -> str:
    """Type= as systemd.service(5) defaults it: dbus with BusName=, simple with ExecStart=, else
    oneshot."""
    declared = unit.last("Service", "Type")
    if declared:
        return declared.strip()
    if unit.last("Service", "BusName"):
        return "dbus"
    if unit.values("Service", "ExecStart"):
        return "simple"
    return "oneshot"


def long_running(unit: Unit) -> bool:
    return unit.kind == "service" and service_type(unit) in LONG_RUNNING_KINDS


def exec_commands(unit: Unit) -> list[str]:
    keys = ("ExecStartPre", "ExecStart", "ExecStartPost", "ExecStop", "ExecStopPost")
    return [value for key in keys for value in unit.values("Service", key)]


# ---------------------------------------------------------------------------
# Litestream, env files, the host budget


def _yaml_scalar(text: str) -> object:
    text = text.strip()
    if text in ("[]", "{}"):
        return [] if text == "[]" else {}
    if len(text) >= 2 and text[0] == text[-1] and text[0] in "'\"":
        return text[1:-1]
    return text


def _strip_comment(line: str) -> str:
    quote = ""
    for index, char in enumerate(line):
        if quote:
            if char == quote:
                quote = ""
        elif char in "'\"":
            quote = char
        elif char == "#" and (index == 0 or line[index - 1] in " \t"):
            return line[:index].rstrip()
    return line.rstrip()


def parse_yaml_subset(text: str) -> object:
    """The block-style YAML subset a Litestream config uses: mappings, sequences of scalars or of
    mappings, quoted or plain scalars and comments. Anything else raises ValueError."""
    rows = []
    for raw in text.splitlines():
        if "\t" in raw[: len(raw) - len(raw.lstrip())]:
            raise ValueError("a tab in indentation")
        line = _strip_comment(raw)
        if line.strip():
            rows.append((len(line) - len(line.lstrip()), line.strip()))

    def block(index: int, indent: int) -> tuple[object, int]:
        if index >= len(rows):
            return None, index
        if rows[index][1].startswith("- ") or rows[index][1] == "-":
            items: list[object] = []
            while (
                index < len(rows)
                and rows[index][0] == indent
                and rows[index][1][:1] == "-"
            ):
                rest = rows[index][1][1:].strip()
                if not rest:
                    value, index = block(index + 1, rows[index + 1][0])
                    items.append(value)
                    continue
                if re.match(r"^[^'\"\s][^:]*:(\s|$)", rest):
                    rows[index] = (indent + 2, rest)
                    value, index = block(index, indent + 2)
                    items.append(value)
                    continue
                items.append(_yaml_scalar(rest))
                index += 1
            return items, index
        mapping: dict[str, object] = {}
        while index < len(rows) and rows[index][0] == indent:
            key, colon, rest = rows[index][1].partition(":")
            if not colon or rows[index][1].startswith("- "):
                raise ValueError(f"not a mapping entry: {rows[index][1]!r}")
            index += 1
            if rest.strip():
                mapping[key.strip()] = _yaml_scalar(rest)
            elif index < len(rows) and rows[index][0] > indent:
                mapping[key.strip()], index = block(index, rows[index][0])
            elif (
                index < len(rows)
                and rows[index][0] == indent
                and rows[index][1][:1] == "-"
            ):
                mapping[key.strip()], index = block(index, indent)
            else:
                mapping[key.strip()] = None
        if index < len(rows) and rows[index][0] > indent:
            raise ValueError(f"unexpected indentation at {rows[index][1]!r}")
        return mapping, index

    value, index = block(0, rows[0][0] if rows else 0)
    if index != len(rows):
        raise ValueError(f"unparsed from {rows[index][1]!r}")
    return value


@dataclasses.dataclass(frozen=True, slots=True)
class Litestream:
    rel: str
    document: dict[str, object] | None
    error: str | None

    def databases(self) -> list[dict[str, object]]:
        if not self.document:
            return []
        dbs = self.document.get("dbs")
        return (
            [db for db in dbs if isinstance(db, dict)] if isinstance(dbs, list) else []
        )


def load_litestream(root: Path, path: Path) -> Litestream:
    rel = path.relative_to(root).as_posix()
    try:
        document = parse_yaml_subset(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError) as error:
        return Litestream(rel, None, str(error))
    if not isinstance(document, dict):
        return Litestream(rel, None, "the top level is not a mapping")
    return Litestream(rel, document, None)


def replica_urls(db: dict[str, object]) -> list[str]:
    """The replica URLs one `dbs` entry names, in v0.5's `replica:` form and the legacy list."""
    found: list[str] = []
    candidates: list[object] = [db.get("replica")]
    legacy = db.get("replicas")
    if isinstance(legacy, list):
        candidates.extend(legacy)
    for candidate in candidates:
        if isinstance(candidate, dict):
            url = candidate.get("url") or candidate.get("path")
            if isinstance(url, str) and url:
                found.append(url)
            elif candidate.get("type") and candidate.get("bucket"):
                found.append(f"{candidate['type']}://{candidate['bucket']}")
    return found


def env_assignments(path: Path) -> list[tuple[int, str, str]]:
    """`KEY=VALUE` lines of an env file, `export` tolerated, comments skipped."""
    out = []
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return out
    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith(("#", ";")) or "=" not in line:
            continue
        key, _, value = line.removeprefix("export ").partition("=")
        out.append((number, key.strip(), value.strip()))
    return out


@dataclasses.dataclass(frozen=True, slots=True)
class HostBudget:
    memory: int | None
    cpus: float | None
    error: str | None


def load_host_budget(path: Path) -> HostBudget | None:
    if not path.is_file():
        return None
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        return HostBudget(None, None, f"unreadable: {error}")
    if not isinstance(document, dict):
        return HostBudget(None, None, "not a JSON object")
    memory = size_bytes(str(document.get("memory", "")))
    cpus = document.get("cpus")
    return HostBudget(
        memory,
        float(cpus) if isinstance(cpus, int | float) else None,
        None if memory is not None else "memory is not a byte size such as 1400M",
    )


# ---------------------------------------------------------------------------
# The subject


@dataclasses.dataclass(slots=True)
class Subject:
    root: Path
    units: dict[str, Unit]
    litestream: list[Litestream]
    env_files: list[Path]
    scripts: dict[str, Path]
    host_budget: HostBudget | None
    journald: list[Path]
    drop_ins: int

    def of_kind(self, kind: str) -> list[Unit]:
        return [unit for unit in self.units.values() if unit.kind == kind]

    @property
    def services(self) -> list[Unit]:
        return self.of_kind("service")

    @property
    def timers(self) -> list[Unit]:
        return self.of_kind("timer")

    @property
    def system_services(self) -> list[Unit]:
        return [unit for unit in self.services if not unit.user]

    def examined(self) -> int:
        return len(self.services) + len(self.timers)

    def timer_target(self, timer: Unit) -> str:
        return (timer.last("Timer", "Unit") or "").strip() or timer.name[
            : -len(".timer")
        ] + (".service")

    def timer_activated(self) -> set[str]:
        return {self.timer_target(timer) for timer in self.timers}

    def pulled_in(self) -> set[str]:
        """Every unit another subject unit pulls in, so it starts without its own [Install]."""
        keys = ("Wants", "Requires", "BindsTo", "Upholds", "Requisite")
        return {
            name
            for unit in self.units.values()
            for key in keys
            for name in unit.words("Unit", key)
        }

    def script_text(self, unit: Unit) -> str:
        """The unit's commands plus every `deploy/` script they name by basename."""
        parts = list(exec_commands(unit))
        for command in exec_commands(unit):
            for word in command.replace('"', " ").replace("'", " ").split():
                script = self.scripts.get(Path(word).name)
                if script is not None:
                    parts.append(script.read_text(encoding="utf-8", errors="replace"))
        return "\n".join(parts)


def load_subject(root: Path) -> Subject:
    deploy = root / DEPLOY
    units: dict[str, Unit] = {}
    drop_ins = 0
    litestream: list[Litestream] = []
    env_files: list[Path] = []
    scripts: dict[str, Path] = {}
    journald: list[Path] = []
    if deploy.is_dir():
        for path in sorted(deploy.rglob("*")):
            if not path.is_file():
                continue
            parent = path.parent.name
            if parent.endswith(".d") and path.suffix == ".conf":
                if parent.endswith(tuple(f"{suffix}.d" for suffix in UNIT_KINDS)):
                    drop_ins += 1
                if parent == "journald.conf.d":
                    journald.append(path)
                continue
            if path.suffix in UNIT_KINDS:
                units[path.name] = parse_unit(root, path)
            elif path.name.startswith("litestream") and path.suffix in (
                ".yml",
                ".yaml",
            ):
                litestream.append(load_litestream(root, path))
            elif ".env" in path.name or path.name.startswith(".env"):
                env_files.append(path)
            elif path.suffix in (".sh", ".py", ".bash"):
                scripts[path.name] = path
    return Subject(
        root=root,
        units=units,
        litestream=litestream,
        env_files=env_files,
        scripts=scripts,
        host_budget=load_host_budget(deploy / "host-budget.json"),
        journald=journald,
        drop_ins=drop_ins,
    )


# ---------------------------------------------------------------------------
# Calendar expressions (systemd.time(7) "CALENDAR EVENTS"), measured against systemd-analyze 255

WEEKDAYS = {
    "mon": 0, "monday": 0, "tue": 1, "tuesday": 1, "wed": 2, "wednesday": 2, "thu": 3,
    "thursday": 3, "fri": 4, "friday": 4, "sat": 5, "saturday": 5, "sun": 6, "sunday": 6,
}  # fmt: skip
SHORTHANDS = {
    "minutely", "hourly", "daily", "monthly", "weekly", "yearly", "annually", "quarterly",
    "semiannually",
}  # fmt: skip
_ZONES: set[str] = set()


def _is_zone(token: str) -> bool:
    if not _ZONES:
        _ZONES.update(zoneinfo.available_timezones())
    return token.upper() == "UTC" or token in _ZONES


def _weekday_problem(token: str) -> str | None:
    for item in token.rstrip(",").split(","):
        if not item:
            return f"empty weekday in {token!r}"
        ends = re.split(r"\.\.|-", item)
        if len(ends) > 2 or not all(end.lower() in WEEKDAYS for end in ends):
            return f"{item!r} is not a weekday or a weekday range"
        if len(ends) == 2 and WEEKDAYS[ends[0].lower()] > WEEKDAYS[ends[1].lower()]:
            return f"weekday range {item!r} runs backwards"
    return None


def _chain_problem(
    text: str, low: int, high: int, what: str, fraction: bool = False
) -> str | None:
    number = r"\d+(?:\.\d+)?" if fraction else r"\d+"
    for item in text.split(","):
        match = re.fullmatch(rf"(\*|{number})(?:\.\.({number}))?(?:/({number}))?", item)
        if not match:
            return f"{what} {item!r} is not a value, range or repetition"
        start, end, step = match.groups()
        if step is not None and float(step) <= 0:
            return f"{what} repetition in {item!r} is not positive"
        if start == "*" and (end is not None or step is not None):
            return f"{what} {item!r}: * takes no range or repetition"
        for bound in (start, end):
            if bound in (None, "*"):
                continue
            value = float(bound)
            if what == "year" and len(bound) == 2:
                value += 1900 if value >= 70 else 2000
            if not low <= value < high + 1:
                return f"{what} {bound} is outside {low}..{high}"
        if start not in (None, "*") and end is not None and float(start) > float(end):
            return f"{what} range {item!r} runs backwards"
    return None


def _date_problem(token: str) -> str | None:
    last_day = "~" in token
    parts = re.split(r"[-~]", token)
    if len(parts) == 2:
        year, month, day = None, parts[0], parts[1]
    elif len(parts) == 3:
        year, month, day = parts
    else:
        return f"date {token!r} is not [YEAR-]MONTH-DAY"
    if last_day and not re.search(r"[0-9*]~", token):
        return f"date {token!r} misplaces ~"
    problems = [
        _chain_problem(year, 1970, 2199, "year") if year is not None else None,
        _chain_problem(month, 1, 12, "month"),
        _chain_problem(day, 1, 31, "day"),
    ]
    return next((problem for problem in problems if problem), None)


def _time_problem(token: str) -> str | None:
    parts = token.split(":")
    if len(parts) not in (2, 3):
        return f"time {token!r} is not HOUR:MINUTE[:SECOND]"
    problems = [
        _chain_problem(parts[0], 0, 23, "hour"),
        _chain_problem(parts[1], 0, 59, "minute"),
        _chain_problem(parts[2], 0, 59, "second", fraction=True)
        if len(parts) == 3
        else None,
    ]
    return next((problem for problem in problems if problem), None)


def calendar_problem(expression: str) -> str | None:
    """Why systemd 255 would refuse an OnCalendar= expression, or None when it parses."""
    tokens = expression.split()
    if not tokens:
        return "an empty expression"
    if (
        len(tokens) > 1
        and re.search(r"[A-Za-z]", tokens[-1])
        and not _weekday_like(tokens[-1])
    ):
        zone = tokens.pop()
        if tokens[-1].lower() not in SHORTHANDS and not _is_zone(zone):
            return f"{zone!r} is not UTC or an IANA time zone"
        if not _is_zone(zone):
            return f"{zone!r} is not UTC or an IANA time zone"
    if len(tokens) == 1 and tokens[0].lower() in SHORTHANDS:
        return None
    if tokens and re.match(r"^[A-Za-z]", tokens[0]):
        problem = _weekday_problem(tokens.pop(0))
        if problem:
            return problem
        if not tokens:
            return None
    if len(tokens) > 2:
        return f"{' '.join(tokens)!r} holds more than a date and a time"
    dates = [token for token in tokens if ":" not in token]
    times = [token for token in tokens if ":" in token]
    if len(dates) > 1 or len(times) > 1 or (tokens and tokens[-1] in dates and times):
        return f"{' '.join(tokens)!r} is not [DATE] [TIME]"
    for date in dates:
        if not re.search(r"[-~]", date):
            return f"{date!r} is neither a date nor a time"
        problem = _date_problem(date)
        if problem:
            return problem
    for time_spec in times:
        problem = _time_problem(time_spec)
        if problem:
            return problem
    return None


def _weekday_like(token: str) -> bool:
    return _weekday_problem(token) is None or token.lower() in SHORTHANDS


# ---------------------------------------------------------------------------
# The check catalog


@dataclasses.dataclass(frozen=True, slots=True)
class Finding:
    unit: str
    detail: str


@dataclasses.dataclass(frozen=True, slots=True)
class Check:
    id: str
    stage: str
    severity: str  # "blocking" | "advisory"
    reason: str
    source: str
    run: Callable[[Subject], list[Finding]]


CATALOG: list[Check] = []
STAGES = (
    "inventory", "directives", "service", "sandbox", "resources", "secrets", "logging", "timers",
    "backup",
)  # fmt: skip


def check(check_id: str, severity: str, reason: str, source: str):
    """Register one check; its stage is its id's first dotted part."""

    def register(function: Callable[[Subject], list[Finding]]):
        stage = check_id.split(".", 1)[0]
        assert stage in STAGES, check_id
        CATALOG.append(Check(check_id, stage, severity, reason, source, function))
        return function

    return register


def each(units: Iterable[Unit], test: Callable[[Unit], str | None]) -> list[Finding]:
    """A finding for every unit `test` names a problem for."""
    out = []
    for unit in units:
        detail = test(unit)
        if detail:
            out.append(Finding(unit.rel, detail))
    return out


# --- inventory --------------------------------------------------------------------------------


@check(
    "inventory.units-present",
    "blocking",
    "units-none",
    "SPEC-V2-1840: a guard that examined nothing refuses; the subject is deploy/*.service|timer",
)
def _units_present(subject: Subject) -> list[Finding]:
    if subject.examined():
        return []
    return [
        Finding(DEPLOY, f"no .service or .timer file under {subject.root / DEPLOY}")
    ]


@check(
    "inventory.syntax",
    "blocking",
    "unit-syntax",
    "systemd.syntax(7) and systemd.unit(5): sections, Key=Value lines and valid unit names; "
    "systemd ignores an unknown section",
)
def _syntax(subject: Subject) -> list[Finding]:
    return [
        Finding(unit.rel, problem)
        for unit in subject.units.values()
        for problem in unit.syntax
    ]


# --- directives -------------------------------------------------------------------------------


@check(
    "directives.known",
    "blocking",
    "key-unknown",
    "systemd 255 load-fragment table: an unknown or misplaced key is ignored with 'Unknown key "
    "name ... ignoring' (systemd-analyze verify)",
)
def _known(subject: Subject) -> list[Finding]:
    out = []
    for unit in subject.units.values():
        for a in unit.assignments:
            if a.key.startswith("X-") or a.section.startswith("X-"):
                continue
            allowed = SECTION_KEYS.get(a.section)
            if allowed is not None and a.key not in allowed:
                out.append(
                    Finding(
                        unit.rel,
                        f"{a.source}:{a.line}: {a.key}= is not a [{a.section}] key",
                    )
                )
    return out


@check(
    "directives.deprecated",
    "blocking",
    "directive-deprecated",
    "systemd NEWS 229/230/240/246/252 and the v258 load-fragment table; systemd-analyze verify "
    "warns on each",
)
def _deprecated(subject: Subject) -> list[Finding]:
    out = []
    for unit in subject.units.values():
        for a in unit.assignments:
            why = DEPRECATED.get((a.section, a.key))
            if why:
                out.append(Finding(unit.rel, f"{a.source}:{a.line}: {a.key}= {why}"))
            elif (
                a.key in ("StandardOutput", "StandardError")
                and a.value in OBSOLETE_OUTPUT
            ):
                out.append(
                    Finding(
                        unit.rel,
                        f"{a.source}:{a.line}: {a.key}={a.value} is obsolete since systemd 246 "
                        "(read as journal)",
                    )
                )
    return out


@check(
    "directives.absolute-paths",
    "blocking",
    "path-not-absolute",
    "systemd.exec(5): EnvironmentFile=, ReadWritePaths=, WorkingDirectory= take absolute paths; "
    "systemd-analyze verify ignores or refuses a relative one",
)
def _absolute(subject: Subject) -> list[Finding]:
    out = []
    path_keys = (
        "EnvironmentFile",
        "ReadWritePaths",
        "ReadOnlyPaths",
        "InaccessiblePaths",
    )
    for unit in subject.services:
        for key in path_keys:
            for word in unit.words("Service", key):
                path = word.lstrip("-+")
                if not path.startswith(("/", "%")):
                    out.append(Finding(unit.rel, f"{key}={word} is not absolute"))
        directory = unit.last("Service", "WorkingDirectory")
        if directory and not directory.lstrip("-").startswith(("/", "~", "%")):
            out.append(
                Finding(unit.rel, f"WorkingDirectory={directory} is not absolute")
            )
        for command in unit.values("Service", "ExecStart"):
            program = (
                command.lstrip("@-:+!").split()[0] if command.strip("@-:+!") else ""
            )
            if program and "/" in program and not program.startswith(("/", "%")):
                out.append(
                    Finding(unit.rel, f"ExecStart program {program} is a relative path")
                )
    return out


# --- service ----------------------------------------------------------------------------------


@check(
    "service.exec-start",
    "blocking",
    "execstart-missing",
    "systemd-analyze verify: 'Service has no ExecStart=, ExecStop=, or SuccessAction=. Refusing.'",
)
def _exec_start(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            None
            if u.values("Service", "ExecStart") or u.values("Service", "ExecStop")
            else "no ExecStart="
        ),
    )


@check(
    "service.type-forking",
    "blocking",
    "type-forking",
    "systemd.service(5) Type=: 'The use of this type is discouraged, use notify, notify-reload, "
    "or dbus instead.'",
)
def _forking(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: "Type=forking" if service_type(u) == "forking" else None,
    )


@check(
    "service.restart",
    "blocking",
    "restart-missing",
    "systemd.service(5) Restart=: 'on-failure is the recommended choice for long-running "
    "services'",
)
def _restart(subject: Subject) -> list[Finding]:
    def test(unit: Unit) -> str | None:
        if not long_running(unit):
            return None
        restart = (unit.last("Service", "Restart") or "no").strip()
        return (
            None if restart in RESTART_RECOVERS else f"Restart={restart} never recovers"
        )

    return each(subject.services, test)


@check(
    "service.oneshot-restart",
    "blocking",
    "oneshot-restart-rejected",
    "systemd.service(5) Restart=: 'always and on-success are rejected' for Type=oneshot; "
    "systemd-analyze verify refuses the unit",
)
def _oneshot_restart(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            f"Type=oneshot with Restart={u.last('Service', 'Restart')}"
            if service_type(u) == "oneshot"
            and (u.last("Service", "Restart") or "").strip() in ("always", "on-success")
            else None
        ),
    )


@check(
    "service.kill-mode",
    "blocking",
    "kill-mode-escapes",
    "systemd.kill(5): KillMode=process 'not recommended!', none 'strongly recommended against!'; "
    "systemd 246 warns on none, verify calls it deprecated",
)
def _kill_mode(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            f"KillMode={u.last('Service', 'KillMode')} lets processes escape the unit"
            if (u.last("Service", "KillMode") or "").strip() in ("process", "none")
            else None
        ),
    )


def _families(unit: Unit) -> tuple[bool, set[str]] | None:
    """RestrictAddressFamilies= as (allow_list, families); None when the unit never sets it."""
    values = unit.values("Service", "RestrictAddressFamilies")
    if not values:
        return None
    allow, families = True, set()
    for value in values:
        words = value.split()
        if words and words[0].startswith("~"):
            allow = False
            words[0] = words[0][1:]
        families.update(word for word in words if word)
    if families == {"none"}:
        return True, set()
    return allow, families


@check(
    "service.notify-socket",
    "blocking",
    "notify-socket-blocked",
    "sd_notify(3) sends an AF_UNIX datagram; systemd.exec(5) RestrictAddressFamilies= restricts "
    "socket(2), so Type=notify or WatchdogSec= without AF_UNIX can never report READY=1",
)
def _notify_socket(subject: Subject) -> list[Finding]:
    def test(unit: Unit) -> str | None:
        watchdog = seconds(unit.last("Service", "WatchdogSec")) or 0
        if service_type(unit) not in NOTIFY_TYPES and not watchdog:
            return None
        families = _families(unit)
        if families is None:
            return None
        allow, listed = families
        if (allow and "AF_UNIX" not in listed) or (not allow and "AF_UNIX" in listed):
            return (
                "the notify socket needs AF_UNIX, which RestrictAddressFamilies= denies"
            )
        return None

    return each(subject.services, test)


@check(
    "service.network-online",
    "blocking",
    "network-online-half",
    "systemd.special(7) network-online.target: pull it in 'via a Wants= type dependency and "
    "order themselves after it'",
)
def _network_online(subject: Subject) -> list[Finding]:
    target = "network-online.target"

    def test(unit: Unit) -> str | None:
        after = target in unit.words("Unit", "After")
        wanted = any(target in unit.words("Unit", key) for key in ("Wants", "Requires"))
        if after and not wanted:
            return "After=network-online.target without Wants= (nothing pulls the target in)"
        if wanted and not after:
            return "Wants=network-online.target without After= (started in parallel with it)"
        return None

    return each(subject.services, test)


def _start_limit(unit: Unit) -> tuple[float, int, float]:
    interval = seconds(unit.last("Unit", "StartLimitIntervalSec"))
    burst = unit.last("Unit", "StartLimitBurst")
    restart = seconds(unit.last("Service", "RestartSec"))
    return (
        DEFAULT_START_LIMIT_INTERVAL if interval is None else interval,
        DEFAULT_START_LIMIT_BURST
        if not (burst or "").strip().isdigit()
        else int(burst or "0"),
        DEFAULT_RESTART_SEC if restart is None else restart,
    )


def _never_trips(unit: Unit) -> str | None:
    """systemd.unit(5): a unit started more than `burst` times in `interval` is refused. A crash
    loop starts once every RestartSec at best, so burst x RestartSec past the interval never
    reaches `failed` (the arithmetic v9's ADR-086 C2 corrected)."""
    if not long_running(unit):
        return None
    restart = (unit.last("Service", "Restart") or "no").strip()
    if restart not in RESTART_RECOVERS:
        return None
    interval, burst, restart_sec = _start_limit(unit)
    if interval == 0 or burst * restart_sec <= interval:
        return None
    return (
        f"StartLimitBurst={burst} x RestartSec={restart_sec:g}s = {burst * restart_sec:g}s > "
        f"StartLimitIntervalSec={interval:g}s: a crash loop never reaches failed"
    )


@check(
    "service.on-failure-inert",
    "blocking",
    "on-failure-inert",
    "systemd.unit(5) OnFailure= fires on 'failed', and StartLimit* is the only road there for a "
    "restarting unit; a limit that cannot trip leaves OnFailure= provably inert",
)
def _on_failure_inert(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: _never_trips(u) if u.values("Unit", "OnFailure") else None,
    )


@check(
    "service.installable",
    "blocking",
    "install-missing",
    "systemctl(1) enable reads [Install]: a long-running service no subject unit pulls in, with "
    "no WantedBy=/RequiredBy=, never starts at boot",
)
def _installable(subject: Subject) -> list[Finding]:
    pulled = subject.pulled_in() | subject.timer_activated()

    def test(unit: Unit) -> str | None:
        if not long_running(unit) or unit.name in pulled:
            return None
        if any(
            unit.values("Install", key)
            for key in ("WantedBy", "RequiredBy", "UpheldBy")
        ):
            return None
        return "no [Install] WantedBy= and nothing in the subject pulls it in"

    return each(subject.services, test)


@check(
    "service.type-exec",
    "advisory",
    "type-simple",
    "systemd.service(5) Type=: 'It is recommended to use Type=exec for long-running services' "
    "(notify when the service can report readiness)",
)
def _type_exec(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            "Type=simple reports success before execve()"
            if service_type(u) == "simple"
            else None
        ),
    )


@check(
    "service.restart-sec",
    "advisory",
    "restart-sec-default",
    "systemd.service(5) RestartSec= 'Defaults to 100ms': a crash loop hammers a small host",
)
def _restart_sec(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            "RestartSec= unset (100ms)"
            if long_running(u) and u.last("Service", "RestartSec") is None
            else None
        ),
    )


@check(
    "service.start-limit",
    "advisory",
    "start-limit-never-trips",
    "systemd.unit(5) StartLimitIntervalSec=/StartLimitBurst=: size them so a crash loop stops",
)
def _start_limit_trips(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: None if u.values("Unit", "OnFailure") else _never_trips(u),
    )


@check(
    "service.on-failure",
    "advisory",
    "on-failure-missing",
    "systemd.unit(5) OnFailure=: page someone when a daemon or a scheduled job reaches failed",
)
def _on_failure(subject: Subject) -> list[Finding]:
    scheduled = subject.timer_activated()
    return each(
        subject.system_services,
        lambda u: (
            "no OnFailure="
            if (long_running(u) or u.name in scheduled)
            and not u.values("Unit", "OnFailure")
            else None
        ),
    )


@check(
    "service.timeout-stop",
    "advisory",
    "timeout-stop-default",
    "systemd.service(5) TimeoutStopSec= defaults to DefaultTimeoutStopSec (90s); set the drain",
)
def _timeout_stop(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            "TimeoutStopSec= unset"
            if long_running(u)
            and u.last("Service", "TimeoutStopSec") is None
            and u.last("Service", "TimeoutSec") is None
            else None
        ),
    )


@check(
    "service.timeout-stop-finite",
    "advisory",
    "timeout-stop-infinite",
    "systemd.service(5) TimeoutStopSec=infinity disables the SIGKILL escalation, so a hung stop "
    "holds the shutdown",
)
def _timeout_finite(subject: Subject) -> list[Finding]:
    def test(unit: Unit) -> str | None:
        for key in ("TimeoutStopSec", "TimeoutSec"):
            value = unit.last("Service", key)
            if value is not None and seconds(value) in (float("inf"), 0.0):
                return f"{key}={value} never escalates to SIGKILL"
        return None

    return each(subject.services, test)


@check(
    "service.watchdog",
    "advisory",
    "watchdog-missing",
    "systemd.service(5) WatchdogSec= with sd_notify WATCHDOG=1: liveness a crashed-but-running "
    "daemon cannot fake",
)
def _watchdog(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            "no WatchdogSec="
            if long_running(u) and not seconds(u.last("Service", "WatchdogSec"))
            else None
        ),
    )


@check(
    "service.watchdog-notify",
    "advisory",
    "watchdog-without-notify",
    "systemd.service(5) WatchdogSec=: 'The service must call sd_notify(3) regularly with "
    "WATCHDOG=1'; Type=notify is how a service that does so says it is ready",
)
def _watchdog_notify(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            f"WatchdogSec= on Type={service_type(u)}"
            if seconds(u.last("Service", "WatchdogSec"))
            and service_type(u) not in NOTIFY_TYPES
            else None
        ),
    )


@check(
    "service.documentation",
    "advisory",
    "documentation-missing",
    "systemd.unit(5) Documentation=: 'first reference documentation that explains what the "
    "unit's purpose is'",
)
def _documentation(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            "no Documentation="
            if long_running(u) and not u.values("Unit", "Documentation")
            else None
        ),
    )


@check(
    "service.description",
    "advisory",
    "description-missing",
    "systemd.unit(5) Description=: the name systemctl and the journal show",
)
def _description(subject: Subject) -> list[Finding]:
    return each(
        subject.units.values(),
        lambda u: "no Description=" if not u.last("Unit", "Description") else None,
    )


# --- sandbox (system units only: systemd.exec(5) says the namespacing ones need privilege) ------


def _dynamic(unit: Unit) -> bool:
    return truthy(unit.last("Service", "DynamicUser")) is True


def _protect_system(unit: Unit) -> str:
    if _dynamic(unit):
        return "strict"
    value = (unit.last("Service", "ProtectSystem") or "no").strip().lower()
    return "yes" if truthy(value) is True else value


def _protect_home(unit: Unit) -> str:
    if _dynamic(unit):
        return "read-only"
    value = (unit.last("Service", "ProtectHome") or "no").strip().lower()
    return "yes" if truthy(value) is True else value


def _syscall_allow_list(unit: Unit) -> bool:
    values = unit.values("Service", "SystemCallFilter")
    return bool(values) and not values[0].lstrip().startswith("~")


@check(
    "sandbox.identity",
    "blocking",
    "runs-as-root",
    "systemd.exec(5) User=: 'the default is root' for system services; NIST SP 800-53 AC-6 least "
    "privilege; systemd-analyze security weighs User=/DynamicUser= highest",
)
def _identity(subject: Subject) -> list[Finding]:
    def test(unit: Unit) -> str | None:
        if _dynamic(unit):
            return None
        user = (unit.last("Service", "User") or "").strip()
        if user in ("", "root", "0"):
            return f"runs as root ({'User=' + user if user else 'no User='})"
        return None

    return each(subject.system_services, test)


@check(
    "sandbox.protect-system",
    "blocking",
    "protect-system-off",
    "systemd.exec(5) ProtectSystem=: 'recommended to enable this setting for all long-running "
    "services'",
)
def _protect_system_check(subject: Subject) -> list[Finding]:
    return each(
        subject.system_services,
        lambda u: (
            "ProtectSystem= off"
            if long_running(u) and _protect_system(u) not in ("yes", "full", "strict")
            else None
        ),
    )


@check(
    "sandbox.protect-home",
    "blocking",
    "protect-home-off",
    "systemd.exec(5) ProtectHome=: 'recommended to enable this setting for all long-running "
    "services (in particular network-facing ones)'",
)
def _protect_home_check(subject: Subject) -> list[Finding]:
    return each(
        subject.system_services,
        lambda u: (
            "ProtectHome= off"
            if long_running(u) and _protect_home(u) not in ("yes", "read-only", "tmpfs")
            else None
        ),
    )


@check(
    "sandbox.syscall-filter",
    "blocking",
    "syscall-allowlist-missing",
    "systemd.exec(5) SystemCallFilter=: 'recommended to enforce system call allow lists for all "
    "long-running system services' (@system-service)",
)
def _syscall_filter(subject: Subject) -> list[Finding]:
    return each(
        subject.system_services,
        lambda u: (
            "no SystemCallFilter= allow list"
            if long_running(u) and not _syscall_allow_list(u)
            else None
        ),
    )


def _flag_check(
    check_id: str, key: str, reason: str, source: str, accept: Callable[[str], bool]
):
    """An advisory hardening directive, judged on every system service."""

    def run(subject: Subject) -> list[Finding]:
        return each(
            subject.system_services,
            lambda u: None if accept_unit(u) else f"{key}= not set",
        )

    def accept_unit(unit: Unit) -> bool:
        value = unit.last("Service", key)
        return value is not None and accept(value.strip())

    check(check_id, "advisory", reason, source)(run)


def _yes(value: str) -> bool:
    return truthy(value) is True


_flag_check(
    "sandbox.no-new-privileges", "NoNewPrivileges", "no-new-privileges-off",
    "systemd.exec(5) NoNewPrivileges=: 'the simplest and most effective way to ensure that a "
    "process and its children can never elevate privileges'", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.private-tmp", "PrivateTmp", "private-tmp-off",
    "systemd.exec(5) PrivateTmp=: private /tmp and /var/tmp", lambda v: v.lower() != "no"
    and truthy(v) is not False,
)  # fmt: skip
_flag_check(
    "sandbox.private-devices", "PrivateDevices", "private-devices-off",
    "systemd.exec(5) PrivateDevices=: no physical devices", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.syscall-architectures", "SystemCallArchitectures", "syscall-architectures-unset",
    "systemd.exec(5): 'recommended to combine this option with SystemCallArchitectures=native'",
    lambda v: "native" in v.split(),
)  # fmt: skip
_flag_check(
    "sandbox.kernel-tunables", "ProtectKernelTunables", "protect-kernel-tunables-off",
    "systemd.exec(5) ProtectKernelTunables=: 'recommended to turn this on for most services'", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.kernel-modules", "ProtectKernelModules", "protect-kernel-modules-off",
    "systemd.exec(5) ProtectKernelModules=: 'recommended to turn this on for most services'", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.kernel-logs", "ProtectKernelLogs", "protect-kernel-logs-off",
    "systemd.exec(5) ProtectKernelLogs=: 'recommended to turn this on for most services'", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.control-groups", "ProtectControlGroups", "protect-control-groups-off",
    "systemd.exec(5) ProtectControlGroups=: 'recommended to turn this on for most services'", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.clock", "ProtectClock", "protect-clock-off",
    "systemd.exec(5) ProtectClock=: 'recommended to turn this on for most services'", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.hostname", "ProtectHostname", "protect-hostname-off",
    "systemd.exec(5) ProtectHostname=; systemd-analyze security scores it", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.namespaces", "RestrictNamespaces", "restrict-namespaces-off",
    "systemd.exec(5) RestrictNamespaces=; systemd-analyze security scores each namespace", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.realtime", "RestrictRealtime", "restrict-realtime-off",
    "systemd.exec(5) RestrictRealtime=: realtime scheduling can starve a small host", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.suid-sgid", "RestrictSUIDSGID", "restrict-suid-sgid-off",
    "systemd.exec(5) RestrictSUIDSGID=: 'recommended to restrict creation of SUID/SGID files'",
    _yes,
)  # fmt: skip
_flag_check(
    "sandbox.personality", "LockPersonality", "lock-personality-off",
    "systemd.exec(5) LockPersonality=; systemd-analyze security scores it", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.write-execute", "MemoryDenyWriteExecute", "memory-deny-write-execute-off",
    "systemd.exec(5) MemoryDenyWriteExecute= (waive it for a JIT runtime)", _yes,
)  # fmt: skip
_flag_check(
    "sandbox.proc", "ProtectProc", "protect-proc-off",
    "systemd.exec(5) ProtectProc=invisible; systemd-analyze security scores it",
    lambda v: v in ("invisible", "noaccess", "ptraceable"),
)  # fmt: skip
_flag_check(
    "sandbox.proc-subset", "ProcSubset", "proc-subset-off",
    "systemd.exec(5) ProcSubset=pid; systemd-analyze security scores it", lambda v: v == "pid",
)  # fmt: skip
_flag_check(
    "sandbox.umask", "UMask", "umask-permissive",
    "systemd.exec(5) UMask= 'Defaults to 0022': world-readable files; systemd-analyze security "
    "scores it", lambda v: v.isdigit() and int(v, 8) & 0o007 == 0o007,
)  # fmt: skip


@check(
    "sandbox.protect-system-strict",
    "advisory",
    "protect-system-not-strict",
    "systemd.exec(5) ProtectSystem=strict: the whole hierarchy read-only but ReadWritePaths=",
)
def _strict(subject: Subject) -> list[Finding]:
    # A daemon with ProtectSystem= off is sandbox.protect-system's blocking finding; this advisory
    # is the stronger setting, so for a daemon it speaks only when the protection is on but weaker.
    def test(unit: Unit) -> str | None:
        level = _protect_system(unit)
        if level == "strict" or (long_running(unit) and level not in ("yes", "full")):
            return None
        return f"ProtectSystem={level}"

    return each(subject.system_services, test)


@check(
    "sandbox.protect-home-all",
    "advisory",
    "protect-home-unset",
    "systemd.exec(5) ProtectHome=: recommended for every service that needs no user data",
)
def _home_all(subject: Subject) -> list[Finding]:
    return each(
        subject.system_services,
        lambda u: (
            "ProtectHome= off"
            if not long_running(u)
            and _protect_home(u) not in ("yes", "read-only", "tmpfs")
            else None
        ),
    )


@check(
    "sandbox.syscall-filter-all",
    "advisory",
    "syscall-filter-unset",
    "systemd.exec(5) SystemCallFilter=@system-service: 'a relatively safe basic choice for the "
    "majority of system services'",
)
def _syscall_all(subject: Subject) -> list[Finding]:
    return each(
        subject.system_services,
        lambda u: (
            "no SystemCallFilter= allow list"
            if not long_running(u) and not _syscall_allow_list(u)
            else None
        ),
    )


@check(
    "sandbox.capabilities",
    "advisory",
    "capability-bounding-set-unset",
    "systemd.exec(5) CapabilityBoundingSet=: 'If this option is not used ... no limits on the "
    "capabilities of the process are enforced'",
)
def _capabilities(subject: Subject) -> list[Finding]:
    return each(
        subject.system_services,
        lambda u: (
            "CapabilityBoundingSet= unset"
            if not u.assigned("Service", "CapabilityBoundingSet")
            else None
        ),
    )


@check(
    "sandbox.ambient-capabilities",
    "advisory",
    "ambient-capabilities-set",
    "systemd.exec(5) AmbientCapabilities=: every capability a non-root service holds is one to "
    "justify",
)
def _ambient(subject: Subject) -> list[Finding]:
    return each(
        subject.system_services,
        lambda u: (
            f"AmbientCapabilities={' '.join(u.words('Service', 'AmbientCapabilities'))}"
            if u.words("Service", "AmbientCapabilities")
            else None
        ),
    )


@check(
    "sandbox.address-families",
    "advisory",
    "address-families-unrestricted",
    "systemd.exec(5) RestrictAddressFamilies=: 'limit exposure of processes to remote access, in "
    "particular via exotic and sensitive network protocols'",
)
def _families_check(subject: Subject) -> list[Finding]:
    return each(
        subject.system_services,
        lambda u: "RestrictAddressFamilies= unset" if _families(u) is None else None,
    )


@check(
    "sandbox.state-directory",
    "advisory",
    "state-directory-preferred",
    "systemd.exec(5) StateDirectory=/LogsDirectory=: 'Using these options is recommended' over "
    "hand-made writable paths",
)
def _state_directory(subject: Subject) -> list[Finding]:
    def test(unit: Unit) -> str | None:
        for word in unit.words("Service", "ReadWritePaths"):
            path = word.lstrip("-+")
            if path.startswith(("/var/lib/", "/var/log/", "/var/cache/")):
                return f"ReadWritePaths={path} is what StateDirectory=/LogsDirectory= manage"
        return None

    return each(subject.system_services, test)


# --- resources --------------------------------------------------------------------------------


@check(
    "resources.memory-order",
    "blocking",
    "memory-high-not-below-max",
    "systemd.resource-control(5): 'use MemoryHigh= as the main control mechanism and use "
    "MemoryMax= as the last line of defense'; a MemoryHigh= at or above MemoryMax= never throttles",
)
def _memory_order(subject: Subject) -> list[Finding]:
    def test(unit: Unit) -> str | None:
        section = "Slice" if unit.kind == "slice" else "Service"
        high = size_bytes(unit.last(section, "MemoryHigh"))
        ceiling = size_bytes(unit.last(section, "MemoryMax"))
        if high is not None and ceiling is not None and high >= ceiling:
            return f"MemoryHigh={unit.last(section, 'MemoryHigh')} >= MemoryMax"
        return None

    return each([u for u in subject.units.values() if u.kind != "timer"], test)


def _budget_problem(subject: Subject) -> str | None:
    budget = subject.host_budget
    assert budget is not None and budget.memory is not None
    services = subject.system_services
    slices = {
        (unit.last("Service", "Slice") or "").strip()
        for unit in services
        if unit.kind == "service"
    }
    if len(slices) == 1 and "" not in slices:
        stack = subject.units.get(slices.pop())
        cap = size_bytes(stack.last("Slice", "MemoryMax")) if stack else None
        if cap is not None and cap <= budget.memory:
            return None
    total = 0
    unbounded = []
    oneshot_peak = 0
    for unit in services:
        ceiling = size_bytes(unit.last("Service", "MemoryMax"))
        if ceiling is None:
            unbounded.append(unit.name)
        elif long_running(unit):
            total += ceiling
        else:
            oneshot_peak = max(oneshot_peak, ceiling)
    if unbounded:
        return f"no MemoryMax= (and no capped subject slice) on {', '.join(unbounded)}"
    total += oneshot_peak
    if total > budget.memory:
        return f"daemons plus the largest job may use {total} bytes > the budget {budget.memory}"
    return None


@check(
    "resources.budget",
    "blocking",
    "memory-budget-exceeded",
    "deploy/host-budget.json is the subject's own declared share of a shared host; every ceiling "
    "must fit it, or the kernel's global OOM killer decides instead of each unit's own",
)
def _budget(subject: Subject) -> list[Finding]:
    budget = subject.host_budget
    if budget is None:
        return []
    if budget.error:
        return [Finding("deploy/host-budget.json", budget.error)]
    problem = _budget_problem(subject)
    return [Finding("deploy/host-budget.json", problem)] if problem else []


def _long_running_needs(key: str) -> Callable[[Subject], list[Finding]]:
    def run(subject: Subject) -> list[Finding]:
        return each(
            subject.system_services,
            lambda u: (
                f"no {key}=" if long_running(u) and not u.last("Service", key) else None
            ),
        )

    return run


@check(
    "resources.memory-max",
    "advisory",
    "memory-max-missing",
    "systemd.resource-control(5) MemoryMax=: the last line of defense, inside the unit's cgroup; "
    "a scheduled job's spike needs one too (i345: a daily render job with none)",
)
def _memory_max(subject: Subject) -> list[Finding]:
    return each(
        subject.system_services,
        lambda u: "no MemoryMax=" if not u.last("Service", "MemoryMax") else None,
    )


check(
    "resources.memory-high",
    "advisory",
    "memory-high-missing",
    "systemd.resource-control(5) MemoryHigh=: 'This is the main mechanism to control memory usage "
    "of a unit'",
)(_long_running_needs("MemoryHigh"))
check(
    "resources.tasks-max",
    "advisory",
    "tasks-max-missing",
    "systemd.resource-control(5) TasksMax=: a bounded process count (fork bombs, thread leaks)",
)(_long_running_needs("TasksMax"))
check(
    "resources.cpu-quota",
    "advisory",
    "cpu-quota-missing",
    "systemd.resource-control(5) CPUQuota=: one daemon cannot take a 2-vCPU host from the stacks "
    "it shares",
)(_long_running_needs("CPUQuota"))


@check(
    "resources.batch-priority",
    "advisory",
    "batch-priority-missing",
    "systemd.exec(5) Nice=, CPUSchedulingPolicy=, IOSchedulingClass=: a scheduled job yields to "
    "the daemons it shares a small host with",
)
def _batch(subject: Subject) -> list[Finding]:
    scheduled = subject.timer_activated()

    def test(unit: Unit) -> str | None:
        if unit.name not in scheduled or long_running(unit):
            return None
        nice = unit.last("Service", "Nice")
        if nice and nice.strip().lstrip("-").isdigit() and int(nice) > 0:
            return None
        if (unit.last("Service", "CPUSchedulingPolicy") or "") in ("idle", "batch"):
            return None
        if (unit.last("Service", "IOSchedulingClass") or "") in ("idle", "3"):
            return None
        return "no Nice=, idle/batch CPUSchedulingPolicy= or idle IOSchedulingClass="

    return each(subject.services, test)


@check(
    "resources.host-budget",
    "advisory",
    "host-budget-undeclared",
    "a shared small host needs its share written down: deploy/host-budget.json",
)
def _host_budget(subject: Subject) -> list[Finding]:
    if subject.host_budget is not None:
        return []
    return [
        Finding(
            "deploy/host-budget.json", "absent: no memory budget to hold ceilings to"
        )
    ]


# --- secrets ----------------------------------------------------------------------------------


def _environment_pairs(unit: Unit) -> list[tuple[str, str]]:
    pairs = []
    for value in unit.values("Service", "Environment"):
        try:
            words = shlex.split(value)
        except ValueError:
            words = value.split()
        for word in words:
            name, equals, assigned = word.partition("=")
            if equals:
                pairs.append((name, assigned))
    return pairs


@check(
    "secrets.environment-literal",
    "blocking",
    "secret-in-environment",
    "systemd.exec(5) Environment=: 'environment variables are not suitable for passing secrets "
    "... Use LoadCredential='; a literal in a committed unit is a leaked secret",
)
def _environment_literal(subject: Subject) -> list[Finding]:
    def test(unit: Unit) -> str | None:
        for name, value in _environment_pairs(unit):
            if literal_secret(name, value):
                return f"Environment= assigns {name} a literal"
        return None

    return each(subject.services, test)


@check(
    "secrets.env-file",
    "blocking",
    "secret-in-env-file",
    "a committed env file under deploy/ holds only placeholders for secret-named keys (the "
    "credential lives in the secret store)",
)
def _env_file(subject: Subject) -> list[Finding]:
    out = []
    for path in subject.env_files:
        rel = path.relative_to(subject.root).as_posix()
        for number, name, value in env_assignments(path):
            if literal_secret(name, value):
                out.append(Finding(rel, f"line {number}: {name} holds a literal"))
    return out


@check(
    "secrets.litestream-credential",
    "blocking",
    "litestream-credential-literal",
    "Litestream reference/config: credentials by ${VAR} expansion or ambient identity, never a "
    "literal in the committed file",
)
def _litestream_credential(subject: Subject) -> list[Finding]:
    out = []

    def walk(node: object, rel: str) -> None:
        if isinstance(node, dict):
            for key, value in node.items():
                if key in LITESTREAM_SECRET_KEYS and isinstance(value, str):
                    if not is_placeholder(value) and not value.startswith("/"):
                        out.append(Finding(rel, f"{key} holds a literal"))
                walk(value, rel)
        elif isinstance(node, list):
            for item in node:
                walk(item, rel)

    for config in subject.litestream:
        walk(config.document, config.rel)
    return out


@check(
    "secrets.environment-file-optional",
    "advisory",
    "environment-file-optional",
    "systemd.exec(5) EnvironmentFile=-: a missing file is skipped silently, so the service starts "
    "without its configuration",
)
def _environment_optional(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            "EnvironmentFile=- skips a missing file silently"
            if long_running(u)
            and any(w.startswith("-") for w in u.words("Service", "EnvironmentFile"))
            else None
        ),
    )


@check(
    "secrets.credentials",
    "advisory",
    "secrets-via-environment",
    "systemd.exec(5): 'Use LoadCredential=, LoadCredentialEncrypted= or SetCredentialEncrypted= "
    "... to pass data to unit processes securely'",
)
def _credentials(subject: Subject) -> list[Finding]:
    secret_keys = {
        name
        for path in subject.env_files
        for _, name, value in env_assignments(path)
        if SECRET_NAME.search(name) and not is_reference(name, value)
    }

    def test(unit: Unit) -> str | None:
        uses_credentials = any(
            unit.values("Service", key)
            for key in (
                "LoadCredential",
                "LoadCredentialEncrypted",
                "SetCredentialEncrypted",
            )
        )
        if uses_credentials:
            return None
        named = sorted(
            name for name, _ in _environment_pairs(unit) if SECRET_NAME.search(name)
        )
        if unit.values("Service", "EnvironmentFile") and secret_keys:
            named += sorted(secret_keys)
        return f"secrets via the environment: {', '.join(named)}" if named else None

    return each(subject.services, test)


# --- logging ----------------------------------------------------------------------------------


@check(
    "logging.journal",
    "advisory",
    "output-bypasses-journal",
    "systemd.exec(5) StandardOutput=file:/append:/truncate: bypasses journald's rotation and "
    "rate limits",
)
def _journal(subject: Subject) -> list[Finding]:
    def test(unit: Unit) -> str | None:
        for key in ("StandardOutput", "StandardError"):
            value = (unit.last("Service", key) or "").strip()
            if value.startswith(("file:", "append:", "truncate:")):
                return f"{key}={value}"
        return None

    return each(subject.services, test)


@check(
    "logging.identifier",
    "advisory",
    "syslog-identifier-missing",
    "systemd.exec(5) SyslogIdentifier=: one stable tag to filter a daemon's journal by",
)
def _identifier(subject: Subject) -> list[Finding]:
    return each(
        subject.services,
        lambda u: (
            "no SyslogIdentifier="
            if long_running(u) and not u.last("Service", "SyslogIdentifier")
            else None
        ),
    )


@check(
    "logging.journal-cap",
    "advisory",
    "journal-size-uncapped",
    "journald.conf(5) SystemMaxUse= defaults to 10% of the file system (up to 4G); a small disk "
    "wants a drop-in",
)
def _journal_cap(subject: Subject) -> list[Finding]:
    for path in subject.journald:
        text = path.read_text(encoding="utf-8", errors="replace")
        if re.search(r"(?m)^\s*SystemMaxUse\s*=\s*\S", text):
            return []
    return [Finding(f"{DEPLOY}/journald.conf.d", "no drop-in sets SystemMaxUse=")]


# --- timers -----------------------------------------------------------------------------------


TRIGGERS = (
    "OnCalendar", "OnActiveSec", "OnBootSec", "OnStartupSec", "OnUnitActiveSec",
    "OnUnitInactiveSec", "OnClockChange", "OnTimezoneChange",
)  # fmt: skip


@check(
    "timers.section",
    "blocking",
    "timer-section-missing",
    "systemd.timer(5): 'Timer unit files must include a [Timer] section'",
)
def _timer_section(subject: Subject) -> list[Finding]:
    return each(
        subject.timers, lambda u: None if "Timer" in u.sections else "no [Timer]"
    )


@check(
    "timers.trigger",
    "blocking",
    "timer-no-trigger",
    "systemd-analyze verify: 'Timer unit lacks value setting. Refusing.'",
)
def _trigger(subject: Subject) -> list[Finding]:
    return each(
        subject.timers,
        lambda u: (
            None
            if "Timer" not in u.sections
            or any(u.values("Timer", key) for key in TRIGGERS)
            else "no OnCalendar= or monotonic trigger"
        ),
    )


@check(
    "timers.target",
    "blocking",
    "timer-target-missing",
    "systemd.timer(5) Unit=: 'defaults to a service that has the same name as the timer'; a "
    "target the subject does not ship activates nothing",
)
def _timer_target(subject: Subject) -> list[Finding]:
    def test(timer: Unit) -> str | None:
        target = subject.timer_target(timer)
        if target in subject.units:
            return None
        base, _, suffix = target.partition("@")
        if suffix and f"{base}@.{suffix.rsplit('.', 1)[-1]}" in subject.units:
            return None
        return f"activates {target}, which deploy/ does not hold"

    return each(subject.timers, test)


@check(
    "timers.calendar",
    "blocking",
    "calendar-invalid",
    "systemd.time(7) CALENDAR EVENTS; systemd 255 ignores what it cannot parse ('Failed to parse "
    "calendar specification, ignoring')",
)
def _calendar(subject: Subject) -> list[Finding]:
    out = []
    for timer in subject.timers:
        for expression in timer.values("Timer", "OnCalendar"):
            problem = calendar_problem(expression)
            if problem:
                out.append(Finding(timer.rel, f"OnCalendar={expression}: {problem}"))
    return out


@check(
    "timers.persistent",
    "blocking",
    "persistent-without-calendar",
    "systemd.timer(5) Persistent=: 'this setting only has an effect on timers configured with "
    "OnCalendar='",
)
def _persistent(subject: Subject) -> list[Finding]:
    return each(
        subject.timers,
        lambda u: (
            "Persistent=true on a timer with no OnCalendar="
            if truthy(u.last("Timer", "Persistent"))
            and not u.values("Timer", "OnCalendar")
            else None
        ),
    )


@check(
    "timers.remain",
    "blocking",
    "timer-target-remains",
    "systemd.timer(5): a RemainAfterExit=yes service is 'only activated once, and then stay "
    "around forever'",
)
def _remain(subject: Subject) -> list[Finding]:
    def test(timer: Unit) -> str | None:
        target = subject.units.get(subject.timer_target(timer))
        if target is not None and truthy(target.last("Service", "RemainAfterExit")):
            return f"{target.name} has RemainAfterExit=yes: the timer fires it once"
        return None

    return each(subject.timers, test)


@check(
    "timers.installable",
    "blocking",
    "timer-install-missing",
    "systemctl(1) enable reads [Install]; a timer with no WantedBy= (timers.target) never starts",
)
def _timer_install(subject: Subject) -> list[Finding]:
    pulled = subject.pulled_in()
    return each(
        subject.timers,
        lambda u: (
            None
            if u.name in pulled
            or u.values("Install", "WantedBy")
            or u.values("Install", "RequiredBy")
            else "no [Install] WantedBy=timers.target"
        ),
    )


@check(
    "timers.catch-up",
    "advisory",
    "calendar-not-persistent",
    "systemd.timer(5) Persistent=: 'useful to catch up on missed runs of the service when the "
    "system was powered down'",
)
def _catch_up(subject: Subject) -> list[Finding]:
    return each(
        subject.timers,
        lambda u: (
            "a calendar timer without Persistent=true"
            if u.values("Timer", "OnCalendar")
            and not truthy(u.last("Timer", "Persistent"))
            else None
        ),
    )


@check(
    "timers.timezone",
    "advisory",
    "calendar-timezone-implicit",
    "systemd.time(7): a calendar event names UTC or an IANA zone, or follows whatever zone the "
    "host is set to",
)
def _timezone(subject: Subject) -> list[Finding]:
    out = []
    for timer in subject.timers:
        for expression in timer.values("Timer", "OnCalendar"):
            last = expression.split()[-1] if expression.split() else ""
            if calendar_problem(expression) is None and not _is_zone(last):
                out.append(
                    Finding(timer.rel, f"OnCalendar={expression} names no time zone")
                )
    return out


@check(
    "timers.spread",
    "advisory",
    "randomized-delay-missing",
    "systemd.timer(5) RandomizedDelaySec=: 'prevent them from firing all at the same time, "
    "possibly resulting in resource congestion'",
)
def _spread(subject: Subject) -> list[Finding]:
    return each(
        subject.timers,
        lambda u: (
            "no RandomizedDelaySec="
            if u.values("Timer", "OnCalendar")
            and not seconds(u.last("Timer", "RandomizedDelaySec"))
            else None
        ),
    )


@check(
    "timers.accuracy",
    "advisory",
    "accuracy-coarse",
    "systemd.timer(5): 'set AccuracySec=1us and RandomizedDelaySec= to some higher value' "
    "(AccuracySec defaults to 1min)",
)
def _accuracy(subject: Subject) -> list[Finding]:
    return each(
        subject.timers,
        lambda u: (
            "RandomizedDelaySec= with the default AccuracySec=1min"
            if seconds(u.last("Timer", "RandomizedDelaySec"))
            and u.last("Timer", "AccuracySec") is None
            else None
        ),
    )


@check(
    "timers.target-oneshot",
    "advisory",
    "timer-target-long-running",
    "systemd.timer(5): an active unit 'is not restarted, but simply left running'; a scheduled "
    "job is Type=oneshot",
)
def _target_oneshot(subject: Subject) -> list[Finding]:
    def test(timer: Unit) -> str | None:
        target = subject.units.get(subject.timer_target(timer))
        if target is not None and long_running(target):
            return f"{target.name} is Type={service_type(target)}"
        return None

    return each(subject.timers, test)


@check(
    "timers.target-enabled",
    "advisory",
    "timer-target-enabled",
    "a timer-activated service with its own [Install] WantedBy= also runs at boot",
)
def _target_enabled(subject: Subject) -> list[Finding]:
    scheduled = subject.timer_activated()
    return each(
        subject.services,
        lambda u: (
            "a scheduled job with its own [Install] WantedBy="
            if u.name in scheduled
            and not long_running(u)
            and u.values("Install", "WantedBy")
            else None
        ),
    )


@check(
    "timers.name",
    "advisory",
    "timer-unit-name-mismatch",
    "systemd.timer(5) Unit=: 'recommended that the unit name that is activated and the unit name "
    "of the timer unit are named identically'",
)
def _timer_name(subject: Subject) -> list[Finding]:
    return each(
        subject.timers,
        lambda u: (
            f"Unit={u.last('Timer', 'Unit')}"
            if u.last("Timer", "Unit")
            and subject.timer_target(u) != u.name[: -len(".timer")] + ".service"
            else None
        ),
    )


# --- backup (3-2-1, Litestream, restore drills) -----------------------------------------------


def _stateful(subject: Subject) -> bool:
    return bool(subject.litestream) or any(
        unit.values("Service", "StateDirectory")
        or unit.values("Service", "ReadWritePaths")
        for unit in subject.system_services
    )


def _backup_jobs(subject: Subject) -> list[Unit]:
    scheduled = subject.timer_activated()
    return [
        unit
        for unit in subject.services
        if unit.name in scheduled
        and "backup" in unit.name
        and "restore" not in unit.name
    ]


def _drills(subject: Subject) -> list[Unit]:
    scheduled = subject.timer_activated()
    return [
        unit
        for unit in subject.services
        if unit.name in scheduled
        and (
            "restore" in unit.name or "litestream restore" in subject.script_text(unit)
        )
    ]


def _replicated(subject: Subject) -> list[str]:
    return [
        url
        for config in subject.litestream
        for db in config.databases()
        for url in replica_urls(db)
    ]


@check(
    "backup.copies",
    "blocking",
    "backup-copies-short",
    "CISA/US-CERT Data Backup Options (3-2-1): 'Keep 3 copies of any important file: 1 primary "
    "and 2 backups' -- here a Litestream replica and a scheduled *-backup job",
)
def _copies(subject: Subject) -> list[Finding]:
    if not _stateful(subject):
        return []
    copies = (1 if _replicated(subject) else 0) + (1 if _backup_jobs(subject) else 0)
    if copies >= 2:
        return []
    have = []
    if _replicated(subject):
        have.append("a Litestream replica")
    if _backup_jobs(subject):
        have.append("a scheduled backup job")
    return [
        Finding(
            DEPLOY, f"{copies} backup copy ({', '.join(have) or 'none'}); 3-2-1 needs 2"
        )
    ]


@check(
    "backup.offsite",
    "blocking",
    "backup-not-offsite",
    "CISA/US-CERT Data Backup Options (3-2-1): 'Store 1 copy offsite'",
)
def _offsite(subject: Subject) -> list[Finding]:
    if not _stateful(subject):
        return []
    for url in _replicated(subject):
        scheme = url.split("://", 1)[0].lower() if "://" in url else "file"
        if scheme in REMOTE_SCHEMES:
            return []
    for job in _backup_jobs(subject):
        if re.search(
            r"\b(?:gs|s3)://|gcloud storage cp|gsutil cp|rclone|aws s3",
            subject.script_text(job),
        ):
            return []
    return [Finding(DEPLOY, "no replica or backup job writes off the host")]


@check(
    "backup.restore-drill",
    "blocking",
    "restore-drill-missing",
    "CISA #StopRansomware Guide: 'regularly test the availability and integrity of backups'; "
    "Litestream docs: test restore with litestream restore -o",
)
def _drill(subject: Subject) -> list[Finding]:
    if not _stateful(subject) or _drills(subject):
        return []
    return [
        Finding(DEPLOY, "no timer-activated restore drill (*-restore-drill.service)")
    ]


@check(
    "backup.litestream-replica",
    "blocking",
    "litestream-no-replica",
    "Litestream reference/config: every dbs entry names its replica",
)
def _litestream_replica(subject: Subject) -> list[Finding]:
    out = []
    for config in subject.litestream:
        if config.error:
            out.append(Finding(config.rel, f"unreadable: {config.error}"))
            continue
        databases = config.databases()
        if not databases:
            out.append(Finding(config.rel, "no dbs entry"))
        for db in databases:
            if not replica_urls(db):
                out.append(
                    Finding(config.rel, f"{db.get('path', '?')} names no replica")
                )
    return out


@check(
    "backup.litestream-legacy",
    "blocking",
    "litestream-replicas-deprecated",
    "Litestream v0.5 migration: 'Transition from the deprecated replicas array to the current "
    "single replica field'",
)
def _litestream_legacy(subject: Subject) -> list[Finding]:
    return [
        Finding(config.rel, f"{db.get('path', '?')} uses the replicas: list")
        for config in subject.litestream
        for db in config.databases()
        if "replicas" in db
    ]


@check(
    "backup.litestream-unit",
    "advisory",
    "litestream-unit-missing",
    "Litestream guides/systemd: run replication as a systemd service so it restarts with the host",
)
def _litestream_unit(subject: Subject) -> list[Finding]:
    if not subject.litestream:
        return []
    for unit in subject.services:
        if "litestream replicate" in " ".join(exec_commands(unit)):
            return []
    return [Finding(DEPLOY, "no unit runs litestream replicate")]


@check(
    "backup.drill-integrity",
    "advisory",
    "restore-drill-integrity-unchecked",
    "Litestream docs: after litestream restore, check the copy with PRAGMA integrity_check",
)
def _drill_integrity(subject: Subject) -> list[Finding]:
    return each(
        _drills(subject),
        lambda u: (
            None
            if re.search(
                r"integrity_check|quick_check", subject.script_text(u), re.IGNORECASE
            )
            else "the drill never runs PRAGMA integrity_check"
        ),
    )


@check(
    "backup.litestream-snapshot",
    "advisory",
    "litestream-snapshot-default",
    "Litestream reference/config snapshot: interval and retention default to 24h; state the "
    "restore chain you want",
)
def _litestream_snapshot(subject: Subject) -> list[Finding]:
    return [
        Finding(config.rel, "no snapshot: interval/retention")
        for config in subject.litestream
        if config.document is not None
        and not isinstance(config.document.get("snapshot"), dict)
    ]


@check(
    "backup.litestream-validation",
    "advisory",
    "litestream-validation-off",
    "Litestream reference/config validation: 'periodic integrity checks for LTX files'",
)
def _litestream_validation(subject: Subject) -> list[Finding]:
    return [
        Finding(config.rel, "no validation: interval")
        for config in subject.litestream
        if config.document is not None
        and not isinstance(config.document.get("validation"), dict)
    ]


# ---------------------------------------------------------------------------
# Running


BY_ID = {entry.id: entry for entry in CATALOG}


@dataclasses.dataclass(frozen=True, slots=True)
class Outcome:
    check: Check
    findings: list[Finding]
    waived: list[tuple[Finding, str]]

    @property
    def verdict(self) -> str:
        if not self.findings:
            return "pass"
        return "fail" if self.check.severity == "blocking" else "advisory"


def run_check(subject: Subject, entry: Check) -> Outcome:
    if not subject.examined():
        return Outcome(entry, [Finding(DEPLOY, "units-none: no unit to examine")], [])
    findings = entry.run(subject)
    if entry.severity == "blocking":
        return Outcome(entry, findings, [])
    kept, waived = [], []
    for finding in findings:
        unit = next((u for u in subject.units.values() if u.rel == finding.unit), None)
        # `Unit.waived` is the ONE place a waiver with no why is dropped; a second guard here would
        # hide a regression there (mutation S26638 measured exactly that shadowing).
        why = unit.waived().get(entry.reason) if unit else None
        if why is not None:
            waived.append((finding, why))
        else:
            kept.append(finding)
    return Outcome(entry, kept, waived)


def report(subject: Subject, outcomes: list[Outcome]) -> dict[str, object]:
    empty = not subject.examined()
    blocking = [
        o for o in outcomes if o.findings and (o.check.severity == "blocking" or empty)
    ]
    return {
        "schema": SCHEMA,
        "root": str(subject.root),
        "examined": {
            "units": subject.examined(),
            "services": len(subject.services),
            "timers": len(subject.timers),
            "slices": len(subject.of_kind("slice")),
            "drop_ins": subject.drop_ins,
            "env_files": len(subject.env_files),
            "litestream_configs": len(subject.litestream),
            "scripts": len(subject.scripts),
        },
        "verdict": "FAIL" if blocking else "OK",
        "reasons": sorted(
            {("units-none" if empty else o.check.reason) for o in blocking}
        ),
        "advisories": sorted(
            {
                o.check.reason
                for o in outcomes
                if o.findings and not empty and o.check.severity == "advisory"
            }
        ),
        "checks": [
            {
                "id": o.check.id,
                "stage": o.check.stage,
                "severity": o.check.severity,
                "reason": "units-none" if empty else o.check.reason,
                "verdict": "fail" if empty else o.verdict,
                "findings": [dataclasses.asdict(f) for f in o.findings],
                "waived": [
                    {**dataclasses.asdict(f), "why": why} for f, why in o.waived
                ],
            }
            for o in outcomes
        ],
    }


def text_report(body: dict[str, object]) -> str:
    examined = body["examined"]
    assert isinstance(examined, dict)
    lines = [
        f"durable-unit-lint: {body['verdict']} examined {examined['units']} unit(s) "
        f"({examined['services']} service, {examined['timers']} timer) under {body['root']}"
    ]
    checks = body["checks"]
    assert isinstance(checks, list)
    for entry in checks:
        if entry["verdict"] == "pass" and not entry["waived"]:
            continue
        lines.append(
            f"  {entry['verdict'].upper():8} {entry['id']} ({entry['reason']})"
        )
        for finding in entry["findings"]:
            lines.append(f"      {finding['unit']}: {finding['detail']}")
        for waived in entry["waived"]:
            lines.append(f"      waived {waived['unit']}: {waived['why']}")
    return "\n".join(lines)


def emit(body: dict[str, object], form: str) -> None:
    print(json.dumps(body, sort_keys=True) if form == "json" else text_report(body))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="durable-unit-lint.py",
        description="Judge a repository's deploy/ systemd units offline (SPEC-V2-2195).",
    )
    sub = parser.add_subparsers(dest="command", required=True)
    one = sub.add_parser("check", help="run one check")
    one.add_argument("--id", required=True)
    every = sub.add_parser("lint", help="run every check")
    listing = sub.add_parser("catalog", help="print the check catalog")
    for command in (one, every):
        command.add_argument("--root", required=True)
    for command in (one, every, listing):
        command.add_argument("--format", choices=("text", "json"), default="text")
    args = parser.parse_args(argv)

    if args.command == "catalog":
        rows = [
            {k: getattr(c, k) for k in ("id", "stage", "severity", "reason", "source")}
            for c in CATALOG
        ]
        if args.format == "json":
            print(json.dumps(rows, indent=1))
        else:
            for row in rows:
                print(f"{row['id']}\t{row['severity']}\t{row['reason']}")
        print(f"examined {len(rows)} checks", file=sys.stderr)
        return 0

    root = Path(args.root)
    if not root.is_dir():
        print(f"--root {root} is not a directory", file=sys.stderr)
        return 2
    if args.command == "check" and args.id not in BY_ID:
        print(f"unknown check id: {args.id}", file=sys.stderr)
        return 2
    subject = load_subject(root.resolve())
    chosen = [BY_ID[args.id]] if args.command == "check" else CATALOG
    body = report(subject, [run_check(subject, entry) for entry in chosen])
    emit(body, args.format)
    if args.command == "check":
        # One check: exit 1 on any finding, blocking or advisory; the row's severity decides.
        outcome = body["checks"][0]
        return 1 if outcome["findings"] else 0
    return 1 if body["verdict"] == "FAIL" else 0


if __name__ == "__main__":
    sys.exit(main())
