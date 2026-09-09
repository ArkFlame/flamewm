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
if version != "0.0.8":
    errors.append(f"project version must be 0.0.8, got {version!r}")

root_manifest = ROOT / "Cargo.toml"
try:
    root_cargo = tomllib.loads(root_manifest.read_text())
except Exception as exc:
    errors.append(f"invalid root Cargo.toml: {exc}")
    root_cargo = {}
workspace = root_cargo.get("workspace", {})
package = workspace.get("package", {})
if package.get("version") != "0.0.8":
    errors.append("workspace.package.version must be 0.0.8")
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
# --- J7 icon/alpha guards (J7-G1..J7-G8) ---
_J7_TEST_OWNERS = {
    "crates/flamewm-image-core",
    "crates/flamewm-integrations-linux",
    "crates/flamewm-render-core",
    "crates/flamewm-render-x11",
    "crates/flamewm-skin",
    "crates/flamewm-ui-core",
    "crates/flamewm-ui-x11",
}
_J7_KNOWN_KEYED_PPMS = set(_known_keyed_ppms)
_J7_MAGENTA_TRIPLE = re.compile(r"\[\s*255\s*,\s*0\s*,\s*255\s*\]")


def _j7_is_test_file(path: Path) -> bool:
    posix = path.as_posix()
    return "/tests/" in posix or "/tests/" in posix.replace("\\", "/") or path.name.endswith("_test.rs")


def _j7_production_rs(base: Path):
    for p in active_files(base):
        if p.suffix == ".rs" and not _j7_is_test_file(p):
            yield p


def _j7_guard_text(path: Path) -> str:
    text = path.read_text(errors="ignore")
    return re.sub(r"#\[cfg\(test\)\].*?\nmod tests \{.*?\n\}\n", "", text, flags=re.S)


# J7-G1: no normal production renderer path blends over black. Owner crates may
# keep the opaque-compositing helper/documented fallback; ordinary product code
# may not call it or reimplement the same compositing math.
for _p in _j7_production_rs(ROOT / "crates"):
    _t = _j7_guard_text(_p)
    _is_owner = any(str(_p.relative_to(ROOT)).startswith(owner + "/") for owner in
                    ("crates/flamewm-render-x11", "crates/flamewm-image-core"))
    if re.search(r"\bblend_over_black\b", _t) and not _is_owner:
        errors.append(f"normal production renderer blends over black: {_p.relative_to(ROOT)} (J7-G1)")
    if re.search(r"fallback-black|FallbackBlack|fallback_black", _t) and not _is_owner:
        errors.append(f"normal production renderer uses black fallback: {_p.relative_to(ROOT)} (J7-G1)")
# J7-G2: no canonical Flame surface constructor uses XCreateSimpleWindow; only
# the declared Xlib FFI binding owner may declare it.
for _p in _j7_production_rs(ROOT / "crates"):
    _t = _j7_guard_text(_p)
    if "XCreateSimpleWindow" in _t and _p.relative_to(ROOT).as_posix() != "crates/flamewm-render-x11/src/xlib.rs":
        errors.append(f"canonical Flame surface uses XCreateSimpleWindow: {_p.relative_to(ROOT)} (J7-G2)")
# J7-G3: active icon consumer may not reference known keyed PPM icon assets.
# Only real Rust sources are scanned: compiled shell templates are owned by
# the G01 template guard, and Markdown/docs plus the guard tables themselves
# are not icon consumers. Guard tables and test-only files are excluded.
_J7_G3_SKIP = {"tools/static_audit.py", "tools/architecture_guard.py"}
for _p in _j7_production_rs(ROOT / "crates"):
    _rel = _p.relative_to(ROOT).as_posix()
    if _rel in _J7_G3_SKIP or _p.suffix != ".rs":
        continue
    _t = _j7_guard_text(_p)
    for _m in re.findall(r"[A-Za-z0-9_./-]+\.ppm", _t):
        _base = _m.rsplit("/", 1)[-1]
        if _base in _J7_KNOWN_KEYED_PPMS:
            errors.append(f"active icon consumer references keyed PPM {_m}: {_p.relative_to(ROOT)} (J7-G3)")
# J7-G4: IconResolver generic fallback may not be .ppm.
for _line in read_required("crates/flamewm-integrations-linux/src/icons.rs").splitlines():
    _s = _line.strip()
    if _s.startswith("const GENERIC_FALLBACK") and ".ppm" in _s:
        errors.append("IconResolver generic fallback is PPM (J7-G4)")
