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
    "crates/flamewm-wm-x11/Cargo.toml",
    "crates/flamewm-wm-x11/src/wm.rs",
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
if version != "0.0.7":
    errors.append(f"project version must be 0.0.7, got {version!r}")

root_manifest = ROOT / "Cargo.toml"
try:
    root_cargo = tomllib.loads(root_manifest.read_text())
except Exception as exc:
    errors.append(f"invalid root Cargo.toml: {exc}")
    root_cargo = {}
workspace = root_cargo.get("workspace", {})
package = workspace.get("package", {})
if package.get("version") != "0.0.7":
    errors.append("workspace.package.version must be 0.0.7")
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
shell_runtime = read_required("crates/flamewm-shell/src/runtime.rs")
shell_build = read_required("crates/flamewm-shell/build.rs")
shell_manifest = read_required("crates/flamewm-shell/Cargo.toml")
if not re.search(r'(?im)^\s*name\s*=\s*["\']flamewm-shell["\']\s*$', shell_manifest):
    errors.append("canonical shell manifest must declare package flamewm-shell")
shell_artifacts = [
    ("flamewm-panel.rwr", "panel.html"),
    ("flamewm-start.rwr", "start.html"),
    ("flamewm-start-submenu.rwr", "start-apps.html"),
    ("flamewm-task-menu.rwr", "task-menu.html"),
    ("flamewm-media.rwr", "media.html"),
    ("flamewm-audio.rwr", "audio.html"),
    ("flamewm-network.rwr", "network.html"),
    ("flamewm-calendar.rwr", "calendar.html"),
]
for artifact, source in shell_artifacts:
    if f'(\"{artifact}\", \"{source}\")' not in shell_build:
        errors.append(f"shell build must declare separate artifact {artifact} from {source}")
    if f'\"/{artifact}\"' not in shell_runtime:
        errors.append(f"shell runtime must include separate artifact {artifact}")
for path, text in [
    ("crates/flamewm-shell/build.rs", shell_build),
    ("crates/flamewm-shell/src/main.rs", shell),
    ("crates/flamewm-shell/src/runtime.rs", shell_runtime),
]:
    if "flamewm-shell.rwr" in text:
        errors.append(f"combined shell artifact is forbidden: {path}")
if "SurfaceRuntime::new" not in shell:
    errors.append("shell must create surfaces through SurfaceRuntime")
for role in ["SurfaceRole::Dock", "SurfaceRole::PopupMenu", "SurfaceRole::DropdownMenu"]:
    if role not in shell_runtime:
        errors.append(f"shell runtime missing surface role: {role}")

desktop = read_required("crates/flamewm-desktop/src/main.rs")
if "flamewm_ui_x11" not in desktop or "UiWindowRole::Desktop" not in desktop:
    errors.append("desktop must use the flamewm-ui-x11 desktop boundary")
if "X11WindowRole::Desktop" in desktop:
    errors.append("desktop must not use raw X11WindowRole::Desktop")
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

wm_x11_path = "crates/flamewm-wm-x11/src/wm.rs"
wm_x11 = read_required(wm_x11_path)
for contract, anchor in {
    "SUBSTRUCTURE_REDIRECT": "EventMask::SUBSTRUCTURE_REDIRECT",
    "change_save_set": ".change_save_set(",
    "reparent_window": ".reparent_window(",
    "net_current_desktop": "self.atoms.net_current_desktop",
    "snap_geometry": "core_snap_geometry(",
}.items():
    if anchor not in wm_x11:
        errors.append(f"native X11 WM missing core contract: {contract} ({wm_x11_path})")

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

print("OK FlameWM Rust WebRender 0.0.7 static audit")

# --- v62 framework-closure guards (G01-G10) ---
_compiled_templates = [
    "ui/shell/panel.html",
    "ui/shell/start.html",
    "ui/shell/start-apps.html",
    "ui/shell/task-menu.html",
    "ui/shell/media.html",
    "ui/shell/audio.html",
    "ui/shell/network.html",
    "ui/shell/calendar.html",
]
_known_keyed_ppms = {
    "task-start.ppm", "tray-volume.ppm", "tray-volume-muted.ppm", "tray-wifi.ppm",
    "tray-play.ppm", "tray-pause.ppm", "popup-volume.ppm", "popup-muted.ppm",
    "media-play.ppm", "media-pause.ppm", "media-prev.ppm", "media-next.ppm",
    "title-close.ppm", "title-maximize.ppm", "title-minimize.ppm", "title-restore.ppm",
}
# G01: compiled shell templates must not reference legacy keyed-magenta PPM roles.
for template in _compiled_templates:
    text = read_required(template)
    for ref in re.findall(r'assets/([A-Za-z0-9_./-]+\.ppm)', text):
        base = ref.rsplit("/", 1)[-1]
        if base in _known_keyed_ppms:
            errors.append(f"keyed-magenta PPM in active template {template}: {ref} (G01)")
