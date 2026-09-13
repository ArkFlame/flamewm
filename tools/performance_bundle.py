#!/usr/bin/env python3
"""Bundle a .performance RUN_DIR into an uploadable tar.gz.

Usage: tools/performance_bundle.py <RUN_DIR> <OUT.tar.gz>

Archive contains the run directory basename only (no absolute/parent paths).
Overwrites any previous OUT atomically (temp file + rename). Excludes
sockets, device nodes, and symlinks resolving outside the run dir.
Includes manifest/logs/profiles/summary/workload/gdb/root screenshot when
present (i.e. all regular files under RUN_DIR, minus excluded types).
"""
from __future__ import annotations

import os
import sys
import tarfile
import tempfile
from pathlib import Path


def _inside(run: Path, p: Path) -> bool:
    try:
        p.resolve().relative_to(run.resolve())
        return True
    except (ValueError, OSError):
        return False


def bundle(run_dir: str, out_path: str) -> int:
    run = Path(run_dir)
    out = Path(out_path)
    if not run.is_dir():
        print(f"performance_bundle: not a directory: {run_dir}", file=sys.stderr)
        return 1
    run_real = run.resolve()
    members: list[tuple[Path, str]] = []
    for child in sorted(run.rglob("*")):
        rel = child.relative_to(run)
        arc = str(Path(run.name) / rel)
        try:
            if child.is_symlink():
                target = (child.parent / os.readlink(child)).resolve()
                try:
                    target.relative_to(run_real)
                except ValueError:
                    continue
                if not target.exists():
                    continue
                members.append((child, arc))
            elif child.is_socket() or child.is_fifo() or child.is_block_device() or child.is_char_device():
                continue
            elif child.is_file():
                members.append((child, arc))
            elif child.is_dir():
                members.append((child, arc))
        except OSError:
            continue
    out.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=out.name + ".", suffix=".tmp", dir=str(out.parent))
    os.close(fd)
    try:
        with tarfile.open(tmp, "w:gz") as tf:
            for src, arc in members:
                try:
                    tf.add(str(src), arcname=arc, recursive=False)
                except OSError:
                    continue
        os.replace(tmp, str(out))
    finally:
        try:
            os.unlink(tmp)
        except OSError:
            pass
    print(f"performance_bundle: {out} ({len(members)} entries)")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("usage: performance_bundle.py <RUN_DIR> <OUT.tar.gz>", file=sys.stderr)
        raise SystemExit(2)
    raise SystemExit(bundle(sys.argv[1], sys.argv[2]))
