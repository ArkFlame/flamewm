#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import os
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
errors: list[str] = []


def require(path: str) -> Path:
    p = ROOT / path
    if not p.exists():
        errors.append(f"missing required path: {path}")
    return p


def read_required(path: str) -> str:
    p = ROOT / path
    return p.read_text() if p.exists() else ""


required = [
    "Cargo.toml",
    "FLAMEWM_VERSION",
    "README.md",
    "docs/TODO.md",
    "docs/ARCHITECTURE.md",
    "ui/shell/index.html",
    "ui/shell/flamewm.css",
    "crates/flamewm-wm/Cargo.toml",
    "crates/flamewm-wm/src/main.rs",
    "crates/flamewm-wm/src/wm.rs",
    "crates/flamewm-shell/Cargo.toml",
    "crates/flamewm-shell/build.rs",
    "crates/flamewm-shell/src/main.rs",
    "crates/flamewm-render-core/Cargo.toml",
    "crates/flamewm-render-x11/Cargo.toml",
    "assets/branding/flamewm-start.svg",
    "assets/branding/flamewm-wallpaper.webp",
    "assets/branding/flamewm-wordmark.png",
    "scripts/doctor",
    "scripts/check",
    "scripts/xephyr",
    "scripts/verify-visual",
    "tools/verify_rwr_promotion.py",
]
for path in required:
    require(path)

version = read_required("FLAMEWM_VERSION").strip()
if version != "0.0.4":
    errors.append(f"project version must be 0.0.4, got {version!r}")

root_manifest = ROOT / "Cargo.toml"
try:
    root_cargo = tomllib.loads(root_manifest.read_text())
except Exception as exc:
    errors.append(f"invalid root Cargo.toml: {exc}")
    root_cargo = {}
workspace = root_cargo.get("workspace", {})
package = workspace.get("package", {})
if package.get("version") != "0.0.4":
    errors.append("workspace.package.version must be 0.0.4")
if package.get("rust-version") != "1.85":
    errors.append("workspace.package.rust-version must be 1.85")
if workspace.get("members") != ["crates/*"]:
    errors.append("workspace members must be exactly crates/*")
if "exclude" in workspace:
    errors.append("workspace must not carry an engine/vendor exclude list")
if (ROOT / "engine").exists():
    errors.append("active /engine tree is forbidden; all production code belongs under /crates")

vendor = {
    "flamewm-c-reference": 500,
    "RustWebRender-0.0.9": 150,
}
vendor_root = ROOT / ".vendor"
allowed_vendor_dirs = set(vendor)
actual_vendor_dirs = {p.name for p in vendor_root.iterdir() if p.is_dir()} if vendor_root.exists() else set()
if actual_vendor_dirs != allowed_vendor_dirs:
    errors.append(
        f".vendor directories must be exactly {sorted(allowed_vendor_dirs)}, got {sorted(actual_vendor_dirs)}"
    )
for name, minimum in vendor.items():
    p = require(f".vendor/{name}")
    if p.exists():
        count = sum(1 for x in p.rglob("*") if x.is_file() or x.is_symlink())
        if count < minimum:
            errors.append(f"vendor tree {name} unexpectedly small: {count} < {minimum}")
if (rwr := ROOT / ".vendor/RustWebRender-0.0.9").exists():
    size = sum(p.stat().st_size for p in rwr.rglob("*") if p.is_file())
    if size > 12 * 1024 * 1024:
        errors.append(f"RustWebRender-0.0.9 vendor tree exceeds 12 MiB: {size} bytes")

# Every active crate must be a valid standalone manifest. Active source may never reach into reference corpora.
for manifest in sorted((ROOT / "crates").glob("*/Cargo.toml")):
    rel = manifest.relative_to(ROOT)
    try:
        data = tomllib.loads(manifest.read_text())
    except Exception as exc:
        errors.append(f"invalid manifest {rel}: {exc}")
        continue
    pkg = data.get("package", {})
    literal_rust = pkg.get("rust-version")
    if literal_rust == "0.0.1":
        errors.append(f"invalid copied rust-version 0.0.1 in {rel}")
    text = manifest.read_text()
    if re.search(r'(?im)^\s*path\s*=\s*["\'][^"\']*(?:\.vendor|/engine|engine/|icewm-rust)', text):
        errors.append(f"active path dependency reaches reference/legacy code: {rel}")
    for forbidden in ["wry", "webkit", "electron", "chromium", "tauri", "webview"]:
        if re.search(rf'(?im)^\s*{re.escape(forbidden)}\s*=', text):
            errors.append(f"forbidden browser/webview runtime dependency {forbidden} in {rel}")

def active_files(base: Path):
    """Yield source files while excluding generated Python cache entries."""
    for p in base.rglob("*"):
        if not p.is_file() or "target" in p.parts or "__pycache__" in p.parts:
            continue
        if p.suffix.lower() in {".pyc", ".pyo"}:
            continue
        yield p


