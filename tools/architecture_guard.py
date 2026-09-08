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
            found.add((path, f"{role}-source-loc>{limit} ({loc})"))

    # Suppressions are checked across owners too: native declarations may use
    # naming/dead-code allowances, but never blanket warning suppression.
    for source in sorted((root / "crates").rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        if BROAD_SUPPRESSION.search(text):
            found.add((relative_path(root, source), "broad-warning-suppression"))
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
