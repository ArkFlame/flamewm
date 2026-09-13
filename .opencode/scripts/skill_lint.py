#!/usr/bin/env python3
"""Validate local skill metadata and routing contract."""
from __future__ import annotations

import argparse
import importlib.util
import re
import sys
from pathlib import Path


def load_router(path: Path):
    spec = importlib.util.spec_from_file_location("skill_route", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skills-dir", type=Path, default=Path(".opencode/skills"))
    parser.add_argument("--router", type=Path, default=Path(__file__).with_name("skill_route.py"))
    args = parser.parse_args()
    errors: list[str] = []
    router = load_router(args.router)
    canonical = set(router.CANONICAL_SKILLS.values())
    for skill in sorted(canonical):
        matches = list(args.skills_dir.glob(f"{skill}/SKILL.md"))
        if not matches:
            errors.append(f"missing canonical skill: {skill}")
            continue
        text = matches[0].read_text(encoding="utf-8")
        if not re.match(r"(?m)^---\n[\s\S]*?^---\n", text):
            errors.append(f"missing frontmatter: {matches[0]}")
        elif not re.search(rf"(?m)^name:\s*{re.escape(skill)}\s*$", text):
            errors.append(f"frontmatter name mismatch: {matches[0]}")
    for keyword, skills in router.ROUTES.items():
        if not keyword or not skills:
            errors.append(f"invalid route: {keyword!r}")
        for skill in skills:
            if skill not in canonical:
                errors.append(f"route uses noncanonical skill: {keyword}={skill}")
    samples = (
        ("normal", "fix Rust X11 crash", ["crates/flamewm-wm-x11/src/wm.rs"], ("flamewm-rust-systems",)),
        ("normal", "ICCCM behavior", [], ("flamewm-x11-window-manager",)),
        ("normal", "historical corpus", [], ("flamewm-reference-mining",)),
        ("normal", "runtime integration", [], ("flamewm-runtime-evidence",)),
        ("normal", "performance tuning", [], ("flamewm-performance-proof",)),
        ("normal", "release package", [], ("flamewm-release-proof",)),
        ("complex", "parallel architecture feature", [], ("flamewm-parallel-execution",)),
        ("normal", "popup full screen in Xephyr with shell/UI paths", [], ("flamewm-source-first", "flamewm-rust-systems", "flamewm-semantic-modularization", "flamewm-runtime-evidence")),
        ("normal", "icon lookup latency", [], ("flamewm-source-first", "flamewm-rust-systems", "flamewm-performance-proof")),
        ("normal", "module split", [], ("flamewm-source-first", "flamewm-rust-systems", "flamewm-semantic-modularization")),
    )
    for mode, text, paths, expected_skills in samples:
        result = router.route(mode, text, paths)
        if "flamewm-source-first" not in result["required"]:
            errors.append(f"sample lacks source-first: {text}")
        if any(word in text for word in router.FAILURES) and "flamewm-failure-resolution" not in result["required"] + result["optional"]:
            errors.append(f"sample lacks failure-resolution: {text}")
        if len(result["required"]) + len(result["optional"]) > (6 if mode == "complex" else 4):
            errors.append(f"sample exceeds cap: {text}")
        for expected in expected_skills:
            if expected not in result["required"] + result["optional"]:
                errors.append(f"sample misses route {expected}: {text}")
    if errors:
        print("\n".join(f"ERROR {error}" for error in errors), file=sys.stderr)
        return 1
    print("skill_lint: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
