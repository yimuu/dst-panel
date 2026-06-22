#!/bin/sh
set -e

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$repo_root"

sh ./build_window.sh
