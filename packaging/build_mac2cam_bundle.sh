#!/usr/bin/env bash
# Build a self-contained, ad-hoc signed Mac2CAM.app for the current Mac.
# This follows build_macos_signed.sh's launcher, icon, Quick Look, and signing
# layout while intentionally omitting DMG creation and notarization.
set -euo pipefail

cd "$(dirname "$0")/.."
TARGET="aarch64-apple-darwin"
VERSION="${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}"
DIST="dist"
STAGE="target/mac2cam-package"
APP="$DIST/Mac2CAM.app"
APP_ID="com.twodcam.mac2cam"
BUILD_STAMP="${MAC2CAM_BUILD_STAMP:-$(date +%Y%m%d_%H%M%S)}"
export MAC2CAM_BUILD_STAMP

echo "==> Building Mac2CAM_$BUILD_STAMP for $TARGET"
cargo build --release --target "$TARGET" --bin OpenCADStudio --bin ocs_launcher
cargo build --release --target "$TARGET" -p dwg-thumbnailer

echo "==> Creating application and document icons"
rm -rf "$STAGE"
mkdir -p "$STAGE" "$DIST"
ICONSET="$STAGE/Mac2CAM.iconset"
mkdir -p "$ICONSET"
for SIZE in 16 32 64 128 256 512 1024; do
    rsvg-convert -w "$SIZE" -h "$SIZE" assets/logo.svg -o "$ICONSET/icon_${SIZE}x${SIZE}.png"
done
for BASE in 16 32 128 256 512; do
    cp "$ICONSET/icon_$((BASE * 2))x$((BASE * 2)).png" "$ICONSET/icon_${BASE}x${BASE}@2x.png"
done
iconutil -c icns "$ICONSET" -o "$STAGE/AppIcon.icns"

for TYPE in dwg dxf; do
    ICONSET="$STAGE/$TYPE.iconset"
    mkdir -p "$ICONSET"
    for SIZE in 16 32 64 128 256 512 1024; do
        rsvg-convert -w "$SIZE" -h "$SIZE" "assets/mimetypes/image-vnd.$TYPE.svg" \
            -o "$ICONSET/icon_${SIZE}x${SIZE}.png"
    done
    for BASE in 16 32 128 256 512; do
        cp "$ICONSET/icon_$((BASE * 2))x$((BASE * 2)).png" "$ICONSET/icon_${BASE}x${BASE}@2x.png"
    done
    iconutil -c icns "$ICONSET" -o "$STAGE/$(echo "$TYPE" | tr '[:lower:]' '[:upper:]').icns"
done

echo "==> Building Quick Look thumbnail extension"
EXT="$STAGE/DWGThumbnail.appex"
mkdir -p "$EXT/Contents/MacOS"
swiftc \
    -sdk "$(xcrun --sdk macosx --show-sdk-path)" \
    -target arm64-apple-macos11 \
    -O -parse-as-library -application-extension \
    -module-name DWGThumbnail \
    -import-objc-header crates/dwg-thumbnailer/macos/dwg_thumbnailer.h \
    crates/dwg-thumbnailer/macos/ThumbnailProvider.swift \
    -L "target/$TARGET/release" -ldwg_thumbnailer \
    -framework QuickLookThumbnailing -framework CoreGraphics \
    -framework ImageIO -framework Foundation -framework Security \
    -framework SystemConfiguration -liconv \
    -Xlinker -e -Xlinker _NSExtensionMain \
    -o "$EXT/Contents/MacOS/DWGThumbnail"
sed \
    -e "s/__VERSION__/$VERSION/g" \
    -e 's/io.github.HakanSeven12.OpenCadStudio.DWGThumbnail/com.twodcam.mac2cam.DWGThumbnail/g' \
    crates/dwg-thumbnailer/macos/Info.plist > "$EXT/Contents/Info.plist"

echo "==> Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/PlugIns"
cp "target/$TARGET/release/ocs_launcher" "$APP/Contents/MacOS/Mac2CAM"
cp "target/$TARGET/release/OpenCADStudio" "$APP/Contents/MacOS/OpenCADStudio-App"
chmod +x "$APP/Contents/MacOS/Mac2CAM" "$APP/Contents/MacOS/OpenCADStudio-App"
cp "$STAGE/AppIcon.icns" "$STAGE/DWG.icns" "$STAGE/DXF.icns" "$APP/Contents/Resources/"
cp -R "$EXT" "$APP/Contents/PlugIns/"
sed \
    -e "s/__VERSION__/$VERSION/g" \
    -e "s/OCS-__BUILD_STAMP__/Mac2CAM_$BUILD_STAMP/g" \
    -e 's#<string>OpenCADStudio</string>#<string>Mac2CAM</string>#' \
    -e "s/io.github.HakanSeven12.OpenCadStudio/$APP_ID/g" \
    packaging/Info.plist > "$APP/Contents/Info.plist"

echo "==> Ad-hoc signing"
codesign --force --deep --sign - --timestamp=none "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"
plutil -lint "$APP/Contents/Info.plist"

echo "==> Done: $APP"
