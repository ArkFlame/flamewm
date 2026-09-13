#!/usr/bin/env python3
"""J18 frame-engine guard fixtures: negative cases must trip the guard.

Run: python3 tools/test_j18_frame_guard.py
"""
from __future__ import annotations

import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import architecture_guard as g


def make_root(files: dict[str, str]) -> Path:
    tmp = Path(tempfile.mkdtemp(prefix="j18-"))
    for rel, text in files.items():
        p = tmp / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text)
    return tmp


def check_negative_second_geometry_owner() -> None:
    root = make_root({
        "crates/flamewm-wm-x11/src/frame/model.rs": "pub struct PlacementState { pub a: u32 }\n",
        "crates/flamewm-wm-x11/src/frame/geometry.rs": "pub struct GeometryPlan { pub a: u32 }\n",
        "crates/other/src/evil.rs": "pub struct PlacementState { pub b: u32 }\n",
    })
    found = g.j18_violations(root)
    caps = {c for _, c in found}
    assert "j18-g02-single-geometry-owner" in caps, f"J18-G02 must reject second PlacementState owner, got {found}"


def check_negative_second_hit_owner() -> None:
    root = make_root({
        "crates/flamewm-wm-x11/src/frame/model.rs": "pub struct PlacementState { pub a: u32 }\n",
        "crates/flamewm-wm-x11/src/frame/geometry.rs": "pub struct GeometryPlan { pub a: u32 }\n",
        "crates/flamewm-wm-x11/src/frame/input.rs": "pub fn target_for_xid() {}\n",
        "crates/evil/src/hit.rs": "pub fn resolve_pointer_intent() {}\n",
    })
    found = g.j18_violations(root)
    assert any(c == "j18-g03-single-pointer-hit-owner" for _, c in found), f"J18-G03 must reject second hit owner, got {found}"


def check_negative_old_paint_and_raw_title() -> None:
    root = make_root({
        "crates/flamewm-wm-x11/src/frame/model.rs": "pub struct PlacementState { pub a: u32 }\n",
        "crates/flamewm-wm-x11/src/frame/geometry.rs": "pub struct GeometryPlan { pub a: u32 }\n",
        "crates/flamewm-wm-x11/src/frame/input.rs": "pub fn target_for_xid() {}\n",
        "crates/flamewm-wm-x11/src/wm.rs": "fn a() { paint::paint_frame(x); }\n",
        "crates/flamewm-wm-x11/src/extra.rs": "fn b() { r.draw_title(x); }\n",
    })
    found = g.j18_violations(root)
    caps = {c for _, c in found}
    # wm.rs is a geometry owner, not a title owner, so its paint call trips G04.
    assert "j18-g04-no-old-decoration-paint" in caps, f"J18-G04 must reject, got {found}"
    assert "j18-g05-no-raw-title-renderer" in caps, f"J18-G05 must reject, got {found}"


def check_negative_outer_write() -> None:
    root = make_root({
        "crates/flamewm-wm-x11/src/frame/model.rs": "pub struct PlacementState { pub a: u32 }\n",
        "crates/flamewm-wm-x11/src/frame/geometry.rs": "pub struct GeometryPlan { pub a: u32 }\n",
        "crates/flamewm-wm-x11/src/frame/input.rs": "pub fn target_for_xid() {}\n",
        "crates/flamewm-wm-x11/src/decoration/paint.rs": "pub fn paint_frame() {}\n",
        "crates/flamewm-wm-x11/src/stray.rs": "fn b(s: &mut S) { s.outer = r; }\n",
    })
    found = g.j18_violations(root)
    assert any(c == "j18-g01-frame-geometry-owner" for _, c in found), f"J18-G01 must reject, got {found}"


def main() -> int:
    check_negative_second_geometry_owner()
    check_negative_second_hit_owner()
    check_negative_old_paint_and_raw_title()
    check_negative_outer_write()
    # Live tree must not trip J18 itself.
    live = g.j18_violations(ROOT)
    assert not live, f"J18 trips on live tree: {sorted(live)}"
    print("OK j18 frame-engine guard fixtures (4 negative + live clean)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
