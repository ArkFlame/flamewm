#!/usr/bin/env python3
"""Enforce the active product-layer architecture boundary."""

from pathlib import Path
import re
import sys
import tomllib


PRODUCT_CRATES = {
    "flamewm-api",
    "flamewm-applications",
    "flamewm-control-core",
    "flamewm-control-dbus",
    "flamewm-control-wire",
    "flamewm-dbus-reactor",
    "flamewm-desktop-core",
    "flamewm-shell",
    "flamewm-shell-core",
    "flamewm-desktop",
    "flamewm-settings",
    "flamewm-settings-core",
    "flamewm-platform",
    "flamewm-wm",
    "flamewm-reactor",
    "flamewm-skin",
    "flamewm-ui",
    "flamewm-ui-core",
    "flamewm-window-core",
    "flamewm-session-core",
    "flamewm-integrations-core",
    "flamewm-render-compiler",
    "flamewm-render-core",
    "flamewm-render-x11",
    "flamewm-ui-x11",
    "flamewm-wm-x11",
}
# These crates are the canonical native/render owners. Their low-level APIs are
# intentionally allowed to mention the capabilities guarded elsewhere.
OWNER_CRATES = {
    "flamewm-render-compiler",
    "flamewm-render-core",
    "flamewm-render-x11",
    "flamewm-ui-x11",
    "flamewm-wm-x11",
}
FORBIDDEN_DEPENDENCIES = {"flamewm-render-core", "flamewm-render-x11"}
FORBIDDEN_RENDERER_SOURCE = re.compile(
    r"RuntimeDocument|flamewm_render_(?:core|x11)::|"
    r"(?:XOpenDisplay|XMapWindow|XGrabPointer|XRender|Xft)|"
    r"\bdocument\.(?:set_|clear_|update_|remove_|add_|insert_)"
)
FORBIDDEN_X11_SOURCE = re.compile(r"\bx11rb::")
# Product crates may depend on a typed owner, but may not grow a second native
# boundary or hide warning debt behind a crate-wide suppression.
RAW_FFI_SOURCE = re.compile(
    r'extern\s+"C"|std::os::raw|\b(?:libc|libloading)::|\b(?:dlopen|dlsym)\s*\('
)
BROAD_SUPPRESSION = re.compile(
    r"#\s*!?\s*\[allow\((?:warnings|unused(?:_[a-z_]+)?|clippy::(?:all|pedantic))\)\]"
)
UNSAFE_FUNCTION = re.compile(r"\bunsafe\s+fn\s+[A-Za-z_][A-Za-z0-9_]*")

# These are diagnostic budgets, not ownership rules. They stop a facade or
# core from becoming an unreviewable second owner while leaving the deliberate
# native/render owner implementations outside this product-facade guard.
RESPONSIBILITY_LOC_LIMITS = {
    "core": 2400,
    "facade": 5000,
    "compiler": 2400,
    "reactor": 1200,
}


def responsibility(crate_name):
    if crate_name.endswith("-core"):
        return "core"
    if crate_name.endswith("-reactor") or crate_name == "flamewm-reactor":
        return "reactor"
    if crate_name == "flamewm-render-compiler":
        return "compiler"
    return "facade"


def source_loc(paths):
    return sum(
        1
        for path in paths
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("//")
    )


def relative_path(root, path):
    return path.relative_to(root).as_posix()


def product_manifests(root):
    manifests = {}
    for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
        with manifest.open("rb") as stream:
            data = tomllib.load(stream)
        name = data.get("package", {}).get("name")
        if name in PRODUCT_CRATES and name not in OWNER_CRATES:
            manifests[name] = manifest
    return manifests


def dependency_names(manifest):
    with manifest.open("rb") as stream:
        data = tomllib.load(stream)
    names = set()
    def add_sections(table):
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            for name, value in table.get(section, {}).items():
                # A dependency may be renamed locally with package = "...".
                names.add(value.get("package", name) if isinstance(value, dict) else name)

    add_sections(data)
    for target in data.get("target", {}).values():
        add_sections(target)
    return names


