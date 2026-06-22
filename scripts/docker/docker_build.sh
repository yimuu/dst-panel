#!/bin/bash
set -e

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
cd "$repo_root"

./docker_build.sh "$@"
