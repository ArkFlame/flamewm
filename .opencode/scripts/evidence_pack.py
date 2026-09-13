#!/usr/bin/env python3
"""Summarize run evidence without copying complete logs."""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

PSS = re.compile(r"(?:^|\s)(\S+)\s+pid=\d+\s+pss=(\d+)kB|\bpss=(\d+)\s*kB")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("--max-files", type=int, default=20)
    args = parser.parse_args()
    run = args.run_dir
    result = {"run_dir": str(run), "manifest": None, "logs": [], "profiles": [], "pss_kb": {}}
    if not run.is_dir():
        result["error"] = "not a directory"
        print(json.dumps(result, sort_keys=True))
        return 1
    seen = 0
    for path in sorted(run.rglob("*")):
        if not path.is_file():
            continue
        rel = str(path.relative_to(run))
        if path.name in {"manifest.json", "run.json"}:
            try:
                result["manifest"] = json.loads(path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                result["manifest"] = {"path": rel, "status": "unreadable"}
        if path.suffix == ".log":
            result["logs"].append({"path": rel, "bytes": path.stat().st_size})
        if path.name.endswith(".profile.log") or path.name == "pss.txt":
            values = []
            try:
                for line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
                    for match in PSS.finditer(line):
                        values.append(int(match.group(2) or match.group(3)))
            except OSError:
                values = []
            result["profiles"].append({"path": rel, "samples": len(values), "first_kb": values[0] if values else None, "last_kb": values[-1] if values else None, "max_kb": max(values) if values else None})
        if path.suffix == ".log" or path.name == "pss.txt":
            seen += 1
            if seen >= args.max_files:
                break
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
