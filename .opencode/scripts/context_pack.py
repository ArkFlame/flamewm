#!/usr/bin/env python3
"""Emit bounded current source facts for handoffs."""
from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--path", action="append", required=True)
    parser.add_argument("--max-chars", type=int, default=1200)
    parser.add_argument("--max-lines", type=int, default=40)
    args = parser.parse_args()
    facts = []
    remaining = max(0, args.max_chars)
    for name in args.path:
        path = Path(name)
        fact = {"path": str(path), "exists": path.exists()}
        if path.is_file() and remaining:
            try:
                lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
                text = "\n".join(lines[:max(0, args.max_lines)])[:remaining]
                fact.update({"lines": len(lines), "excerpt": text})
                remaining -= len(text)
            except OSError as exc:
                fact["error"] = str(exc)
        elif path.exists():
            fact["kind"] = "directory" if path.is_dir() else "other"
        facts.append(fact)
    print(json.dumps({"facts": facts, "truncated": remaining == 0}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
