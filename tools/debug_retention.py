#!/usr/bin/env python3
"""Retain bounded .debug per-run log directories.

Policy (env overridable):
  FLAMEWM_DEBUG_CLEANUP=1  (default on; anything else disables)
  FLAMEWM_DEBUG_KEEP_RUNS=10 (keep newest N matching runs)
  FLAMEWM_DEBUG_MAX_AGE_DAYS=0 (delete matching runs older than this,
                                but never below the KEEP_RUNS floor)

Only directories directly under the debug dir whose name matches
^\\d{8}T\\d{6}Z-\\d+$ are candidates. Everything else is protected:
  - files/symlinks (e.g. `latest`) are never deleted
  - names not matching the regex are never deleted
  - runs containing a `.keep` file are pinned and never deleted
  - the run targeted by the `latest` symlink is never deleted
  - the current run (--current RUN_ID or FLAMEWM_CURRENT_RUN) is never deleted
  - candidates resolving outside the debug dir are never deleted

Writes `retention.txt` into `.debug/<current-run>/retention.txt`
on every active run (falls back to the debug dir when current is unknown).
"""
from __future__ import annotations

import argparse
import os
import re
import sys
import time
from pathlib import Path

RUN_RE = re.compile(r"^\d{8}T\d{6}Z-\d+$")
RETENTION_FILE = "retention.txt"


def _env_int(name: str, default: int) -> int:
    try:
        return max(0, int(os.environ.get(name, str(default))))
    except ValueError:
        return default


def _latest_target(debug: Path) -> str:
    link = debug / "latest"
    try:
        if link.is_symlink():
            target = os.readlink(link)
            resolved = (debug / target).resolve() if not os.path.isabs(target) else Path(target)
            try:
                name = resolved.relative_to(debug.resolve()).parts[0]
            except ValueError:
                return resolved.name
            return name
    except OSError:
        pass
    return ""


def retain(debug: Path, current: str = "", keep_runs: int = 10,
           max_age_days: int = 0) -> tuple[list[str], list[str], list[str]]:
    """Return (kept, deleted, protected) run names. Performs deletion."""
    latest = _latest_target(debug)
    try:
        debug_real = debug.resolve()
    except OSError:
        return [], [], []
    entries: list[tuple[str, float]] = []
    protected: list[str] = []
    for child in sorted(debug.iterdir(), key=lambda p: p.name):
        name = child.name
        if not RUN_RE.match(name):
            continue
        if child.is_symlink():
            protected.append(name)
            continue
        if not child.is_dir():
            protected.append(name)
            continue
        try:
            if child.resolve() != debug_real / name or debug_real not in child.resolve().parents:
                protected.append(name)
                continue
        except OSError:
            protected.append(name)
            continue
        if name == current or name == latest:
            protected.append(name)
            continue
        if (child / ".keep").exists():
            protected.append(name)
            continue
        try:
            mtime = child.stat().st_mtime
        except OSError:
            protected.append(name)
            continue
        entries.append((name, mtime))
    entries.sort(key=lambda e: e[1], reverse=True)
    now = time.time()
    max_age_secs = max_age_days * 86400
    current_counts_toward_floor = bool(current and RUN_RE.match(current)
                                      and (debug / current).is_dir()
                                      and not (debug / current).is_symlink()
                                      and not (debug / current / ".keep").exists())
    historical_slots = max(0, keep_runs - int(current_counts_toward_floor))
    kept = [n for n, _ in entries[:historical_slots]]
    kept_set = set(kept)
    deleted: list[str] = []
    for name, mtime in entries[historical_slots:]:
        if now - mtime < max_age_secs:
            kept.append(name)
            kept_set.add(name)
            continue
        target = debug / name
        try:
            if target.is_symlink() or not target.is_dir():
                protected.append(name)
                continue
            if target.resolve() != debug_real / name:
                protected.append(name)
                continue
            import shutil
            shutil.rmtree(target)
            deleted.append(name)
        except OSError:
            protected.append(name)
    return kept, deleted, sorted(set(protected))


def write_retention(debug: Path, kept, deleted, protected, keep_runs, max_age_days,
                    current: str = "") -> None:
    lines = [
        f"keep_runs={keep_runs}",
        f"max_age_days={max_age_days}",
        f"kept={' '.join(sorted(set(kept)))}",
        f"deleted={' '.join(sorted(deleted))}",
        f"protected={' '.join(sorted(protected))}",
    ]
    text = "\n".join(lines) + "\n"
    targets: list[Path] = []
    if current and RUN_RE.match(current):
        targets.append(debug / current / RETENTION_FILE)
    targets.append(debug / RETENTION_FILE)
    for target in targets:
        try:
            target.write_text(text)
        except OSError:
            pass


def self_test() -> int:
    import shutil
    import tempfile
    tmp = Path(tempfile.mkdtemp(prefix="flamewm-debug-retention-test-"))
    try:
        debug = tmp / "debug"
        debug.mkdir()
        old = time.time() - 10 * 86400

        def mk(name, mtime=None):
            d = debug / name
            d.mkdir()
            (d / "wm.log").write_text("debug.session process=wm pid=1")
            if mtime is not None:
                os.utime(d, (mtime, mtime))
            return d

        # 12 old runs: with keep=10 + current protected, oldest must go.
        runs = [mk(f"202001{i:02d}T000000Z-{i}", old + i) for i in range(1, 13)]
        cur = runs[-1]
        latest_run = cur
        (debug / "latest").symlink_to(latest_run.name)
        (debug / "notes.txt").write_text("keep me")
        (debug / "weird-name").mkdir()
        kept, deleted, protected = retain(debug, current=cur.name,
                                          keep_runs=10, max_age_days=0)
        assert cur.exists(), "current must survive"
        assert latest_run.exists(), "latest target must survive"
        assert (debug / "notes.txt").exists(), "nonmatching file must survive"
        assert (debug / "weird-name").exists(), "nonmatching dir must survive"
        assert not (debug / "20200101T000000Z-1").exists(), "oldest must go"
        assert len([d for d in debug.iterdir()
                    if RUN_RE.match(d.name) and d.is_dir()]) == 10, "bounded at 10"
        assert latest_run.name in protected and cur.name in protected
        write_retention(debug, kept, deleted, protected, 10, 0)
        assert (debug / RETENTION_FILE).is_file(), "retention.txt must be written"
        print("OK debug_retention self-test")
        return 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="Bound .debug per-run retention.")
    ap.add_argument("debug_dir", nargs="?", default=".debug")
    ap.add_argument("--current", default=os.environ.get("FLAMEWM_CURRENT_RUN", ""))
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args(argv)
    if args.self_test:
        return self_test()
    if os.environ.get("FLAMEWM_DEBUG_CLEANUP", "1") != "1":
        return 0
    debug = Path(args.debug_dir)
    if not debug.is_dir():
        return 0
    keep_runs = _env_int("FLAMEWM_DEBUG_KEEP_RUNS", 10)
    max_age_days = _env_int("FLAMEWM_DEBUG_MAX_AGE_DAYS", 0)
    kept, deleted, protected = retain(debug, current=args.current,
                                      keep_runs=keep_runs,
                                      max_age_days=max_age_days)
    write_retention(debug, kept, deleted, protected, keep_runs, max_age_days,
                    args.current)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
