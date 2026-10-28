#!/usr/bin/env sh
set -eu
DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$DIR/target/release"
PORTABLE="$DIR/portable"

mkdir -p "$PORTABLE"/input "$PORTABLE"/json

cp "$BUILD_DIR/duplix" "$PORTABLE/duplix"
cp "$DIR/start" "$PORTABLE/start"
cp "$DIR/start.sh" "$PORTABLE/start.sh" || true
cp "$DIR/README.md" "$PORTABLE/"
cp "$DIR/Duplix_CLI_Plan.md" "$PORTABLE/" || true

chmod +x "$PORTABLE/duplix" "$PORTABLE/start"

cat > "$PORTABLE/README_PORTABLE.txt" <<'EOF'
Duplix Portable
===============
- Ejecuta ./start o ./duplix directamente desde este USB/disco.
- Todo el historial queda en ./json dentro de esta carpeta.
- Por defecto escanea ./input. Puedes indicar otra ruta:
  ./start --path "/ruta/a/escanear"
- Funciona en Linux/macOS. Para Windows usa start.bat y el binario duplix.exe.
EOF

echo "Portable listo en $PORTABLE"
