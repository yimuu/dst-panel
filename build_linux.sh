#!/bin/sh
set -e

# Docker images expect a Linux binary; override RUST_TARGET for arm64 images.
target="${RUST_TARGET:-x86_64-unknown-linux-gnu}"

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required to verify Rust target '$target' before release builds." >&2
  exit 1
fi

if ! rustup target list --installed | grep -qx "$target"; then
  echo "Rust target '$target' is not installed. Run: rustup target add $target" >&2
  exit 1
fi

host_target="$(rustc -vV | awk '/^host:/ { print $2 }')"
linker=""
case "$target" in
  x86_64-unknown-linux-gnu)
    if [ "$host_target" != "$target" ]; then
      linker="${LINUX_LINKER:-x86_64-linux-gnu-gcc}"
    fi
    ;;
  aarch64-unknown-linux-gnu)
    if [ "$host_target" != "$target" ]; then
      linker="${LINUX_LINKER:-aarch64-linux-gnu-gcc}"
    fi
    ;;
esac

if [ -n "$linker" ] && ! command -v "$linker" >/dev/null 2>&1; then
  echo "Linux linker '$linker' is required for Rust target '$target'." >&2
  echo "Install it or set LINUX_LINKER to a compatible linker command." >&2
  exit 1
fi

if [ -n "$linker" ]; then
  case "$target" in
    x86_64-unknown-linux-gnu)
      export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$linker"
      ;;
    aarch64-unknown-linux-gnu)
      export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$linker"
      ;;
  esac
fi

cargo build --release --bin dst-admin-rust --target "$target"
cp "target/$target/release/dst-admin-rust" ./dst-admin-rust