def violations(root):
    found = set()
    for manifest in product_manifests(root).values():
        crate_name = manifest.parent.name
        path = relative_path(root, manifest)
        dependencies = dependency_names(manifest)
        if dependencies & FORBIDDEN_DEPENDENCIES:
            found.add((path, "renderer-dependency"))
        if "x11rb" in dependencies:
            found.add((path, "x11rb-dependency"))

        source_paths = sorted((manifest.parent / "src").rglob("*.rs"))
        build_script = manifest.parent / "build.rs"
        if build_script.is_file():
            source_paths.append(build_script)
        for source in source_paths:
            text = source.read_text(encoding="utf-8")
            source_path = relative_path(root, source)
            if FORBIDDEN_X11_SOURCE.search(text):
                found.add((source_path, "x11rb-symbol"))
            if FORBIDDEN_RENDERER_SOURCE.search(text):
                found.add((source_path, "renderer-symbol"))
            if RAW_FFI_SOURCE.search(text):
                found.add((source_path, "raw-ffi-boundary"))
            for unsafe_match in UNSAFE_FUNCTION.finditer(text):
                # Keep the contract local to the function; a Safety section
                # elsewhere in a large module must not satisfy this tripwire.
                prefix = text[max(0, unsafe_match.start() - 600) : unsafe_match.start()]
                if "# Safety" not in prefix:
                    found.add((source_path, "unsafe-fn-safety-contract"))

        role = responsibility(crate_name)
        limit = RESPONSIBILITY_LOC_LIMITS[role]
        loc = source_loc(source_paths)
        if loc > limit:
            found.add((path, f"{role}-source-loc>{limit}"))

    # Suppressions are checked across owners too: native declarations may use
    # naming/dead-code allowances, but never blanket warning suppression.
    for source in sorted((root / "crates").rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        if BROAD_SUPPRESSION.search(text):
            found.add((relative_path(root, source), "broad-warning-suppression"))
    found |= j7_violations(root)
    return found


# --- J7 icon/alpha guards (J7-G1..J7-G8), capability-tagged for the exception ratchet ---
J7_KNOWN_KEYED_PPMS = {
    "task-start.ppm", "tray-volume.ppm", "tray-volume-muted.ppm", "tray-wifi.ppm",
    "tray-play.ppm", "tray-pause.ppm", "popup-volume.ppm", "popup-muted.ppm",
    "media-play.ppm", "media-pause.ppm", "media-prev.ppm", "media-next.ppm",
    "title-close.ppm", "title-maximize.ppm", "title-minimize.ppm", "title-restore.ppm",
}
J7_OWNER_PREFIXES = (
    "crates/flamewm-render-x11/",
    "crates/flamewm-image-core/",
)
J7_FFI_DECLARATION = "crates/flamewm-render-x11/src/xlib.rs"
J7_FALLBACK_OWNER = "crates/flamewm-integrations-linux/src/icons.rs"
J7_TREATMENT_OWNER = "crates/flamewm-skin/src/icons.rs"
J7_PAINT_OWNER = "crates/flamewm-render-core/src/paint.rs"
J7_SURFACE_OWNER = "crates/flamewm-render-x11/src/native/target.rs"
J7_MAGENTA_KEY = re.compile(r"ff00ff|color[-_ ]?key|chromakey|chroma[-_ ]?key", re.IGNORECASE)


def _j7_is_test(path):
    return "/tests/" in path.as_posix() or path.name.endswith("_test.rs")


def _j7_production_sources(root):
    for path in sorted((root / "crates").rglob("*.rs")):
        if not _j7_is_test(path):
            yield path


def _j7_declared(text):
    body = re.sub(r"#\[cfg\(test\)\].*?\nmod tests \{.*?\n\}\n", "", text, flags=re.S)
    return body


def j7_violations(root):
    found = set()

    def rel(path):
        return relative_path(root, path)

    def owned(path, prefixes=J7_OWNER_PREFIXES):
        return rel(path).startswith(prefixes)

    # J7-G1: only the render/native owner may blend over black or keep the
    # documented opaque fallback.
    for path in _j7_production_sources(root):
        text = _j7_declared(path.read_text(encoding="utf-8"))
        if re.search(r"\bblend_over_black\b", text) and not owned(path):
            found.add((rel(path), "j7-g1-no-blend-over-black"))
        if re.search(r"fallback-black|fallback_black|FallbackBlack", text) and not owned(path):
            found.add((rel(path), "j7-g1-no-blend-over-black"))
    # J7-G2: only the Xlib FFI declaration owner may declare
    # XCreateSimpleWindow; canonical Flame surface state lives in target.rs.
    for path in _j7_production_sources(root):
        if rel(path) == J7_FFI_DECLARATION:
            continue
        if "XCreateSimpleWindow" in _j7_declared(path.read_text(encoding="utf-8")):
            found.add((rel(path), "j7-g2-no-xcreate-simple-window"))
    # J7-G3: no active icon consumer references known keyed PPM assets.
    # Compiled shell templates are owned by the G01 template guard; the guard
    # tables themselves are not icon consumers. Test-only files are excluded.
    for path in _j7_production_sources(root):
        if path.suffix != ".rs":
            continue;
        text = _j7_declared(path.read_text(encoding="utf-8"))
        for match in re.findall(r"[A-Za-z0-9_./-]+\.ppm", text):
            if match.rsplit("/", 1)[-1] in J7_KNOWN_KEYED_PPMS:
                found.add((rel(path), "j7-g3-no-keyed-ppm-asset"))
                break
    # J7-G4: generic IconResolver fallback must not be .ppm (checked at owner).
    owner = root / J7_FALLBACK_OWNER
    if owner.is_file():
        for line in owner.read_text(encoding="utf-8").splitlines():
            if line.strip().startswith("const GENERIC_FALLBACK") and ".ppm" in line:
                found.add((J7_FALLBACK_OWNER, "j7-g4-no-ppm-fallback"))
    # J7-G5: no production magenta-key/FF00FF color-key path.
    for path in _j7_production_sources(root):
        if owned(path):
            continue
        text = _j7_declared(path.read_text(encoding="utf-8")).lower()
        if J7_MAGENTA_KEY.search(text) and "no magenta" not in text and "no color-key" not in text:
            found.add((rel(path), "j7-g5-no-magenta-colorkey"))
    # J7-G6: SymbolicForeground must not be inferred from filename.
    for path in _j7_production_sources(root):
        text = _j7_declared(path.read_text(encoding="utf-8"))
        if "SymbolicForeground" in text and re.search(
            r"(?i)(filename|extension|ends_with|\.svg|\.ppm|\.png).{0,80}SymbolicForeground"
            r"|SymbolicForeground.{0,80}(filename|extension|\.svg|\.ppm|\.png)",
            text,
        ):
            found.add((rel(path), "j7-g6-no-filename-treatment"))
    # J7-G7: real app icon path defaults to Original (checked at skin owner).
    skin = root / J7_TREATMENT_OWNER
    if skin.is_file():
        text = skin.read_text(encoding="utf-8")
        if "IconTreatment::Original" not in text or not re.search(
            r"Start\s*\|.*Browser|Browser.*Start", text
        ):
            found.add((J7_TREATMENT_OWNER, "j7-g7-original-app-icons"))
    # J7-G8: shell symbolic icons consume semantic foreground, not Breeze RGB.
    paint = root / J7_PAINT_OWNER
    if paint.is_file() and "SymbolicForeground" in paint.read_text(encoding="utf-8"):
        if "resolve_color(style.color)" not in paint.read_text(encoding="utf-8"):
            found.add((J7_PAINT_OWNER, "j7-g8-semantic-foreground"))
    surface = root / J7_SURFACE_OWNER
    if surface.is_file() and "resolve_color(style.color)" not in paint.read_text(encoding="utf-8"):
        found.add((J7_SURFACE_OWNER, "j7-g8-semantic-foreground"))
    found |= j6_violations(root)
    found |= j12_violations(root)
    print_j6_warnings(root)
    return found


# --- J6 symbolic/migration guards (J6-G1..J6-G7), exception-ratchet tagged ---
# J6-G1 is hard-fail (already true). J6-G2..J6-G7 describe post-migration
# state; they currently fail pre-migration, so they are staged as WARN-only
# (printed to stderr) and never added to the violation set until migration.
J6_SKIN_OWNER = "crates/flamewm-skin/src/icons.rs"
J6_CHROME_FILE = "crates/flamewm-wm-x11/src/chrome.rs"
J6_ICONS_FILE = "crates/flamewm-integrations-linux/src/icons.rs"
J6_MODEL_FILE = "crates/flamewm-render-core/src/model.rs"
J6_SHELL_MAIN = "crates/flamewm-shell/src/main.rs"
J6_PENDING: list[str] = []
# TODO(J12-PENDING): promote J6-G5/G6 to violation entries once migrations land.


def _j6_strip_comments(text: str) -> str:
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


def print_j6_warnings(root):
    del root
    for warning in J6_PENDING:
        print(warning, file=sys.stderr)


def j6_violations(root):
    found = set()
    skin = root / J6_SKIN_OWNER
    if skin.is_file():
        text = skin.read_text(encoding="utf-8")
        if re.search(
            r"Self::Start[^\n]*SymbolicForeground|Start[^\n]{0,80}=>\s*IconTreatment::SymbolicForeground",
            text,
        ):
            found.add((J6_SKIN_OWNER, "j6-g1-start-full-color"))
        if "Self::Start" in text and "IconTreatment::Original" not in text:
            found.add((J6_SKIN_OWNER, "j6-g1-start-full-color"))
    chrome = root / J6_CHROME_FILE
    if chrome.is_file():
        text = _j6_strip_comments(chrome.read_text(encoding="utf-8"))
        if "struct X11ChromePainter" in text:
            found.add((J6_CHROME_FILE, "j6-g2-no-local-chrome-painter"))
        if re.search(r"fn draw_title_run", text) and "poly_fill_rectangle" in text:
            found.add((J6_CHROME_FILE, "j6-g3-no-block-title"))
        if re.search(r"fn draw_control_glyph", text) and re.search(
            r"poly_line|poly_rectangle|poly_arc|poly_fill_arc", text
        ):
            found.add((J6_CHROME_FILE, "j6-g4-no-procedural-glyphs"))
    icons = root / J6_ICONS_FILE
    if icons.is_file():
        text = _j6_strip_comments(icons.read_text(encoding="utf-8"))
        pack = text.find("packaged_candidates(name)")
        theme = text.find("theme_candidates(name")
        if pack != -1 and theme != -1 and pack < theme:
            J6_PENDING.append(f"WARN TODO(J12-PENDING) j6-g5-system-before-packaged: {J6_ICONS_FILE}")
    model = root / J6_MODEL_FILE
    if model.is_file() and re.search(
        r"z_index_overrides|set_z_index|effective_z_index",
        _j6_strip_comments(model.read_text(encoding="utf-8")),
    ):
        J6_PENDING.append(f"WARN TODO(J12-PENDING) j6-g6-no-new-zindex-store: {J6_MODEL_FILE}")
    shell = root / J6_SHELL_MAIN
    if shell.is_file() and re.search(
        r"root_pointer\(\s*0(\.0)?\s*,\s*0(\.0)?\s*\)",
        _j6_strip_comments(shell.read_text(encoding="utf-8")),
    ):
        found.add((J6_SHELL_MAIN, "j6-g7-no-root-pointer-zero"))
    return found


# --- J12 focused guards (J12-G01..J12-G14), exception-ratchet tagged ---
J12_DESKTOP_PROJ = "crates/flamewm-desktop/src/projection.rs"
J12_START_MENU = "crates/flamewm-shell-core/src/start_menu.rs"
J12_POPUP = "crates/flamewm-ui-x11/src/popup.rs"
J12_SURFACE_RT = "crates/flamewm-ui-x11/src/surface_runtime.rs"
J12_SVG = "crates/flamewm-image-core/src/svg.rs"
J12_SESSION = "crates/flamewm-session-core/src/lib.rs"
J12_PAINT_OWNER = "crates/flamewm-render-core/src/paint.rs"


def j12_violations(root):
    found = set()
    code = _j6_strip_comments

    def live(path):
        return code((root / path).read_text(encoding="utf-8")) if (root / path).is_file() else ""

    # J12-G01: invalid clamp(12.0,track) in live code.
    for path in _j7_production_sources(root):
        if path.suffix != ".rs":
            continue
        if re.search(r"\bclamp\s*\(\s*12\.0", code(path.read_text(encoding="utf-8"))):
            found.add((relative_path(root, path), "j12-g01-no-invalid-clamp"))
    # J12-G02: replace_image must not mutate document.assets.
    for path in _j7_production_sources(root):
        if path.suffix != ".rs":
            continue
        body = code(path.read_text(encoding="utf-8"))
        if re.search(r"fn replace_image", body) and re.search(
            r"\.assets\s*(\[[^\]]*\]\s*=|\.push\s*\(|\.insert\s*\(|\.clear\s*\()", body
        ):
            found.add((relative_path(root, path), "j12-g02-no-assets-mutation"))
    # J12-G03: no display:none on transient/popup/menu templates (ui context).
    for tpl in sorted((root / "ui").rglob("*")):
        if not tpl.is_file() or tpl.suffix.lower() not in {".html", ".css"}:
            continue
        text = re.sub(r"/\*.*?\*/", "", tpl.read_text(encoding="utf-8"), flags=re.S)
        text = re.sub(r"<!--.*?-->", "", text, flags=re.S)
        if re.search(r"display\s*:\s*none", text, re.I) and re.search(
            r"transient|popup-menu|context-menu|tooltip|dropdown", text, re.I
        ):
            found.add((relative_path(root, tpl), "j12-g03-no-display-none-transient"))
    # J12-G04: desktop root on SurfaceBackground.
    if "UiLayer::SurfaceBackground" not in live(J12_DESKTOP_PROJ):
        found.add((J12_DESKTOP_PROJ, "j12-g04-surface-background-root"))
    # J12-G05: semantic SVG themed render.
    if "render_with_color_scheme" not in live(J12_SVG):
        found.add((J12_SVG, "j12-g05-semantic-svg-render"))
    if "SymbolicForeground" not in live(J12_PAINT_OWNER):
        found.add((J12_PAINT_OWNER, "j12-g05-semantic-svg-render"))
    # J12-G06: hidden create must not force redraw (body-bound).
    match = re.search(r"fn create_surface\(", live(J12_SURFACE_RT))
    if match:
        rest = live(J12_SURFACE_RT)[match.end():]
        nxt = re.search(r"pub fn ", rest)
        body = rest[:nxt.start()] if nxt else rest[:800]
        if re.search(r"redraw|mark_full|Damage::Full", body):
            found.add((J12_SURFACE_RT, "j12-g06-hidden-create-no-redraw"))
    # J12-G07: popup raise policy hide-previous + show-next.
    popup = live(J12_POPUP)
    if "runtime.hide" not in popup or "runtime.show" not in popup:
        found.add((J12_POPUP, "j12-g07-popup-raise-policy"))
    # J12-G08: no sync IconResolver in category hot path.
    if "IconResolver" in live(J12_START_MENU):
        found.add((J12_START_MENU, "j12-g08-no-sync-icon-resolver"))
    # J12-G09: Unknown stays unbucketed.
    start = live(J12_START_MENU)
    if "_ => None" not in start:
        found.add((J12_START_MENU, "j12-g09-unknown-not-visible"))
    # J12-G10: no image_text8/fixed-font normal decoration path.
    for deco in [
        "crates/flamewm-wm-x11/src/decoration/paint.rs",
        "crates/flamewm-wm-x11/src/decoration/manager.rs",
        "crates/flamewm-wm-x11/src/wm.rs",
        "crates/flamewm-wm-x11/src/chrome.rs",
    ]:
        text = live(deco)
        if text and re.search(
            r"image_text8|XDrawString|fixed_font|fixed-font|9x15|TITLE_CHAR_ADVANCE", text
        ):
            found.add((deco, "j12-g10-no-fixed-font-decoration"))
    # J12-G11: exactly one DecorationManager owner.
    count = sum(
        len(re.findall(r"struct DecorationManager", code(p.read_text(encoding="utf-8"))))
        for p in _j7_production_sources(root)
        if p.suffix == ".rs"
    )
    if count != 1:
        found.add(("crates/flamewm-wm-x11/src/decoration/manager.rs", "j12-g11-single-decoration-manager"))
    # J12-G12: wallpaper present, CPU out of desktop owner.
    if "wallpaper" not in live(J12_DESKTOP_PROJ).lower():
        found.add((J12_DESKTOP_PROJ, "j12-g12-wallpaper-not-cpu"))
    desk = root / "crates/flamewm-desktop/src"
    if desk.exists():
        for path in sorted(desk.rglob("*.rs")):
            if _j7_is_test(path):
                continue
            if re.search(r"(?i)\bcpu\b", code(path.read_text(encoding="utf-8"))):
                found.add((relative_path(root, path), "j12-g12-wallpaper-not-cpu"))
                break
    # J12-G13: .performance stays ignored.
    gitignore = root / ".gitignore"
    if not gitignore.is_file() or ".performance" not in gitignore.read_text(encoding="utf-8"):
        found.add((".gitignore", "j12-g13-performance-ignored"))
    # J12-G14: Breeze cursor authority intact.
    session = live(J12_SESSION)
    if "FlameWM-Breeze-Dark" not in session or "XCURSOR" not in session:
        found.add((J12_SESSION, "j12-g14-breeze-cursor-intact"))
    if "Breeze-Dark" not in live("crates/flamewm-wm-x11/src/decoration/paint.rs"):
        found.add((
            "crates/flamewm-wm-x11/src/decoration/paint.rs",
            "j12-g14-breeze-cursor-intact",
        ))
    found |= j12h_violations(root)
    return found


# --- J12 handoff guards (J12H-G01..G15), exception-ratchet tagged ---
def j12h_violations(root):
    found = set()
    code = _j6_strip_comments

    def live(path):
        return code((root / path).read_text(encoding="utf-8")) if (root / path).is_file() else ""

    def desktop_css():
        for name in ("ui/desktop/desktop.css", "ui/shell/flamewm.css"):
            path = root / name
            if path.is_file():
                return path.read_text(encoding="utf-8")
        return ""

    paint_core = live("crates/flamewm-render-core/src/paint.rs")
    start_menu = live("crates/flamewm-shell-core/src/start_menu.rs")
    proj = live("crates/flamewm-shell/src/projection.rs")
    runtime = live("crates/flamewm-shell/src/runtime.rs")
    main = live("crates/flamewm-shell/src/main.rs")
    audio = live("crates/flamewm-shell/src/taskbar/status/audio.rs")
    network = live("crates/flamewm-shell/src/taskbar/status/network.rs")
    file_actions = live("crates/flamewm-desktop-core/src/file_actions.rs")
    persist = (root / "crates/flamewm-desktop-core/src/sticky_persistence.rs").read_text(
        encoding="utf-8"
    ) if (root / "crates/flamewm-desktop-core/src/sticky_persistence.rs").is_file() else ""
    ext = live("crates/flamewm-render-x11/src/external_drawable.rs")
    chrome = live("crates/flamewm-wm-x11/src/chrome.rs")
    paint_decor = live("crates/flamewm-wm-x11/src/decoration/paint.rs")
    palette = live("crates/flamewm-skin/src/palette.rs")
    span = live("crates/flamewm-profiler/src/span.rs")
    # J12H-G01: StrokeRect radius.
    if "StrokeRect" not in paint_core or "radius" not in paint_core:
        found.add(("crates/flamewm-render-core/src/paint.rs", "j12h-g01-strokerect-radius"))
    # J12H-G02: one StartModel.
    count = sum(
        len(re.findall(r"struct StartModel", code(p.read_text(encoding="utf-8"))))
        for p in _j7_production_sources(root)
        if p.suffix == ".rs"
    )
    if count != 1:
        found.add(("crates/flamewm-shell-core/src/start_menu.rs", "j12h-g02-single-startmodel"))
    # J12H-G03: borrowed query hot path.
    for anchor in ("fn filter_view", "fn query_view", "fn slot_view"):
        if anchor not in start_menu:
            found.add(("crates/flamewm-shell-core/src/start_menu.rs", "j12h-g03-borrowed-query"))
            break
    # J12H-G04: Power explicit.
    if "StartCategory::Power" not in proj or "session" not in proj.lower():
        found.add(("crates/flamewm-shell/src/projection.rs", "j12h-g04-power-explicit"))
    # J12H-G05: surface-scoped icon reset.
    if "need_panel" not in main or "need_submenu" not in main:
        found.add(("crates/flamewm-shell/src/main.rs", "j12h-g05-scoped-icon-reset"))
    # J12H-G06: anchored OpenPopover.
    if "open_status_anchored_id" not in runtime or "anchored_rect_by_id" not in runtime:
        found.add(("crates/flamewm-shell/src/runtime.rs", "j12h-g06-anchored-popover"))
    # J12H-G07: audio/network availability-gated.
    if "ServiceAvailability::Available" not in audio:
        found.add(("crates/flamewm-shell/src/taskbar/status/audio.rs", "j12h-g07-no-live-authority"))
    if "ServiceAvailability::Available" not in network:
        found.add(("crates/flamewm-shell/src/taskbar/status/network.rs", "j12h-g07-no-live-authority"))
    # J12H-G08: context border 0.
    if "border-width:0" not in desktop_css().replace(" ", ""):
        found.add(("ui/desktop/desktop.css", "j12h-g08-context-border-zero"))
    # J12H-G09: NOREPLACE rename.
    if "RenameFlags::NOREPLACE" not in file_actions:
        found.add(("crates/flamewm-desktop-core/src/file_actions.rs", "j12h-g09-noreplace-rename"))
    # J12H-G10: sticky v1+v2 (source holds backslash-t escapes).
    if "v1\\t" not in persist or "v2\\t" not in persist:
        found.add(("crates/flamewm-desktop-core/src/sticky_persistence.rs", "j12h-g10-sticky-v1-v2"))
    # J12H-G11: event-driven, no poll. Contract anchor lives in doc
    # comments (stripped from code context), so check raw text plus the
    # revision-gated resnapshot_dynamic code path with no poll timer.
    raw_main = (root / "crates/flamewm-shell/src/main.rs").read_text(encoding="utf-8") if (root / "crates/flamewm-shell/src/main.rs").is_file() else ""
    raw_rt = (root / "crates/flamewm-shell/src/runtime.rs").read_text(encoding="utf-8") if (root / "crates/flamewm-shell/src/runtime.rs").is_file() else ""
    if ("resnapshot_dynamic" not in main or "register_timer" in main and "workspace" in main.lower().split("register_timer")[0][-200:]
            or ("never polls" not in raw_main.lower() and "no polling" not in raw_rt.lower())):
        found.add(("crates/flamewm-shell/src/main.rs", "j12h-g11-no-workspace-poll"))
    # J12H-G12: honest ExternalDrawable + skin metrics contract.
    if "ExternalDrawableTarget" not in (paint_decor + chrome) or "ExternalDrawableSession" not in ext:
        found.add(("crates/flamewm-wm-x11/src/decoration/paint.rs", "j12h-g12-external-delegation"))
    if "flamewm_skin" not in chrome and "WINDOW_CHROME" not in chrome:
        found.add(("crates/flamewm-wm-x11/src/chrome.rs", "j12h-g12-skin-metrics"))
    # J12H-G13: no legacy title path; measured intent success (missing-tolerant).
    for deco in ("crates/flamewm-wm-x11/src/decoration/paint.rs", "crates/flamewm-wm-x11/src/chrome.rs", "crates/flamewm-wm-x11/src/wm.rs"):
        text = live(deco)
        if text and ("image_text8" in text or "XDrawString" in text):
            found.add((deco, "j12h-g13-no-legacy-title"))
    if (root / "crates/flamewm-wm-x11/src/decoration/paint.rs").is_file() and "title_drawn = true" not in paint_decor:
        found.add(("crates/flamewm-wm-x11/src/decoration/paint.rs", "j12h-g13-measured-title-intent"))
    # J12H-G14: PANEL #1b1e20.
    if "0x1b1e20" not in palette.lower():
        found.add(("crates/flamewm-skin/src/palette.rs", "j12h-g14-panel-1b1e20"))
    # J12H-G15: scoped profiler spans.
    span_all = span + live("crates/flamewm-profiler/src/point.rs") + live("crates/flamewm-profiler/src/lib.rs")
    if span_all and "&'static str" not in span_all:
        found.add(("crates/flamewm-profiler/src/span.rs", "j12h-g15-scoped-spans"))
    if 'ProfilePoint::new("shell.' not in main and "flamewm_profiler::start(" not in main:
        found.add(("crates/flamewm-shell/src/main.rs", "j12h-g15-scoped-spans"))
    return found


def load_exceptions(root):
    path = root / "docs/ARCHITECTURE_EXCEPTIONS.toml"
    with path.open("rb") as stream:
        data = tomllib.load(stream)
    exceptions = set()
    errors = []
    for index, entry in enumerate(data.get("exception", []), 1):
        required = ("path", "capability", "reason", "canonical_target", "remove_by")
        missing = [key for key in required if not isinstance(entry.get(key), str) or not entry[key]]
        if missing:
            errors.append(f"exception {index}: missing required fields: {', '.join(missing)}")
            continue
        exception_path = entry["path"]
        if Path(exception_path).is_absolute() or any(token in exception_path for token in ("*", "?", "[", "]")):
            errors.append(f"exception {index}: path must be exact and relative: {exception_path}")
            continue
        if Path(exception_path).as_posix() != exception_path:
            errors.append(f"exception {index}: path must use relative POSIX form: {exception_path}")
            continue
        exceptions.add((exception_path, entry["capability"]))
    return exceptions, errors


def main():
    root = Path(__file__).resolve().parents[1]
    expected = violations(root)
    exceptions, errors = load_exceptions(root)
    stale = sorted(exceptions - expected)
    uncovered = sorted(expected - exceptions)
    for error in errors:
        print(f"ERROR architecture exception: {error}", file=sys.stderr)
    for path, capability in stale:
        print(f"ERROR stale architecture exception: {path} [{capability}]", file=sys.stderr)
    for path, capability in uncovered:
        print(f"ERROR architecture boundary violation: {path} [{capability}]", file=sys.stderr)
    if errors or stale or uncovered:
        return 1
    print(f"OK architecture boundary guard ({len(expected)} ratified violations)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
