#!/usr/bin/env sh
# Compila Duplix y deja el lanzador local `./start` listo.
# No instala nada en el PATH ni en el home: todo queda dentro de esta carpeta.
#
#   ./start.sh        -> compila y prepara ./start
#   ./start           -> organiza ./input
#   ./start --menu    -> menú interactivo
set -eu

DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

echo "· compilando (release)…"
cargo build --release

mkdir -p "$DIR/dist"
cp "$DIR/target/release/duplix" "$DIR/dist/duplix-linux-x86_64"
chmod +x "$DIR/start" 2>/dev/null || true

echo "✓ listo. Usá:  ./start    o    ./start --menu"