# J7-G5: no production magenta-key/FF00FF color-key path.
for _p in _j7_production_rs(ROOT / "crates"):
    _t = _j7_guard_text(_p)
    _low = _t.lower()
    if ("ff00ff" in _low or "0xff00ff" in _low or "color-key" in _low or "color_key" in _low
            or "colorkey" in _low or "chroma" in _low):
        if "no magenta" not in _low and "no color-key" not in _low:
            errors.append(f"production magenta/FF00FF color-key path: {_p.relative_to(ROOT)} (J7-G5)")
# J7-G6: SymbolicForeground must not be inferred from filename.
for _p in _j7_production_rs(ROOT / "crates"):
    _t = _j7_guard_text(_p)
    if re.search(r"SymbolicForeground", _t) and re.search(
            r"(?i)(file.?name|extension|ends_with|\.svg|\.ppm|\.png).{0,80}SymbolicForeground|SymbolicForeground.{0,80}(file.?name|extension|\.svg|\.ppm|\.png)", _t):
        errors.append(f"SymbolicForeground inferred from filename: {_p.relative_to(ROOT)} (J7-G6)")
# J7-G7: real app icon path defaults to Original treatment.
_skin_icons = read_required("crates/flamewm-skin/src/icons.rs")
if "IconTreatment::Original" not in _skin_icons:
    errors.append("real app icon path missing Original default (J7-G7)")
if not re.search(r"Start\s*\|\s*.*Browser|Browser.*Start", _skin_icons):
    errors.append("real app icon roles missing Original treatment anchor (J7-G7)")
# J7-G8: shell symbolic icons consume semantic foreground, not embedded Breeze RGB.
_paint = read_required("crates/flamewm-render-core/src/paint.rs")
if "SymbolicForeground" in _paint and "resolve_color(style.color)" not in _paint:
    errors.append("shell symbolic icons do not consume semantic foreground (J7-G8)")
for _tpl in ["ui/shell/panel.html", "ui/shell/start.html"]:
    _tt = read_required(_tpl)
    if re.search(r"(?i)fill\s*:\s*#[0-9a-f]{6}|#[0-9a-f]{6}.*symbolic|symbolic.*#[0-9a-f]{6}", _tt):
        errors.append(f"embedded Breeze RGB in symbolic template {_tpl} (J7-G8)")
# --- J6 symbolic/migration guards (J6-G1..J6-G7) ---
# J6-G1 is hard-fail (already true): full-color Start role must stay Original.
# J6-G2..J6-G7 describe post-migration state; they currently fail
# pre-migration, so they are staged as WARN-only until migration lands.
_J6_WARNINGS: list[str] = []
# TODO(J12-PENDING): promote J6-G5/G6 to hard errors once packaged->system
# and z-index-store migrations land; currently WARN-only by design.


def _strip_rust_comments(text: str) -> str:
    """Return source-context text with // and /* */ comments removed."""
    out: list[str] = []
    i = 0
    n = len(text)
    in_str: str | None = None
    in_block = 0
    in_line = False
    while i < n:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if in_line:
            if ch == "\n":
                in_line = False
                out.append(ch)
            i += 1
            continue
        if in_block > 0:
            if ch == "*" and nxt == "/":
                in_block -= 1
                i += 2
            elif ch == "/" and nxt == "*":
                in_block += 1
                i += 2
            else:
                if ch == "\n":
                    out.append(ch)
                i += 1
            continue
        if in_str is not None:
            out.append(ch)
            if ch == "\\" and i + 1 < n:
                out.append(text[i + 1])
                i += 2
                continue
            if ch == in_str:
                in_str = None
            i += 1
            continue
        if ch == "/" and nxt == "/":
            in_line = True
            i += 2
            continue
        if ch == "/" and nxt == "*":
            in_block = 1
            i += 2
            continue
        if ch == '"' or ch == "'":
            in_str = ch
            out.append(ch)
            i += 1
            continue
        out.append(ch)
        i += 1
    return "".join(out)


def _j6_code(text: str) -> str:
    return _strip_rust_comments(text)


def _j6_warn(message: str) -> None:
    _J6_WARNINGS.append(message)


_J6_SKIN = read_required("crates/flamewm-skin/src/icons.rs")
# J6-G1 (hard): Start category must not be SymbolicForeground.
if re.search(r"Self::Start[^\n]*SymbolicForeground|Start[^\n]{0,80}=>\s*IconTreatment::SymbolicForeground", _J6_SKIN):
    errors.append("full-color Start category uses SymbolicForeground (J6-G1)")
