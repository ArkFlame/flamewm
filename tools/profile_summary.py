#!/usr/bin/env python3
"""Summarize per-process PSS across xephyr logs and profiler .profile.log files.

Reads ` smaps pss=NkB` lines (profiler report windows) and
`name pid=.. pss=NkB` lines (harness pss.txt/log copies) from each input
file, groups by process name derived from the filename, and prints one row
per process: PSS_FIRST/LAST/MAX/DELTA/SLOPE_KB_PER_MIN.

Also summarizes profiler interaction rows of the form::

    <label> count=N max_wall=Xms p95_wall=Yms total_wall=Zms ... over16ms=K

Sections:

- ``INTERACTION_TOP``: every distinct label aggregated across all input
  files (max of max_wall, max of p95_wall, summed total_wall/count/over16ms),
  sorted by max_wall descending.
- ``WM_INTERACTION``: ``wm.*`` label subset.
- ``RENDER_BREAKDOWN``: ``render.*`` label subset.
- ``SHELL_INTERACTION``: ``shell.*`` label subset.
- ``ICON_PIPELINE``: ``icon.*``/``desktop.icon.*`` label subset.
- ``MISSING_REQUIRED_LABELS``: required labels never observed in any input.
  Absent labels are reported, never inferred as zero. A nonzero exit status
  plus an explicit RED marker is emitted when required labels are missing.
  Idle/canary runs use a smaller required set via the declared mode flag
  (``--mode=idle|canary``, ``--idle``, ``--canary``, or
  ``FLAMEWM_PROFILE_MODE`` env).

Slope is the least-squares fit of PSS (kB) against sample index, scaled by
FLAMEWM_PROFILE_INTERVAL (default 60s) to kB/min. Files that cannot be read
are skipped; no input yields a clear NO-DATA line, never a traceback.
"""
from __future__ import annotations

import os
import re
import sys
from math import isfinite
from pathlib import Path

PSS_PROFILE = re.compile(r"\bpss=(\d+)\s*kB")
PSS_HARNESS = re.compile(r"^(\S+)\s+pid=\d+\s+pss=(\d+)kB")
INTERACTION = re.compile(
    r"^\s*(\S+)\s+count=(\d+)\s+max_wall=([\d.]+)ms\s+"
    r"p95_wall=([\d.]+)ms\s+total_wall=([\d.]+)ms"
    r".*?over16ms=(\d+)"
)

REQUIRED_FULL = (
    "wm.loop",
    "render.present",
    "render.layout",
    "render.paint",
    "shell.startup.total",
    "icon.resolve.semantic",
    "icon.rasterize",
)
REQUIRED_IDLE = ("wm.loop", "render.present")


def _proc_name(path: str) -> str:
    base = Path(path).name
    for suffix in (".profile.log", ".log", ".txt"):
        if base.endswith(suffix):
            base = base[: -len(suffix)]
            break
    if base.startswith("flamewm-"):
        base = base[len("flamewm-"):]
    return base or "unknown"


def _samples_for(path: str) -> list[tuple[str, float]]:
    """Return (process, pss_kb) samples. pss.txt lines carry their own name."""
    samples: list[tuple[str, float]] = []
    default = _proc_name(path)
    is_pss_file = default == "pss" or Path(path).name == "pss.txt"
    try:
        with open(path, errors="ignore") as fh:
            for line in fh:
                m = PSS_HARNESS.match(line.strip())
                if m:
                    name = m.group(1) if is_pss_file else default
                    samples.append((name, float(m.group(2))))
                    continue
                m2 = PSS_PROFILE.search(line)
                if m2:
                    samples.append((default, float(m2.group(1))))
    except OSError:
        return []
    return samples


def _interactions_for(path: str) -> list[tuple[str, dict[str, float]]]:
    """Return (label, stat) interaction rows observed in a file."""
    rows: list[tuple[str, dict[str, float]]] = []
    try:
        with open(path, errors="ignore") as fh:
            for line in fh:
                m = INTERACTION.match(line.rstrip("\n"))
                if not m:
                    continue
                maximum = float(m.group(3))
                p95 = float(m.group(4))
                if not isfinite(maximum) or not isfinite(p95) or p95 > maximum:
                    continue
                rows.append((
                    m.group(1),
                    {
                        "count": float(m.group(2)),
                        "max": maximum,
                        "p95": p95,
                        "total": float(m.group(5)),
                        "over16": float(m.group(6)),
                    },
                ))
    except OSError:
        return []
    return rows


