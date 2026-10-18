#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
DIST="$ROOT/dist"
mkdir -p "$DIST"

targets=(
  x86_64-unknown-linux-musl
  aarch64-unknown-linux-musl
  x86_64-pc-windows-gnu
)

for t in "${targets[@]}"; do
  echo "Building $t..."
  cargo build --release --target "$t"
  case "$t" in
    x86_64-unknown-linux-musl)
      cp "target/$t/release/duplix" "$DIST/duplix-linux-x86_64"
      ;;
    aarch64-unknown-linux-musl)
      cp "target/$t/release/duplix" "$DIST/duplix-linux-aarch64"
      ;;
    x86_64-pc-windows-gnu)
      cp "target/$t/release/duplix.exe" "$DIST/duplix-windows-x86_64.exe"
      ;;
  esac
done

echo "Binarios en $DIST"
ls -lh "$DIST"