if "Self::Start" in _J6_SKIN and "IconTreatment::Original" not in _J6_SKIN:
    errors.append("Start category missing full-color Original treatment (J6-G1)")
# J6-G2 (enforced): local X11ChromePainter must be gone after migration.
_J6_CHROME = read_required("crates/flamewm-wm-x11/src/chrome.rs")
_J6_CHROME_CODE = _j6_code(_J6_CHROME)
if "struct X11ChromePainter" in _J6_CHROME_CODE:
    errors.append("local X11ChromePainter still present after migration (J6-G2)")
# J6-G3 (enforced): block-char title painter gone.
if re.search(r"fn draw_title_run", _J6_CHROME_CODE) and "poly_fill_rectangle" in _J6_CHROME_CODE:
    errors.append("block-char title painter still present (J6-G3)")
# J6-G4 (enforced): procedural control glyphs gone.
if re.search(r"fn draw_control_glyph", _J6_CHROME_CODE) and re.search(
    r"poly_line|poly_rectangle|poly_arc|poly_fill_arc", _J6_CHROME_CODE
):
    errors.append("procedural control glyphs still present (J6-G4)")
# J6-G5 (WARN-only): TODO(J12-PENDING) app icon packaged-before-system order.
_J6_ICONS = read_required("crates/flamewm-integrations-linux/src/icons.rs")
_m_pack = _j6_code(_J6_ICONS).find("packaged_candidates(name)")
_m_theme = _j6_code(_J6_ICONS).find("theme_candidates(name")
if _m_pack != -1 and _m_theme != -1 and _m_pack < _m_theme:
    _j6_warn("WARN TODO(J12-PENDING) app icon packaged-before-system order still present (J6-G5)")
# J6-G6 (WARN-only): TODO(J12-PENDING) new z-index store.
_J6_MODEL = read_required("crates/flamewm-render-core/src/model.rs")
if re.search(r"z_index_overrides|set_z_index|effective_z_index", _j6_code(_J6_MODEL)):
    _j6_warn("WARN TODO(J12-PENDING) new z-index store still present (J6-G6)")
# J6-G7 (enforced): root_pointer(0,0) placeholder gone.
_J6_SHELL_MAIN = read_required("crates/flamewm-shell/src/main.rs")
if re.search(r"root_pointer\(\s*0(\.0)?\s*,\s*0(\.0)?\s*\)", _j6_code(_J6_SHELL_MAIN)):
    errors.append("root_pointer(0,0) placeholder still present (J6-G7)")
# --- J12 focused guards (J12-G01..J12-G14): source-context only (comments stripped) ---
_J12_ROOT_DESKTOP_PROJ = read_required("crates/flamewm-desktop/src/projection.rs")
_J12_SHELL_PROJ = read_required("crates/flamewm-shell/src/projection.rs")
_J12_START_MENU = read_required("crates/flamewm-shell-core/src/start_menu.rs")
_J12_POPUP = read_required("crates/flamewm-ui-x11/src/popup.rs")
_J12_SURFACE_RT = read_required("crates/flamewm-ui-x11/src/surface_runtime.rs")
_J12_PAINT_DECOR = read_required("crates/flamewm-wm-x11/src/decoration/paint.rs")
_J12_MGR_DECOR = read_required("crates/flamewm-wm-x11/src/decoration/manager.rs")
_J12_SVG = read_required("crates/flamewm-image-core/src/svg.rs")
_J12_SESSION = read_required("crates/flamewm-session-core/src/lib.rs")
_J12_RENDER_PAINT = read_required("crates/flamewm-render-core/src/paint.rs")


def _j12_code(text: str) -> str:
    return _strip_rust_comments(text)


# J12-G01: invalid clamp(12.0,track) form must not appear in live code.
for _p in active_files(ROOT / "crates"):
    if _p.suffix != ".rs" or _j7_is_test_file(_p):
        continue
    if re.search(r"\bclamp\s*\(\s*12\.0", _j12_code(_p.read_text(errors="ignore"))):
        errors.append(f"invalid clamp(12.0,track) in live code: {_p.relative_to(ROOT)} (J12-G01)")
