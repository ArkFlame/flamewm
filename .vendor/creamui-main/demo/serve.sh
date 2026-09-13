#!/usr/bin/env sh
# Serves the browser demos as static files, so a page's fullscreen canvas
# and its click/keyboard handling behave like they would on a real host
# (opening index.html directly via file:// blocks the WASM fetch).
#
# Serves demo/ (this script's own directory) so every demo/<name>/ is
# reachable at once -- unless the current directory is itself named
# "demo", in which case that directory is served instead (so `cd demo &&
# ./serve.sh` and `./demo/serve.sh` from the repo root behave the same).
#
# Usage:
#   demo/serve.sh [port]   # default port 8080
set -eu

script_dir=$(cd "$(dirname "$0")" && pwd)
port="${1:-8080}"

if [ "$(basename "$(pwd)")" = "demo" ]; then
  serve_dir=$(pwd)
else
  serve_dir="$script_dir"
fi

echo "Serving $serve_dir at http://localhost:$port"
echo "Run demo/build.sh first if a demo's pkg/ directory is missing."
exec python3 -m http.server "$port" --directory "$serve_dir"
