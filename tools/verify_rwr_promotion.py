#!/usr/bin/env python3
"""Verify the identity-preserving RustWebRender 0.0.9 promotion.

This is a development/provenance gate only. Production never loads `.vendor`.
The post-mortem lesson is enforced mechanically: source owners that FlameWM did
not need to extend must retain equivalent Rust mechanics after the crate-name rename.
"""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
REF = ROOT / ".vendor/RustWebRender-0.0.9"
errors: list[str] = []


NAMESPACE_TRANSLATIONS = {
    "rustwebrender_core": "flamewm_render_core",
    "rustwebrender_compiler": "flamewm_render_compiler",
    "rustwebrender_x11": "flamewm_render_x11",
}


def translate(text: str, translations: tuple[tuple[str, str], ...]) -> str:
    for source, target in translations:
        text = text.replace(source, target)
    return text


def tokens(text: str) -> tuple[str, ...]:
    pattern = re.compile(
        r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|'
        r'[A-Za-z_][A-Za-z0-9_]*|[0-9]+(?:\.[0-9]+)?|==|!=|<=|>=|=>|&&|\|\||'
        r'::|->|\.\.|[{}()\[\];,.:?+*/%&|^!<>=\-~]',
        re.DOTALL,
    )
    raw = tuple(token for token in pattern.findall(text) if not token.startswith(("//", "/*")))
    return tuple(
        token
        for index, token in enumerate(raw)
        if not (token == "," and index + 1 < len(raw) and raw[index + 1] in "})]")
    )