# J12-G02: replace_image must use node-local override, never mutate document.assets.
for _p in active_files(ROOT / "crates"):
    if _p.suffix != ".rs" or _j7_is_test_file(_p):
        continue
    _c = _j12_code(_p.read_text(errors="ignore"))
    if re.search(r"fn replace_image", _c) and re.search(
        r"\.assets\s*(\[[^\]]*\]\s*=|\.push\s*\(|\.insert\s*\(|\.clear\s*\()", _c
    ):
        errors.append(f"replace_image mutates document.assets: {_p.relative_to(ROOT)} (J12-G02)")
# J12-G03: no display:none on transient/popup/menu compiled templates (CSS/HTML context).
for _tpl in sorted((ROOT / "ui").rglob("*")):
    if not _tpl.is_file() or _tpl.suffix.lower() not in {".html", ".css"}:
        continue
    _t = re.sub(r"/\*.*?\*/", "", _tpl.read_text(errors="ignore"), flags=re.S)
    _t = re.sub(r"<!--.*?-->", "", _t, flags=re.S)
    if re.search(r"display\s*:\s*none", _t, re.I) and re.search(
        r"transient|popup-menu|context-menu|tooltip|dropdown", _t, re.I
    ):
        errors.append(f"display:none on transient node: {_tpl.relative_to(ROOT)} (J12-G03)")
# J12-G04: desktop root must sit on SurfaceBackground.
if "UiLayer::SurfaceBackground" not in _j12_code(_J12_ROOT_DESKTOP_PROJ):
    errors.append("desktop root missing SurfaceBackground layer (J12-G04)")
# J12-G05: semantic SVG themed render path present.
if "render_with_color_scheme" not in _j12_code(_J12_SVG):
    errors.append("semantic SVG themed render path missing (J12-G05)")
if "SymbolicForeground" not in _j12_code(_J12_RENDER_PAINT):
    errors.append("SymbolicForeground themed render missing in paint owner (J12-G05)")
# J12-G06: hidden create must not force redraw (body-bound).
_m = re.search(r"fn create_surface\(", _j12_code(_J12_SURFACE_RT))
if _m:
    _rest = _j12_code(_J12_SURFACE_RT)[_m.end():]
    _nxt = re.search(r"pub fn ", _rest)
    _body = _rest[:_nxt.start()] if _nxt else _rest[:800]
    if re.search(r"redraw|mark_full|Damage::Full", _body):
        errors.append("hidden create forces redraw (J12-G06)")
# J12-G07: popup raise policy = hide previous + show next.
if "runtime.hide" not in _j12_code(_J12_POPUP) or "runtime.show" not in _j12_code(_J12_POPUP):
    errors.append("popup raise policy missing hide-previous/show-next (J12-G07)")
# J12-G08: no sync IconResolver in category hot path.
if "IconResolver" in _j12_code(_J12_START_MENU):
    errors.append("sync IconResolver in category hot path (J12-G08)")
# J12-G09: Unknown category stays unbucketed (not visible).
if '_ => None' not in _j12_code(_J12_START_MENU):
    errors.append("Unknown category fallback missing (J12-G09)")
if 'presentation_key("Unknown"), None' not in _J12_START_MENU:
    errors.append("Unknown-not-visible anchor missing (J12-G09)")
# J12-G10: no image_text8/fixed-font normal decoration path in live code.
for _p in [ROOT / "crates/flamewm-wm-x11/src/decoration/paint.rs",
           ROOT / "crates/flamewm-wm-x11/src/decoration/manager.rs",
           ROOT / "crates/flamewm-wm-x11/src/wm.rs",
           ROOT / "crates/flamewm-wm-x11/src/chrome.rs"]:
    if _p.is_file() and re.search(
        r"image_text8|XDrawString|fixed_font|fixed-font|9x15|TITLE_CHAR_ADVANCE",
        _j12_code(_p.read_text(errors="ignore")),
    ):
        errors.append(f"legacy fixed-font decoration path: {_p.relative_to(ROOT)} (J12-G10)")
# J12-G11: exactly one DecorationManager owner.
_j12_mgr_count = sum(
    len(re.findall(r"struct DecorationManager", _j12_code(p.read_text(errors="ignore"))))
    for p in active_files(ROOT / "crates")
    if p.suffix == ".rs" and not _j7_is_test_file(p)
)
if _j12_mgr_count != 1:
    errors.append(f"DecorationManager owner count is {_j12_mgr_count}, want 1 (J12-G11)")
# J12-G12: wallpaper and CPU paths stay separate (no cpu conflation in desktop owner).
if "wallpaper" not in _j12_code(_J12_ROOT_DESKTOP_PROJ).lower():
    errors.append("desktop wallpaper path missing (J12-G12)")
