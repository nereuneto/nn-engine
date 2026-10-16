#!/bin/sh
# Monta o AppImage do NN Studio a partir dos executáveis em target/release.
#   empacotar/linux/appimage.sh 0.1.0
# Precisa do appimagetool no PATH (ou em $APPIMAGETOOL).
set -eu
VERSAO="$1"
RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
APPDIR="$RAIZ/target/NN-Studio.AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"
for b in nn-studio nn nn-jogo; do
  install -m 755 "$RAIZ/target/release/$b" "$APPDIR/usr/bin/$b"
done
install -m 755 "$RAIZ/empacotar/linux/AppRun" "$APPDIR/AppRun"
install -m 644 "$RAIZ/empacotar/linux/nn-studio.desktop" "$APPDIR/nn-studio.desktop"
install -m 644 "$RAIZ/empacotar/linux/nn-studio.desktop" "$APPDIR/usr/share/applications/nn-studio.desktop"
install -m 644 "$RAIZ/crates/nn-studio/assets/icone.png" "$APPDIR/nn-studio.png"
install -m 644 "$RAIZ/crates/nn-studio/assets/icone.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/nn-studio.png"
mkdir -p "$APPDIR/usr/share/doc/nn-studio"
cp "$RAIZ/LICENSE-MIT" "$RAIZ/LICENSE-APACHE" "$RAIZ/CHANGELOG.md" "$APPDIR/usr/share/doc/nn-studio/"
mkdir -p "$RAIZ/dist"
ARCH=x86_64 "${APPIMAGETOOL:-appimagetool}" --no-appstream "$APPDIR" "$RAIZ/dist/NN-Studio-$VERSAO-x86_64.AppImage"
