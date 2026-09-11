#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="OpenCADStudio"
BUNDLE_ID="io.github.HakanSeven12.OpenCadStudio.cam"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BUNDLE="$ROOT_DIR/dist-dev/$APP_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_BINARY="$APP_MACOS/OpenCADStudio-App"

pkill -x "$APP_NAME" >/dev/null 2>&1 || true
pkill -x "OpenCADStudio-App" >/dev/null 2>&1 || true

cd "$ROOT_DIR"
cargo build --bin OpenCADStudio --bin ocs_launcher
mkdir -p "$APP_MACOS" "$APP_CONTENTS/Resources"
cp target/debug/ocs_launcher "$APP_MACOS/OpenCADStudio"
cp target/debug/OpenCADStudio "$APP_BINARY"
chmod +x "$APP_MACOS/OpenCADStudio" "$APP_BINARY"
sed \
  -e 's/__VERSION__/dev/g' \
  -e 's/__BUILD_STAMP__/cam/g' \
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
    sleep 2
    pgrep -x "OpenCADStudio-App" >/dev/null
    ;;
  *)
    echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