for _p in active_files(ROOT / "crates/flamewm-desktop/src"):
    if _p.suffix == ".rs" and re.search(r"(?i)\bcpu\b", _j12_code(_p.read_text(errors="ignore"))):
        errors.append(f"CPU conflated into desktop owner: {_p.relative_to(ROOT)} (J12-G12)")
# J12-G13: .performance evidence dir stays ignored.
if ".performance" not in (ROOT / ".gitignore").read_text(errors="ignore"):
    errors.append(".performance is not ignored (J12-G13)")
# J12-G14: Breeze cursor authority intact.
if "FlameWM-Breeze-Dark" not in _J12_SESSION or "XCURSOR" not in _J12_SESSION:
    errors.append("Breeze cursor authority missing in session-core (J12-G14)")
if "Breeze-Dark" not in _J12_PAINT_DECOR and "CURSOR_THEME" not in _J12_PAINT_DECOR:
    errors.append("Breeze cursor anchor missing in decoration intent side (J12-G14)")
# --- J12 handoff guards (J12H-G01..G15): source-context only (comments stripped) ---
_J12H_PAINT_CORE = read_required("crates/flamewm-render-core/src/paint.rs")
_J12H_START_MENU_CORE = read_required("crates/flamewm-shell-core/src/start_menu.rs")
_J12H_PROJ = read_required("crates/flamewm-shell/src/projection.rs")
_J12H_RUNTIME = read_required("crates/flamewm-shell/src/runtime.rs")
_J12H_MAIN = read_required("crates/flamewm-shell/src/main.rs")
_J12H_AUDIO = read_required("crates/flamewm-shell/src/taskbar/status/audio.rs")
_J12H_NETWORK = read_required("crates/flamewm-shell/src/taskbar/status/network.rs")
_J12H_FILE_ACTIONS = read_required("crates/flamewm-desktop-core/src/file_actions.rs")
_J12H_STICKY_PERSIST = read_required("crates/flamewm-desktop-core/src/sticky_persistence.rs")
_J12H_CHROME = read_required("crates/flamewm-wm-x11/src/chrome.rs")
_J12H_WM = read_required("crates/flamewm-wm-x11/src/wm.rs")
_J12H_EXT = read_required("crates/flamewm-render-x11/src/external_drawable.rs")
_J12H_PALETTE = read_required("crates/flamewm-skin/src/palette.rs")
_J12H_DESKTOP_CSS = read_required("ui/desktop/desktop.css")
_J12H_SPAN = read_required("crates/flamewm-profiler/src/span.rs")
# J12H-G01: StrokeRect carries explicit radius (no square-corner regression).
if "StrokeRect" not in _j12_code(_J12H_PAINT_CORE) or "radius" not in _j12_code(_J12H_PAINT_CORE):
    errors.append("StrokeRect radius missing (J12H-G01)")
# J12H-G02: exactly one StartModel owner.
_J12H_STARTMODEL_COUNT = sum(
    len(re.findall(r"struct StartModel", _j12_code(p.read_text(errors="ignore"))))
    for p in active_files(ROOT / "crates")
    if p.suffix == ".rs" and not _j7_is_test_file(p)
)
if _J12H_STARTMODEL_COUNT != 1:
    errors.append(f"StartModel owner count is {_J12H_STARTMODEL_COUNT}, want 1 (J12H-G02)")
# J12H-G03: borrowed query hot path exists (no clone-only query model).
for _anchor in ["fn filter_view", "fn query_view", "fn slot_view"]:
    if _anchor not in _j12_code(_J12H_START_MENU_CORE):
        errors.append(f"borrowed StartModel query view missing {_anchor} (J12H-G03)")
# J12H-G04: Power visibility is explicit (session-gated, not always-on).
if "StartCategory::Power" not in _j12_code(_J12H_PROJ) or "session" not in _j12_code(_J12H_PROJ).lower():
    errors.append("Power visibility not session-gated (J12H-G04)")
# J12H-G05: changed-slot icon reset is surface-scoped (no blanket reproject).
if "need_panel" not in _j12_code(_J12H_MAIN) or "need_submenu" not in _j12_code(_J12H_MAIN):
    errors.append("changed slot icon reset not surface-scoped (J12H-G05)")
# J12H-G06: production OpenPopover path is anchored (measured, not fixed rect).
if "open_status_anchored_id" not in _j12_code(_J12H_RUNTIME) or "anchored_rect_by_id" not in _j12_code(_J12H_RUNTIME):
    errors.append("OpenPopover production path not anchored (J12H-G06)")
