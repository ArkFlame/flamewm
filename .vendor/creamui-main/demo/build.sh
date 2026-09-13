#!/usr/bin/env sh
# Builds every browser demo under demo/ to a self-contained
# HTML/JS/WASM bundle in demo/<name>/pkg/, ready for demo/serve.sh.
#
# A "demo" is any subdirectory of demo/ with its own Cargo.toml (a
# wasm-bindgen cdylib crate, same shape as demo/showcase) plus an
# index.html that loads its pkg/. Add a new one and this script picks
# it up automatically -- nothing to register here.
#
# Usage:
#   demo/build.sh        # optimized release build (what you'd ship)
#   demo/build.sh --dev   # fast, unoptimized build for local iteration
set -eu

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "error: wasm-pack not found. Install it: https://rustwasm.github.io/wasm-pack/installer/" >&2
  exit 1
fi

profile_flag="--release"
opt_flag=""
if [ "${1:-}" = "--dev" ]; then
  profile_flag="--dev"
  opt_flag="--no-opt"
fi

script_dir=$(cd "$(dirname "$0")" && pwd)
found=0

for demo_toml in "$script_dir"/*/Cargo.toml; do
  [ -f "$demo_toml" ] || continue
  found=1
  demo_dir=$(dirname "$demo_toml")
  name=$(basename "$demo_dir")
  echo "==> building $name ($profile_flag)"
  if [ -n "$opt_flag" ]; then
    wasm-pack build "$demo_dir" --target web "$profile_flag" "$opt_flag" --out-dir pkg
  else
    wasm-pack build "$demo_dir" --target web "$profile_flag" --out-dir pkg
  fi
done

if [ "$found" -eq 0 ]; then
  echo "no demos found under $script_dir (expected a demo/<name>/Cargo.toml)" >&2
  exit 1
fi

echo "==> done. Serve with: $script_dir/serve.sh"