active_roots = [ROOT / "crates", ROOT / "ui", ROOT / "scripts", ROOT / "tools"]
for base in active_roots:
    for p in active_files(base):
        if p.suffix.lower() not in {".rs", ".toml", ".html", ".css", ".py", ".sh", ""}:
            continue
        if p.resolve() == Path(__file__).resolve():
            continue
        text = p.read_text(errors="ignore").replace(".vendor/RustWebRender-0.0.9", ".vendor/<provenance>")
        if re.search(r'(include_bytes!|include_str!|path\s*=|src\s*=)[^\n]*\.vendor', text):
            errors.append(f"production dependency reaches into .vendor: {p.relative_to(ROOT)}")
        if "engine/icewm" in text or "engine/icewm-rust" in text:
            errors.append(f"legacy engine path referenced by active source: {p.relative_to(ROOT)}")

# Product surfaces use FlameWM identity. Historical vendor/provenance names are
# intentionally confined to the promotion verifier and vendor corpus.
for base in [ROOT / "crates", ROOT / "ui", ROOT / "scripts"]:
    for p in active_files(base):
        text = (
            p.read_text(errors="ignore")
            .replace(".vendor/RustWebRender-0.0.9", ".vendor/<provenance>")
            # Keep visual verifier's historical screenshot path operational without
            # treating its provenance filename as active product identity.
            .replace(
                "project_sources/visual/user-rwr-0.0.6-xephyr.png",
                "project_sources/visual/<historical-reference>.png",
            )
        )
        if re.search(r"RustWebRender|rustwebrender|(?<![A-Za-z])rwr-", text):
            errors.append(f"stale renderer identity in active product source: {p.relative_to(ROOT)}")

# No symlink in active paths may escape into reference corpora.
for base_name in ["crates", "ui", "assets", "scripts", "tools"]:
    base = ROOT / base_name
    for p in base.rglob("*"):
        if p.is_symlink():
            try:
                resolved = p.resolve()
            except OSError as exc:
                errors.append(f"broken active symlink {p.relative_to(ROOT)}: {exc}")
                continue
            if (ROOT / ".vendor") in resolved.parents or resolved == ROOT / ".vendor":
                errors.append(f"active symlink reaches .vendor: {p.relative_to(ROOT)}")

html = read_required("ui/shell/index.html")
if 'src="assets/' not in html:
    errors.append("ui/shell/index.html must consume local shell assets")
assets_link = ROOT / "ui/shell/assets"
if not assets_link.is_symlink() or assets_link.resolve() != (ROOT / "assets").resolve():
    errors.append("ui/shell/assets must be an internal symlink to active assets")
if re.search(r"<script\b", html, re.I):
    errors.append("compiled production UI must not embed JavaScript")
for src in re.findall(r'<img[^>]+src="([^"]+)"', html, re.I):
    if src.startswith("assets/") and not (ROOT / "assets/raster" / src.removeprefix("assets/")).exists():
        errors.append(f"missing UI asset: {src}")
for required_id in ["taskbar", "start-menu", "desktop-menu", "settings-window"]:
    if f'id="{required_id}"' not in html:
        errors.append(f"web prototype port missing required UI node: {required_id}")

shell = read_required("crates/flamewm-shell/src/main.rs")
shell_manifest = read_required("crates/flamewm-shell/Cargo.toml")
if not re.search(r'(?im)^\s*name\s*=\s*["\']flamewm-shell["\']\s*$', shell_manifest):
    errors.append("canonical shell manifest must declare package flamewm-shell")
for contract in [
    'include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-shell.rwr"))',
    "X11WindowRole::Desktop",
]:
    if contract not in shell:
        errors.append(f"web shell missing contract: {contract}")
for contract in ["https://wm.arkflame.com"]:
    if contract not in html:
        errors.append(f"canonical shell UI missing contract: {contract}")

stale_shell_name = "flamewm-" + "web-shell"
for base in [ROOT / "crates", ROOT / "scripts", ROOT / "tools"]:
    for p in active_files(base):
        if p.resolve() == Path(__file__).resolve():
            continue
        if stale_shell_name in p.read_text(errors="ignore"):
            errors.append(f"stale renamed shell reference in active source: {p.relative_to(ROOT)}")

wm = read_required("crates/flamewm-wm/src/wm.rs")
for contract in ["SUBSTRUCTURE_REDIRECT", "change_save_set", "reparent_window", "net_current_desktop", "snap_geometry"]:
    if contract not in wm:
        errors.append(f"native WM missing core contract: {contract}")

# No font binaries in active distributable paths. Runtime extraction belongs to ignored target/ only.
for p in ROOT.rglob("*"):
    if ".vendor" in p.parts or "target" in p.parts or "__pycache__" in p.parts or not p.is_file():
        continue
    if p.suffix.lower() in {".pyc", ".pyo"}:
        continue
    if p.suffix.lower() in {".ttf", ".otf", ".woff", ".woff2"}:
        errors.append(f"font binary must not be distributed in active tree: {p.relative_to(ROOT)}")

for script in ["scripts/doctor", "scripts/check", "scripts/xephyr", "scripts/verify-visual", "tools/verify_rwr_promotion.py"]:
    p = ROOT / script
    if p.exists() and not os.access(p, os.X_OK):
        errors.append(f"script is not executable: {script}")

if errors:
    for error in errors:
        print(f"ERROR {error}", file=sys.stderr)
    raise SystemExit(1)

print("OK FlameWM Rust WebRender 0.0.4 static audit")