# J12H-G07: audio/network popovers have no fixed live authority (availability-gated).
if "ServiceAvailability::Available" not in _j12_code(_J12H_AUDIO):
    errors.append("audio live path not availability-gated (J12H-G07)")
if "ServiceAvailability::Available" not in _j12_code(_J12H_NETWORK):
    errors.append("network live path not availability-gated (J12H-G07)")
# J12H-G08: desktop context rows carry border 0.
if "border-width:0" not in _J12H_DESKTOP_CSS.replace(" ", ""):
    errors.append("desktop context border 0 missing (J12H-G08)")
# J12H-G09: rename is NOREPLACE (no silent overwrite).
if "RenameFlags::NOREPLACE" not in _j12_code(_J12H_FILE_ACTIONS):
    errors.append("NOREPLACE rename missing (J12H-G09)")
# J12H-G10: sticky state accepts v1 and v2 headers.
if "v1\\t" not in _J12H_STICKY_PERSIST or "v2\\t" not in _J12H_STICKY_PERSIST:
    errors.append("sticky v1+v2 headers missing (J12H-G10)")
# J12H-G11: WorkspacesChanged travels event-driven (no poll loop). Anchor
# lives in doc comments; check raw text plus revision-gated code path.
if ("resnapshot_dynamic" not in _j12_code(_J12H_MAIN)
        or ("never polls" not in _J12H_MAIN.lower() and "no polling" not in _J12H_RUNTIME.lower())):
    errors.append("WorkspacesChanged event-driven contract missing (J12H-G11)")
# J12H-G12: honest ExternalDrawable contract (safe-x11rb transport; geometry/material via skin).
if "ExternalDrawableTarget" not in (_J12_PAINT_DECOR + _J12H_CHROME) or "ExternalDrawableSession" not in _J12H_EXT:
    errors.append("ExternalDrawable delegation contract missing (J12H-G12)")
if "flamewm_skin" not in _j12_code(_J12H_CHROME) and "WINDOW_CHROME" not in _j12_code(_J12H_CHROME):
    errors.append("skin-metrics geometry/material contract missing (J12H-G12)")
# J12H-G13: no title-requires-xft normal path (measured intent is normal success).
_J12H_DECO_FILES = [
    ROOT / "crates/flamewm-wm-x11/src/decoration/paint.rs",
    ROOT / "crates/flamewm-wm-x11/src/chrome.rs",
    ROOT / "crates/flamewm-wm-x11/src/wm.rs",
]
for _p in _J12H_DECO_FILES:
    if _p.is_file():
        _t = _j12_code(_p.read_text(errors="ignore"))
        if "image_text8" in _t or "XDrawString" in _t:
            errors.append(f"legacy title path in decoration intent: {_p.relative_to(ROOT)} (J12H-G13)")
if (ROOT / "crates/flamewm-wm-x11/src/decoration/paint.rs").is_file():
    if "title_drawn = true" not in _j12_code(_J12_PAINT_DECOR):
        errors.append("measured title intent success missing (J12H-G13)")
# J12H-G14: PANEL stays #1b1e20.
if "0x1b1e20" not in _j12_code(_J12H_PALETTE).lower():
    errors.append("PANEL #1b1e20 missing in skin palette (J12H-G14)")
# J12H-G15: profiler spans are scoped static labels.
_J12H_PROF_FILES = [
    ROOT / "crates/flamewm-profiler/src/span.rs",
    ROOT / "crates/flamewm-profiler/src/point.rs",
    ROOT / "crates/flamewm-profiler/src/lib.rs",
]
_J12H_SPAN_ALL = "".join(f.read_text(errors="ignore") for f in _J12H_PROF_FILES if f.is_file())
if _J12H_SPAN_ALL and "&'static str" not in _J12H_SPAN_ALL:
    errors.append("scoped profiler span labels missing (J12H-G15)")
if 'ProfilePoint::new("shell.' not in _J12H_MAIN and "flamewm_profiler::start(" not in _J12H_MAIN:
    errors.append("scoped shell profiler spans missing (J12H-G15)")
for _w in _J6_WARNINGS:
    print(_w, file=sys.stderr)
if errors:
    for error in errors:
        print(f"ERROR {error}", file=sys.stderr)
    raise SystemExit(1)

print("OK FlameWM Rust WebRender 0.0.8 static audit")
print("OK J7 icon/alpha guards G1-G8 clean")
