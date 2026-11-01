#!/bin/sh
# Compila la versión final y la instala para este usuario, sin sudo.
# Para desinstalar: scripts/install-linux.sh --uninstall
set -eu

BIN="$HOME/.local/bin/convertxyt"
ICON="$HOME/.local/share/icons/hicolor/512x512/apps/convertxyt.png"
DESKTOP="$HOME/.local/share/applications/convertxyt.desktop"

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$BIN" "$ICON" "$DESKTOP"
    update-desktop-database "$(dirname "$DESKTOP")" 2>/dev/null || true
    echo "convertxyt desinstalado"
    exit 0
fi

cd "$(dirname "$0")/.."

npm run tauri build -- --no-bundle

install -Dm755 src-tauri/target/release/convertxyt "$BIN"
install -Dm644 src-tauri/icons/icon.png "$ICON"
mkdir -p "$(dirname "$DESKTOP")"
cat > "$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=convertxyt
Comment=Descarga y convierte videos y audio
Exec=$BIN
Icon=convertxyt
Terminal=false
Categories=AudioVideo;Video;
EOF

update-desktop-database "$(dirname "$DESKTOP")" 2>/dev/null || true
gtk-update-icon-cache -q "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

echo "Instalado: busca convertxyt en tu menú de aplicaciones"
