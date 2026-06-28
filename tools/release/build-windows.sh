#!/bin/sh
set -e

# The GNU target works from Unix shells; override RUST_TARGET for MSVC builds.
target="${RUST_TARGET:-x86_64-pc-windows-gnu}"

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required to verify Rust target '$target' before release builds." >&2
  exit 1
fi

if ! rustup target list --installed | grep -qx "$target"; then
  echo "Rust target '$target' is not installed. Run: rustup target add $target" >&2
  exit 1
fi

if [ "$target" = "x86_64-pc-windows-gnu" ] && ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
  echo "MinGW linker x86_64-w64-mingw32-gcc is required for target '$target'." >&2
  exit 1
fi

cargo build --release --bin dst-admin-rust --target "$target"
cp "target/$target/release/dst-admin-rust.exe" ./dst-admin-rust.exe
