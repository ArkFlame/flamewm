#!/usr/bin/env python3
"""Route repository work to canonical agent skills."""
from __future__ import annotations

import argparse
import json

CANONICAL_SKILLS = {
    "source_first": "flamewm-source-first",
    "parallel": "flamewm-parallel-execution",
    "architecture": "flamewm-semantic-modularization",
    "rust": "flamewm-rust-systems",
    "x11": "flamewm-x11-window-manager",
    "reference": "flamewm-reference-mining",
    "failure": "flamewm-failure-resolution",
    "runtime": "flamewm-runtime-evidence",
    "performance": "flamewm-performance-proof",
    "release": "flamewm-release-proof",
}

ROUTES = {
    "architecture": ("flamewm-semantic-modularization",),
    "boundary": ("flamewm-semantic-modularization",),
    "feature": ("flamewm-semantic-modularization",),
    "rust": ("flamewm-rust-systems",),
    ".rs": ("flamewm-rust-systems",),
    "x11": ("flamewm-x11-window-manager",),
    "icccm": ("flamewm-x11-window-manager",),
    "ewmh": ("flamewm-x11-window-manager",),
    ".vendor": ("flamewm-reference-mining",),
    "historical": ("flamewm-reference-mining",),
    "runtime": ("flamewm-runtime-evidence",),
    "integration": ("flamewm-runtime-evidence",),
    "native behavior": ("flamewm-runtime-evidence",),
    "performance": ("flamewm-performance-proof",),
    "tuning": ("flamewm-performance-proof",),
    "release": ("flamewm-release-proof",),
    "package": ("flamewm-release-proof",),
    "readiness": ("flamewm-release-proof",),
    "parallel": ("flamewm-parallel-execution",),
}
MANDATED_ROUTES = (
    (("popup", "xephyr"), ("flamewm-rust-systems", "flamewm-semantic-modularization", "flamewm-runtime-evidence")),
    (("icon lookup", "latency"), ("flamewm-rust-systems", "flamewm-performance-proof")),
    (("module split",), ("flamewm-rust-systems", "flamewm-semantic-modularization")),
)
FAILURES = ("error", "fail", "failure", "crash", "hang", "regression", "broken")


def route(mode: str, text: str, paths: list[str]) -> dict[str, object]:
    """Return bounded, explainable skills for a work request."""
    words = f"{text} {' '.join(paths)}".lower()
    complex_mode = mode.lower() == "complex"
    limit = 6 if complex_mode else 4
    required: list[str] = []
    optional: list[str] = []
    reasons = {
    }
    substantive = bool(words.strip())
    if substantive:
        required.append("flamewm-source-first")
        reasons["flamewm-source-first"] = "all substantive FlameWM tasks inspect source, callers, contracts, and owners first"
    for tokens, skills in MANDATED_ROUTES:
        if all(token in words for token in tokens):
            for skill in skills:
                if skill not in required:
                    required.append(skill)
                    reasons[skill] = f"mandated by {' + '.join(tokens)}"
    if any(token in words for token in FAILURES):
        required.append("flamewm-failure-resolution")
        reasons["flamewm-failure-resolution"] = "failure-resolution required"
    for keyword, skills in ROUTES.items():
        if keyword in words:
            for skill in skills:
                if skill not in required and skill not in optional:
                    optional.append(skill)
                    reasons[skill] = f"matched {keyword}"
    required = required[:limit]
    optional = optional[:limit - len(required)]
    return {"required": required, "optional": optional, "reasons": reasons}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", default="normal", choices=("normal", "complex"))
    parser.add_argument("--text", default="")
    parser.add_argument("--path", action="append", default=[])
    args = parser.parse_args()
    print(json.dumps(route(args.mode, args.text, args.path), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