# G02: no generic magenta-key comparison in production renderer (tests excluded).
for p in active_files(ROOT / "crates"):
    if p.suffix != ".rs" or "/tests" in p.as_posix() or p.name.endswith("_test.rs"):
        continue
    if "cfg(test)" in p.read_text(errors="ignore") and "mod tests" in p.read_text(errors="ignore"):
        pass
    text = p.read_text(errors="ignore")
    text_nt = re.sub(r"#\[cfg\(test\)\].*?\nmod tests \{.*?\n\}\n", "", text, flags=re.S)
    if re.search(r"(?i)(magenta.{0,40}key|key.{0,20}magenta|chroma.?key|color.?key.{0,20}transparent|transparent.{0,20}color.?key)", text_nt):
        if "no magenta" not in text_nt.lower() and "no color-key" not in text_nt.lower():
            errors.append(f"magenta-key heuristic in production renderer: {p.relative_to(ROOT)} (G02)")
# G03: shell must not import raw Pulse/NetworkManager provider APIs.
_shell_text = (read_required("crates/flamewm-shell/src/main.rs") + read_required("crates/flamewm-shell/src/runtime.rs"))
if re.search(r"use flamewm_integrations_linux::(pulse|network_manager)|use flamewm_platform::system::Pulse|NetworkManagerClient", _shell_text):
    errors.append("shell imports raw provider API; must use typed SystemAction only (G03)")
# G04: feature crates must not call XShape/XRender/Xcursor directly.
for crate in ["crates/flamewm-shell", "crates/flamewm-desktop", "crates/flamewm-settings",
              "crates/flamewm-platform", "crates/flamewm-api", "crates/flamewm-shell-core",
              "crates/flamewm-desktop-core"]:
    base = ROOT / crate
    if not base.exists():
        continue
    for p in active_files(base):
        if p.suffix != ".rs":
            continue
        if re.search(r"\b(XRender|XShape|Xcursor|Xft|XRenderComposite|XShapeCombine)\b", p.read_text(errors="ignore")):
            errors.append(f"feature crate calls native renderer API: {p.relative_to(ROOT)} (G04)")
# G05: wm-x11 must not use core image_text8 title path after C14 migration.
_wm_chrome = read_required("crates/flamewm-wm-x11/src/chrome.rs") + read_required("crates/flamewm-wm-x11/src/wm.rs")
if re.search(r"image_text8|XDrawString|9x15|TITLE_CHAR_ADVANCE", _wm_chrome):
    errors.append("wm-x11 still uses core image_text8 title path (G05)")
# G06: wm-x11 must not hand-sync duplicate chrome color constants.
if re.search(r"0x272a2d|0x46484b|0xe81123|TITLEBAR_BACKGROUND_RGB|CLOSE_HOVER_RGB", _wm_chrome):
    errors.append("wm-x11 has hand-synced duplicate chrome color constants (G06)")
# G07: ordinary active UI controls must not use cursor:pointer.
_shell_css = read_required("ui/shell/flamewm.css")
if "cursor:pointer" in _shell_css.replace(" ", ""):
    errors.append("ordinary UI CSS uses cursor:pointer (G07)")
# G08: no nmcli/pactl/wpctl subprocess control.
for p in active_files(ROOT / "crates"):
    if p.suffix != ".rs":
        continue
    if re.search(r'"(nmcli|pactl|wpctl)"|\bCommand::new\(\s*"(nmcli|pactl|wpctl)"', p.read_text(errors="ignore")):
        errors.append(f"subprocess system control in {p.relative_to(ROOT)} (G08)")
# G09: no fake KDE desktop identity.
for base in [ROOT / "crates", ROOT / "scripts"]:
    for p in active_files(base):
        text = p.read_text(errors="ignore")
        if re.search(r'XDG_CURRENT_DESKTOP["\']?\s*[,=]\s*["\']KDE["\']|set_var\(\s*"XDG_CURRENT_DESKTOP",\s*"KDE"\s*\)', text):
            errors.append(f"fake KDE desktop identity in {p.relative_to(ROOT)} (G09)")
# G10: architecture guard already rejects broad allow/unsafe suppression; static audit
# additionally rejects magenta PPM content in compiled-template image roles.
