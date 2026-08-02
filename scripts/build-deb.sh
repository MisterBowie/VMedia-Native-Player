#!/usr/bin/env sh
set -eu
umask 022

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT_DIR"

APP_NAME=vmedia-player
APP_ID=io.vmedia.native-player
PACKAGE_NAME=vmedia-player
ARCH=$(dpkg --print-architecture)
VERSION=$(
  cargo metadata --no-deps --format-version 1 \
    | sed -n 's/.*"version":"\([^"]*\)".*/\1/p' \
    | head -n 1
)

if [ -z "$VERSION" ]; then
  echo "Failed to determine package version." >&2
  exit 1
fi

MAINTAINER_NAME=$(git config user.name 2>/dev/null || printf '%s' "VMedia Maintainers")
MAINTAINER_EMAIL=$(git config user.email 2>/dev/null || printf '%s' "noreply@example.com")
MAINTAINER=${DEB_MAINTAINER:-"$MAINTAINER_NAME <$MAINTAINER_EMAIL>"}

PKG_STAGING="$ROOT_DIR/release-pkg/deb/${PACKAGE_NAME}_${VERSION}_${ARCH}"
PKG_OUTPUT="$ROOT_DIR/release-pkg/${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"
SHLIBDEPS_DIR=$(mktemp -d)

cleanup() {
  rm -rf "$SHLIBDEPS_DIR"
}

trap cleanup EXIT INT TERM HUP

rm -rf "$PKG_STAGING"
mkdir -p "$PKG_STAGING/DEBIAN"
mkdir -p "$PKG_STAGING/usr/bin"
mkdir -p "$PKG_STAGING/usr/share/applications"
mkdir -p "$PKG_STAGING/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$PKG_STAGING/usr/share/icons/hicolor/128x128/apps"
mkdir -p "$PKG_STAGING/usr/share/icons/hicolor/64x64/apps"
mkdir -p "$PKG_STAGING/usr/share/icons/hicolor/48x48/apps"
mkdir -p "$PKG_STAGING/usr/share/doc/$PACKAGE_NAME"

install -Dm755 target/release/native-player "$PKG_STAGING/usr/bin/$APP_NAME"
install -Dm644 "resources/$APP_ID.desktop" "$PKG_STAGING/usr/share/applications/$APP_ID.desktop"
install -Dm644 "resources/icons/hicolor/256x256/apps/vmedia.png" "$PKG_STAGING/usr/share/icons/hicolor/256x256/apps/vmedia.png"
install -Dm644 "resources/icons/hicolor/128x128/apps/vmedia.png" "$PKG_STAGING/usr/share/icons/hicolor/128x128/apps/vmedia.png"
install -Dm644 "resources/icons/hicolor/64x64/apps/vmedia.png" "$PKG_STAGING/usr/share/icons/hicolor/64x64/apps/vmedia.png"
install -Dm644 "resources/icons/hicolor/48x48/apps/vmedia.png" "$PKG_STAGING/usr/share/icons/hicolor/48x48/apps/vmedia.png"
install -Dm644 "resources/icons/hicolor/256x256/apps/vmedia.png" "$PKG_STAGING/usr/share/icons/hicolor/256x256/apps/$APP_ID.png"
install -Dm644 "resources/icons/hicolor/128x128/apps/vmedia.png" "$PKG_STAGING/usr/share/icons/hicolor/128x128/apps/$APP_ID.png"
install -Dm644 "resources/icons/hicolor/64x64/apps/vmedia.png" "$PKG_STAGING/usr/share/icons/hicolor/64x64/apps/$APP_ID.png"
install -Dm644 "resources/icons/hicolor/48x48/apps/vmedia.png" "$PKG_STAGING/usr/share/icons/hicolor/48x48/apps/$APP_ID.png"
install -Dm644 README.md "$PKG_STAGING/usr/share/doc/$PACKAGE_NAME/README.md"
install -Dm644 README_ZH.md "$PKG_STAGING/usr/share/doc/$PACKAGE_NAME/README_ZH.md"
install -Dm644 LICENSE "$PKG_STAGING/usr/share/doc/$PACKAGE_NAME/LICENSE"

cat > "$PKG_STAGING/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi
EOF

cat > "$PKG_STAGING/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi
EOF

chmod 0755 "$PKG_STAGING/DEBIAN/postinst" "$PKG_STAGING/DEBIAN/postrm"
find "$PKG_STAGING" -type d -exec chmod 0755 {} +

mkdir -p "$SHLIBDEPS_DIR/debian"
cat > "$SHLIBDEPS_DIR/debian/control" <<EOF
Source: $PACKAGE_NAME
Section: video
Priority: optional
Maintainer: $MAINTAINER
Standards-Version: 4.6.2

Package: $PACKAGE_NAME
Architecture: $ARCH
Description: Modern GTK4/libmpv video player
 A lightweight native Linux video player built with Rust, GTK4,
 Libadwaita, and libmpv.
EOF
touch "$SHLIBDEPS_DIR/debian/substvars"

DEPENDS=$(
  cd "$SHLIBDEPS_DIR" && dpkg-shlibdeps -O -Tdebian/substvars "$PKG_STAGING/usr/bin/$APP_NAME" \
    | sed -n 's/^shlibs:Depends=//p'
)

if [ -z "$DEPENDS" ]; then
  echo "Failed to determine shared library dependencies." >&2
  exit 1
fi

INSTALLED_SIZE=$(du -sk "$PKG_STAGING/usr" | cut -f1)

cat > "$PKG_STAGING/DEBIAN/control" <<EOF
Package: $PACKAGE_NAME
Version: $VERSION
Section: video
Priority: optional
Architecture: $ARCH
Maintainer: $MAINTAINER
Depends: $DEPENDS
Installed-Size: $INSTALLED_SIZE
Homepage: https://github.com/MisterBowie/VMedia-Native-Player
Description: Modern GTK4/libmpv video player
 A lightweight native Linux video player built with Rust, GTK4,
 Libadwaita, and libmpv.
EOF

chmod 0644 "$PKG_STAGING/DEBIAN/control"
rm -f "$PKG_OUTPUT"
dpkg-deb --build --root-owner-group "$PKG_STAGING" "$PKG_OUTPUT"
printf '%s\n' "$PKG_OUTPUT"
