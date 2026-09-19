#!/usr/bin/env python3
"""Validate local skill metadata and routing contract."""
from __future__ import annotations

import argparse
import importlib.util
import re
import sys
from pathlib import Path

STALE_SKILL_IDS = ("x11-window-manager-engineering",)
REQUIRED_AGENTS = (
    "flame-coordinator",
    "flame-researcher",
    "flame-builder",
    "flame-repair",
    "flame-rust-reviewer",
    "flame-build-verifier",
    "flame-runtime-verifier",
)
COORDINATOR_DENY_ACTIONS = ("read", "glob", "grep", "shell", "edit")
COORDINATOR_SELF_INSPECT_PATTERNS = (
    "inspect the relevant source",
    "inspect current source",
    "read repository sources",
    "read assigned files before editing",
)
CRITICAL_TERMS = {
    "flamewm-source-first": ("source", "contract", "caller"),
    "flamewm-parallel-execution": ("parallel", "path", "owner"),
    "flamewm-semantic-modularization": ("owner", "authority", "typed"),
    "flamewm-rust-systems": ("rust", "ownership", "event"),
    "flamewm-x11-window-manager": ("x11", "ewmh", "authority"),
    "flamewm-reference-mining": ("reference", "vendor", "evidence"),
    "flamewm-failure-resolution": ("failure", "evidence", "root"),
    "flamewm-runtime-evidence": ("runtime", "native", "verification"),
    "flamewm-performance-proof": ("performance", "baseline", "metric"),
    "flamewm-release-proof": ("release", "build", "artifact"),
}


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
    parser.add_argument("--docs-dir", type=Path, default=Path("project_sources"))
    parser.add_argument("--agents-dir", type=Path, default=Path(".opencode/agents"))
    args = parser.parse_args()
    errors: list[str] = []
    agents_dir = args.agents_dir
    for agent in REQUIRED_AGENTS:
        matches = list(agents_dir.glob(f"{agent}.md"))
        if not matches:
            errors.append(f"missing required agent: {agent}")
    coord_path = agents_dir / "flame-coordinator.md"
    if coord_path.exists():
        coord_text = coord_path.read_text(encoding="utf-8")
        coord_body = coord_text.split("---", 2)[-1] if coord_text.startswith("---") else coord_text
        for action in COORDINATOR_DENY_ACTIONS:
            allow_hit = re.search(
                rf"action:\s*{re.escape(action)}\s*\n\s*resource:[^\n]*\n\s*effect:\s*allow",
                coord_text,
            )
            deny_hit = re.search(
                rf"action:\s*{re.escape(action)}\s*\n\s*resource:[^\n]*\n\s*effect:\s*deny",
                coord_text,
            )
            if allow_hit:
                errors.append(f"coordinator allows {action}: {coord_path}")
            elif not deny_hit:
                errors.append(f"coordinator missing deny for {action}: {coord_path}")
        lowered = coord_body.lower()
        for pattern in COORDINATOR_SELF_INSPECT_PATTERNS:
            if pattern in lowered:
                errors.append(f"coordinator body tells itself to inspect repo ({pattern!r}): {coord_path}")
    for agent_file, action in (
        ("flame-builder.md", "shell"),
        ("flame-build-verifier.md", "edit"),
        ("flame-runtime-verifier.md", "edit"),
    ):
        agent_path = agents_dir / agent_file
        if agent_path.exists():
            agent_text = agent_path.read_text(encoding="utf-8")
            if re.search(
                rf"action:\s*{re.escape(action)}\s*\n\s*resource:[^\n]*\n\s*effect:\s*allow",
                agent_text,
            ):
                errors.append(f"{agent_file} can {action}: {agent_path}")
    router = load_router(args.router)
    canonical = set(router.CANONICAL_SKILLS.values())
    router_text = args.router.read_text(encoding="utf-8")
    for stale_id in STALE_SKILL_IDS:
        if stale_id in router_text:
            errors.append(f"router uses stale ID {stale_id}: {args.router}")
        for path in args.docs_dir.rglob("*.md"):
            if stale_id in path.read_text(encoding="utf-8"):
                errors.append(f"documentation uses stale ID {stale_id}: {path}")
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
        body = re.sub(r"\A---\n[\s\S]*?^---\n", "", text, count=1, flags=re.MULTILINE).strip()
        if len(body) < 80:
            errors.append(f"skill body is not meaningful: {matches[0]}")
        missing_terms = [term for term in CRITICAL_TERMS[skill] if term not in body.lower()]
        if missing_terms:
            errors.append(f"skill lacks critical terms {missing_terms}: {matches[0]}")
        for stale_id in STALE_SKILL_IDS:
            if stale_id in text:
                errors.append(f"skill uses stale ID {stale_id}: {matches[0]}")
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
        ("complex", "S12 popup Xephyr failure", [], ("flamewm-failure-resolution", "flamewm-semantic-modularization", "flamewm-runtime-evidence")),
        ("normal", "panic in ICCCM path", [], ("flamewm-failure-resolution", "flamewm-x11-window-manager")),
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