def _slope_per_min(series: list[float], interval_secs: float) -> float:
    n = len(series)
    if n < 2 or interval_secs <= 0:
        return 0.0
    mean_x = (n - 1) / 2.0
    mean_y = sum(series) / n
    denom = sum((i - mean_x) ** 2 for i in range(n))
    if denom == 0:
        return 0.0
    slope_per_sample = sum((i - mean_x) * (y - mean_y) for i, y in enumerate(series)) / denom
    return slope_per_sample * (60.0 / interval_secs)


def _resolve_mode(argv: list[str]) -> tuple[str, list[str]]:
    mode = (os.environ.get("FLAMEWM_PROFILE_MODE", "") or "").strip().lower()
    rest: list[str] = []
    for arg in argv:
        if arg.startswith("--mode="):
            mode = arg[len("--mode="):].strip().lower()
        elif arg in ("--idle", "--canary"):
            mode = arg[2:]
        else:
            rest.append(arg)
    if mode not in ("idle", "canary", "full", ""):
        mode = "full"
    if not mode:
        mode = "full"
    return mode, rest


def _print_section(title: str, agg: dict[str, dict[str, float]],
                   keep: object = None) -> None:
    print(f"{title}:")
    rows = [(label, st) for label, st in agg.items()
            if keep is None or keep(label)]
    rows.sort(key=lambda kv: kv[1]["max"], reverse=True)
    if not rows:
        print("  NO-DATA no interaction samples in scope")
        return
    for label, st in rows:
        print(f"  {label} count={st['count']:.0f} "
              f"max={st['max']:.2f}ms p95={st['p95']:.2f}ms "
              f"total={st['total']:.2f}ms over16ms={st['over16']:.0f}")


def main(argv: list[str]) -> int:
    mode, paths = _resolve_mode(argv)
    interval = 60.0
    try:
        interval = float(os.environ.get("FLAMEWM_PROFILE_INTERVAL", "60") or 60)
    except ValueError:
        interval = 60.0
    grouped: dict[str, list[float]] = {}
    order: list[str] = []
    agg: dict[str, dict[str, float]] = {}
    for path in paths:
        for name, value in _samples_for(path):
            if name not in grouped:
                grouped[name] = []
                order.append(name)
            grouped[name].append(value)
        for label, st in _interactions_for(path):
            slot = agg.setdefault(label, {"count": 0.0, "max": 0.0,
                                          "p95": 0.0, "total": 0.0,
                                          "over16": 0.0})
            slot["count"] += st["count"]
            slot["total"] += st["total"]
            slot["over16"] += st["over16"]
            slot["max"] = max(slot["max"], st["max"])
            slot["p95"] = max(slot["p95"], st["p95"])
    if not grouped and not agg:
        print("NO-DATA no PSS samples found")
        return 0
    if grouped:
        print(f"INTERVAL_SECS={interval:g}")
        for name in sorted(order):
            s = grouped[name]
            first, last, mx = s[0], s[-1], max(s)
            delta = last - first
            slope = _slope_per_min(s, interval)
            print(f"{name} samples={len(s)} PSS_FIRST={first:.0f}kB "
                  f"PSS_LAST={last:.0f}kB PSS_MAX={mx:.0f}kB "
                  f"PSS_DELTA={delta:+.0f}kB SLOPE_KB_PER_MIN={slope:+.2f}")
    else:
        print("NO-DATA no PSS samples found")
    if agg:
        _print_section("INTERACTION_TOP", agg)
        _print_section("WM_INTERACTION", agg,
                       lambda l: l == "wm" or l.startswith("wm."))
        _print_section("RENDER_BREAKDOWN", agg,
                       lambda l: l == "render" or l.startswith("render."))
        _print_section("SHELL_INTERACTION", agg,
                       lambda l: l == "shell" or l.startswith("shell."))
        _print_section("ICON_PIPELINE", agg,
                       lambda l: l.startswith("icon.")
                       or l.startswith("desktop.icon"))
    else:
        for title in ("INTERACTION_TOP", "WM_INTERACTION",
                      "RENDER_BREAKDOWN", "SHELL_INTERACTION",
                      "ICON_PIPELINE"):
            print(f"{title}:")
            print("  NO-DATA no interaction samples in scope")
    required = REQUIRED_IDLE if mode in ("idle", "canary") else REQUIRED_FULL
    missing = [label for label in required if label not in agg]
    if missing:
        print(f"MISSING_REQUIRED_LABELS mode={mode} "
              f"missing={','.join(missing)} STATUS=RED")
        return 1
    print(f"MISSING_REQUIRED_LABELS mode={mode} missing=none STATUS=GREEN")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
