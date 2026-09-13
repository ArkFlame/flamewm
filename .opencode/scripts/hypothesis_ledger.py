#!/usr/bin/env python3
"""Persistent local hypothesis ledger under ignored agent runtime state."""
from __future__ import annotations

import argparse
import json
from datetime import UTC, datetime
from pathlib import Path

ROOT = Path(".agents/runtime/hypotheses")


def stamp() -> str:
    return datetime.now(UTC).isoformat()


def file_for(name: str) -> Path:
    if not name or Path(name).name != name or name in {".", ".."}:
        raise ValueError("name must be one filename")
    return ROOT / f"{name}.json"


def load(name: str) -> tuple[Path, dict]:
    path = file_for(name)
    if not path.is_file():
        raise ValueError(f"missing ledger: {name}")
    return path, json.loads(path.read_text(encoding="utf-8"))


def save(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    for command in ("init", "add", "mark", "show", "close"):
        sub = commands.add_parser(command)
        sub.add_argument("name")
        if command == "add":
            sub.add_argument("text")
        if command == "mark":
            sub.add_argument("id", type=int)
            sub.add_argument("status", choices=("open", "supported", "rejected"))
        if command == "close":
            sub.add_argument("--reason", default="closed")
    args = parser.parse_args()
    try:
        if args.command == "init":
            ROOT.mkdir(parents=True, exist_ok=True)
            path = file_for(args.name)
            if path.exists():
                raise ValueError(f"ledger exists: {args.name}")
            save(path, {"name": args.name, "closed": False, "entries": [], "created_at": stamp()})
            data = json.loads(path.read_text(encoding="utf-8"))
        else:
            path, data = load(args.name)
            if args.command == "add":
                if data["closed"]:
                    raise ValueError("ledger is closed")
                data["entries"].append({"id": len(data["entries"]) + 1, "text": args.text, "status": "open", "updated_at": stamp()})
                save(path, data)
            elif args.command == "mark":
                entry = next((item for item in data["entries"] if item["id"] == args.id), None)
                if entry is None:
                    raise ValueError(f"missing hypothesis: {args.id}")
                entry["status"] = args.status
                entry["updated_at"] = stamp()
                save(path, data)
            elif args.command == "close":
                data["closed"] = True
                data["closed_at"] = stamp()
                data["close_reason"] = args.reason
                save(path, data)
        print(json.dumps(data, sort_keys=True))
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        parser.error(str(exc))
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