def normalized_tokens(text: str) -> tuple[str, ...]:
    """Compare Rust after formatter-only layout changes."""
    # Current rustfmt also reorders names inside a braced use item while the
    # 0.0.9 source predates that formatting. Keep that layout-only change out
    # of the provenance comparison.
    def sort_use(match: re.Match[str]) -> str:
        names = sorted(
            (part.strip() for part in match.group(2).split(",") if part.strip()),
            key=str.casefold,
        )
        return f"use {match.group(1)}::{{{', '.join(names)}}};"

    text = re.sub(
        r"use (flamewm_render_core|rustwebrender_core)::\{([^{}]*)\};",
        sort_use,
        text,
    )
    try:
        formatted = subprocess.run(
            ["rustfmt", "--edition", "2021", "--emit", "stdout"],
            input=text,
            text=True,
            capture_output=True,
            check=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        formatted = text
    return tokens(formatted)


def strip_patterns(text: str, patterns: tuple[str, ...]) -> str:
    for pattern in patterns:
        text, count = re.subn(pattern, "", text, flags=re.DOTALL | re.MULTILINE)
        if count != 1:
            errors.append(f"declared extension anchor missing or duplicated: {pattern}")
    return text


CORE_EVENTS = r"\s*#\[derive\(Clone, Copy, Debug, PartialEq, Eq\)\]\s*pub enum ActionPhase \{.*?\}\s*#\[derive\(Clone, Debug, PartialEq\)\]\s*pub struct ActionEvent \{.*?\}\s*#\[derive\(Clone, Debug, PartialEq\)\]\s*pub enum ControllerEvent \{.*?\}\s*"
X11_EVENTS = r"\s*#\[derive\(Clone, Copy, Debug, PartialEq, Eq\)\]\s*pub enum ActionPhase \{.*?\}\s*#\[derive\(Clone, Debug, PartialEq\)\]\s*pub struct ActionEvent \{.*?\}\s*"
X11_PUBLIC_EVENTS = r"\s*pub use flamewm_render_core::\{ActionEvent, ActionPhase, ControllerEvent\};\s*"
X11_CONTROLLER_SEAM = r"\s*pub fn run_with_controller_events<F>\(.*?^\}\s*"
X11_ROLE = r"\s*,\s*pub role: X11WindowRole,\s*"
X11_ROLE_ENUM = r"\s*#\[derive\(Clone, Copy, Debug, PartialEq, Eq\)\]\s*pub enum X11WindowRole \{.*?\}\s*"
X11_ROLE_DEFAULT = r"\s*role:\s*X11WindowRole::Normal,\s*"
X11_ROLE_CALL = r"\s*set_window_role\(display, window, config\.role\);\s*"
X11_ROLE_FUNCTION = r"\s*unsafe fn set_window_role\(.*?\n\}(?=\n\nimpl Drop)\s*"
XLIB_PROPERTIES = r"\s*pub const PROP_MODE_REPLACE: c_int = 0;\s*pub const XA_ATOM: Atom = 4;\s*"
XLIB_PROPERTY_FUNCTION = r"\s*pub fn XChangeProperty\(.*?\)\s*->\s*c_int;\s*"


def exact(
    ref: str,
    active: str,
    extensions: tuple[tuple[str, str], ...] = (),
    ref_extensions: tuple[str, ...] = (),
    active_extensions: tuple[str, ...] = (),
) -> None:
    ref_path = REF / ref
    active_path = ROOT / active
    if not ref_path.is_file():
        errors.append(f"vendor promotion anchor missing: {ref}")
        return
    if not active_path.is_file():
        errors.append(f"canonical promoted owner missing: {active}")
        return
    r = strip_patterns(translate(ref_path.read_text(), extensions), ref_extensions)
    a = strip_patterns(active_path.read_text(), active_extensions)
    if normalized_tokens(r) != normalized_tokens(a):
        errors.append(f"promoted owner drifted beyond declared translations: {active} <- {ref}")


if not (REF / "Cargo.lock").is_file() or 'version = "0.0.9"' not in (REF / "Cargo.lock").read_text():
    errors.append("vendor manifest is not RustWebRender 0.0.9")

for rel in ["layout.rs", "paint.rs"]:
    exact(
        f"crates/rustwebrender-core/src/{rel}",
        f"crates/flamewm-render-core/src/{rel}",
        tuple(NAMESPACE_TRANSLATIONS.items()),
    )
exact(
    "crates/rustwebrender-core/src/lib.rs",
    "crates/flamewm-render-core/src/lib.rs",
    tuple(NAMESPACE_TRANSLATIONS.items()),
    active_extensions=(CORE_EVENTS,),
)
exact(
    "crates/rustwebrender-core/src/codec.rs",
    "crates/flamewm-render-core/src/codec.rs",
    tuple(NAMESPACE_TRANSLATIONS.items())
    + (
        ("not a RustWebRender compiled document", "not a FlameWM Render compiled document"),
        ("unsupported RustWebRender format version", "unsupported FlameWM Render format version"),
    ),
)

for rel in ["compile.rs", "css.rs", "html.rs", "lib.rs"]:
    exact(
        f"crates/rustwebrender-compiler/src/{rel}",
        f"crates/flamewm-render-compiler/src/{rel}",
        tuple(NAMESPACE_TRANSLATIONS.items()) + (("RustWebRender", "FlameWM Render"),),
    )
exact(
    "crates/rustwebrender-compiler/src/bin/rwr-inspect.rs",
    "crates/flamewm-render-compiler/src/bin/flamewm-render-inspect.rs",
    tuple(NAMESPACE_TRANSLATIONS.items())
    + (("rwr-inspect", "flamewm-render-inspect"),),
)

# X11 support owners retain the upstream implementation with only the
# declared Flame diagnostics/environment namespace and edition-safe FFI form.
exact(
    "crates/rustwebrender-x11/src/lib.rs",
    "crates/flamewm-render-x11/src/lib.rs",
    tuple(NAMESPACE_TRANSLATIONS.items())
    + (
        ("RustWebRender", "FlameWM Render"),
        ("RWR_ACTION", "FLAMEWM_RENDER_ACTION"),
        ("RWR_TEXT_BACKEND", "FLAMEWM_RENDER_TEXT_BACKEND"),
         ("RWR_ALPHA_BACKEND", "FLAMEWM_RENDER_ALPHA_BACKEND"),
     ),
    ref_extensions=(X11_EVENTS,),
    active_extensions=(X11_PUBLIC_EVENTS, X11_CONTROLLER_SEAM, X11_ROLE, X11_ROLE_ENUM, X11_ROLE_DEFAULT, X11_ROLE_CALL, X11_ROLE_FUNCTION),
)
for rel in ["xft.rs", "xlib.rs", "xrender.rs"]:
    exact(
        f"crates/rustwebrender-x11/src/{rel}",
        f"crates/flamewm-render-x11/src/{rel}",
        tuple(NAMESPACE_TRANSLATIONS.items())
    + (
        ("RustWebRender", "FlameWM Render"),
        ('extern "C" {', 'unsafe extern "C" {'),
            ("RWR_FONTCONFIG_WARNING", "FLAMEWM_RENDER_FONTCONFIG_WARNING"),
            ("RWR_UI_FONT", "FLAMEWM_RENDER_UI_FONT"),
            ("RWR_FONT_FILES", "FLAMEWM_RENDER_FONT_FILES"),
         ),
         active_extensions=(XLIB_PROPERTIES, XLIB_PROPERTY_FUNCTION) if rel == "xlib.rs" else (),
     )
exact(
    "crates/rustwebrender-x11/src/bin/rwr-x11.rs",
    "crates/flamewm-render-x11/src/bin/flamewm-render-x11.rs",
    tuple(NAMESPACE_TRANSLATIONS.items())
    + (("rwr-x11:", "flamewm-render-x11:"), ("usage: rwr-x11", "usage: flamewm-render-x11")),
)

# Deliberately extended owners must retain the exact 0.0.9 mechanisms that
# solved the failed experiment's latency/visual defects.
x11 = (ROOT / "crates/flamewm-render-x11/src/lib.rs").read_text()
for anchor in [
    "XEventsQueued(self.display, QUEUED_AFTER_READING)",
    "XPeekEvent(self.display",
    "XSetGraphicsExposures(display, gc, 0)",
    "XCopyArea(\n            self.display,\n            self.backbuffer,\n            self.window,",
    "ActionPhase::Motion",
    "inside: hover == Some(index)",
]:
    if anchor not in x11:
        errors.append(f"extended X11 owner lost required 0.0.9/product anchor: {anchor}")

core = (ROOT / "crates/flamewm-render-core/src/lib.rs").read_text()
for anchor in ["pub enum ActionPhase", "pub struct ActionEvent", "pub enum ControllerEvent"]:
    if anchor not in core:
        errors.append(f"public renderer event seam missing: {anchor}")

for anchor in ["pub enum X11WindowRole", "_NET_WM_WINDOW_TYPE", "XChangeProperty", "run_with_controller_events"]:
    if anchor not in x11 and anchor != "XChangeProperty":
        errors.append(f"X11 Flame extension missing: {anchor}")
    if anchor == "XChangeProperty" and anchor not in (ROOT / "crates/flamewm-render-x11/src/xlib.rs").read_text():
        errors.append(f"X11 Flame extension missing: {anchor}")

if errors:
    for error in errors:
        print(f"ERROR {error}", file=sys.stderr)
    raise SystemExit(1)
print("OK FlameWM Render 0.0.9 provenance promotion")
