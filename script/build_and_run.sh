#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="Mac2CAM"
BUNDLE_ID="com.twodcam.mac2cam.dev"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BUNDLE="$ROOT_DIR/dist-dev/$APP_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_BINARY="$APP_MACOS/OpenCADStudio-App"

pkill -f "$APP_BUNDLE/Contents/MacOS/" >/dev/null 2>&1 || true

cd "$ROOT_DIR"
cargo build --bin OpenCADStudio --bin ocs_launcher
mkdir -p "$APP_MACOS" "$APP_CONTENTS/Resources"
cp target/debug/ocs_launcher "$APP_MACOS/Mac2CAM"
cp target/debug/OpenCADStudio "$APP_BINARY"
chmod +x "$APP_MACOS/Mac2CAM" "$APP_BINARY"
sed \
  -e 's/__VERSION__/dev/g' \
  -e 's/OCS-__BUILD_STAMP__/Mac2CAM/g' \
  -e 's#<string>OpenCADStudio</string>#<string>Mac2CAM</string>#' \
  -e "s/io.github.HakanSeven12.OpenCadStudio/$BUNDLE_ID/g" \
  packaging/Info.plist > "$APP_CONTENTS/Info.plist"

open_app() {
  /usr/bin/open -n "$APP_BUNDLE"
}

case "$MODE" in
  run)
    open_app
    ;;
  --debug|debug)
    lldb -- "$APP_BINARY"
    ;;
  --logs|logs)
    open_app
    /usr/bin/log stream --info --style compact --predicate 'process == "OpenCADStudio-App"'
    ;;
  --telemetry|telemetry)
    open_app
    /usr/bin/log stream --info --style compact --predicate "subsystem == \"$BUNDLE_ID\""
    ;;
  --verify|verify)
    open_app
    for _ in {1..20}; do
      if pgrep -f -x "$APP_BINARY" >/dev/null; then
        exit 0
      fi
      sleep 0.5
    done
    echo "Mac2CAM GUI did not become ready within 10 seconds" >&2
    exit 1
    ;;
  *)
    echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
