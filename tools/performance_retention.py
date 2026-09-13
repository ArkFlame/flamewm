#!/usr/bin/env python3
"""Retain bounded .performance evidence runs.

Policy (env overridable):
  FLAMEWM_PERFORMANCE_CLEANUP=1   (default on; anything else disables)
  FLAMEWM_PERFORMANCE_KEEP_RUNS=1 (keep newest N matching runs)
  FLAMEWM_PERFORMANCE_MAX_AGE_DAYS=0 (delete matching runs older than this,
                                  but never below the KEEP_RUNS floor)

Only directories directly under the perf dir whose name matches
^\\d{8}T\\d{6}Z-\\d+$ are candidates. Everything else is protected:
  - files/symlinks (e.g. `latest`) are never deleted
  - names not matching the regex are never deleted
  - runs containing a `.keep` file are pinned and never deleted
  - the run targeted by the `latest` symlink is never deleted
  - the current run (--current RUN_ID or FLAMEWM_CURRENT_RUN) is never deleted
  - candidates resolving outside the perf dir are never deleted

Writes `retention.txt` into `.performance/<current-run>/retention.txt`
on every active run (falls back to the perf dir when current is unknown).
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


def _latest_target(perf: Path) -> str:
    link = perf / "latest"
    try:
        if link.is_symlink():
            target = os.readlink(link)
            resolved = (perf / target).resolve() if not os.path.isabs(target) else Path(target)
            try:
                name = resolved.relative_to(perf.resolve()).parts[0]
            except ValueError:
                return resolved.name
            return name
    except OSError:
        pass
    return ""


def retain(perf: Path, current: str = "", keep_runs: int = 12,
           max_age_days: int = 7) -> tuple[list[str], list[str], list[str]]:
    """Return (kept, deleted, protected) run names. Performs deletion."""
    latest = _latest_target(perf)
    try:
        perf_real = perf.resolve()
    except OSError:
        return [], [], []
    entries: list[tuple[str, float]] = []
    protected: list[str] = []
    for child in sorted(perf.iterdir(), key=lambda p: p.name):
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
            if child.resolve() != perf_real / name or perf_real not in child.resolve().parents:
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
    # `current` is protected above, but it still consumes one unpinned run slot.
    # Otherwise KEEP_RUNS=1 retains current plus one historical run.
    current_counts_toward_floor = bool(current and RUN_RE.match(current)
                                      and (perf / current).is_dir()
                                      and not (perf / current).is_symlink()
                                      and not (perf / current / ".keep").exists())
    historical_slots = max(0, keep_runs - int(current_counts_toward_floor))
    kept = [n for n, _ in entries[:historical_slots]]
    kept_set = set(kept)
    deleted: list[str] = []
    for name, mtime in entries[historical_slots:]:
        if now - mtime < max_age_secs:
            kept.append(name)
            kept_set.add(name)
            continue
        target = perf / name
        try:
            if target.is_symlink() or not target.is_dir():
                protected.append(name)
                continue
            if target.resolve() != perf_real / name:
                protected.append(name)
                continue
            import shutil
            shutil.rmtree(target)
            deleted.append(name)
        except OSError:
            protected.append(name)
    return kept, deleted, sorted(set(protected))


def write_retention(perf: Path, kept, deleted, protected, keep_runs, max_age_days,
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
        targets.append(perf / current / RETENTION_FILE)
    targets.append(perf / RETENTION_FILE)
    for target in targets:
        try:
            target.write_text(text)
        except OSError:
            pass


def self_test() -> int:
    import shutil
    import tempfile
    tmp = Path(tempfile.mkdtemp(prefix="flamewm-retention-test-"))
    try:
        perf = tmp / "perf"
        perf.mkdir()
        old = time.time() - 10 * 86400
        now = time.time()

        def mk(name, mtime=None):
            d = perf / name
            d.mkdir()
            (d / "x.txt").write_text("x")
            if mtime is not None:
                os.utime(d, (mtime, mtime))
            return d

        mk("20200101T000000Z-1", old)
        mk("20200102T000000Z-2", old)
        pinned = mk("20200103T000000Z-3", old)
        (pinned / ".keep").write_text("pin")
        cur = mk("20200104T000000Z-4", old)
        latest_run = mk("20200105T000000Z-5", old)
        (perf / "latest").symlink_to(latest_run.name)
        (perf / "notes.txt").write_text("keep me")
        (perf / "latest-file-link").symlink_to("notes.txt")
        (perf / "weird-name").mkdir()
        fresh = mk("20200106T000000Z-6", now)
        esc = tmp / "escape"
        esc.mkdir()
        (perf / "20200107T000000Z-7").symlink_to(str(esc))

        kept, deleted, protected = retain(perf, current=cur.name,
                                          keep_runs=1, max_age_days=7)
        assert (perf / "20200101T000000Z-1").exists() is False, "oldest must go"
        assert (perf / "20200102T000000Z-2").exists() is False, "second oldest must go"
        assert pinned.exists(), "pinned .keep must survive"
        assert cur.exists(), "current must survive"
        assert latest_run.exists(), "latest target must survive"
        assert (perf / "notes.txt").exists(), "nonmatching file must survive"
        assert (perf / "weird-name").exists(), "nonmatching dir must survive"
        assert fresh.exists(), "fresh run within max age must survive"
        assert esc.exists() and (perf / "20200107T000000Z-7").is_symlink(), \
            "symlink must survive"
        assert "20200101T000000Z-1" in deleted
        write_retention(perf, kept, deleted, protected, 1, 7)
        assert (perf / RETENTION_FILE).is_file(), "retention.txt must be written"
        # Idempotence: second pass deletes nothing new.
        _, deleted2, _ = retain(perf, current=cur.name, keep_runs=1, max_age_days=7)
        assert not (set(deleted2) - set(deleted)), "second pass must not delete more"

        # KEEP_RUNS includes current, not one extra historical unpinned run.
        exact = tmp / "exact"
        exact.mkdir()
        def exact_mk(name):
            d = exact / name
            d.mkdir()
            (d / "x.txt").write_text("x")
            os.utime(d, (old, old))
            return d

        old_run = exact_mk("20200101T000000Z-1")
        current_run = exact_mk("20200102T000000Z-2")
        pin_one = exact_mk("20200103T000000Z-3")
        pin_two = exact_mk("20200104T000000Z-4")
        (pin_one / ".keep").write_text("pin")
        (pin_two / ".keep").write_text("pin")
        retain(exact, current=current_run.name, keep_runs=1, max_age_days=0)
        assert not old_run.exists(), "current must consume KEEP_RUNS slot"
        assert current_run.exists() and pin_one.exists() and pin_two.exists(), \
            "only current and pinned runs must survive"
        print("OK performance_retention self-test")
        return 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="Bound .performance evidence retention.")
    ap.add_argument("perf_dir", nargs="?", default=".performance")
    ap.add_argument("--current", default=os.environ.get("FLAMEWM_CURRENT_RUN", ""))
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args(argv)
    if args.self_test:
        return self_test()
    if os.environ.get("FLAMEWM_PERFORMANCE_CLEANUP", "1") != "1":
        return 0
    perf = Path(args.perf_dir)
    if not perf.is_dir():
        return 0
    keep_runs = _env_int("FLAMEWM_PERFORMANCE_KEEP_RUNS", 1)
    max_age_days = _env_int("FLAMEWM_PERFORMANCE_MAX_AGE_DAYS", 0)
    kept, deleted, protected = retain(perf, current=args.current,
                                      keep_runs=keep_runs,
                                      max_age_days=max_age_days)
    write_retention(perf, kept, deleted, protected, keep_runs, max_age_days,
                    args.current)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
