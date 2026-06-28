#!/bin/sh
set -e

# Docker images expect a Linux amd64 binary.
target="${RUST_TARGET:-x86_64-unknown-linux-gnu}"

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required to verify Rust target '$target' before release builds." >&2
  exit 1
fi

if [ "$target" != "x86_64-unknown-linux-gnu" ]; then
  echo "Unsupported Linux release target '$target'. Only x86_64-unknown-linux-gnu is supported." >&2
  exit 1
fi

if ! rustup target list --installed | grep -qx "$target"; then
  echo "Rust target '$target' is not installed. Run: rustup target add $target" >&2
  exit 1
fi

host_target="$(rustc -vV | awk '/^host:/ { print $2 }')"
linker=""
if [ "$host_target" != "$target" ]; then
  linker="${LINUX_LINKER:-x86_64-linux-gnu-gcc}"
fi

if [ -n "$linker" ] && ! command -v "$linker" >/dev/null 2>&1; then
  echo "Linux linker '$linker' is required for Rust target '$target'." >&2
  echo "Install it or set LINUX_LINKER to a compatible linker command." >&2
  exit 1
fi

if [ -n "$linker" ]; then
  export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$linker"
fi

cargo build --release --bin dst-admin-rust --target "$target"
cp "target/$target/release/dst-admin-rust" ./dst-admin-rust
