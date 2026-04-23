#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
UI_DIR="$ROOT_DIR/UI"
DIST_DIR="$UI_DIR/dist/macos"
BUILD_DIR="$UI_DIR/target/macos-package"
STAGING_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cscs-key-ui-macos.XXXXXX")"
APP_NAME="CSCS Key"
APP_BUNDLE_ID="ch.cscs.cscs-key-ui"
APP_EXECUTABLE="cscs-key-ui"
CLI_EXECUTABLE="cscs-key"
APP_ICON_NAME="AppIcon"
APP_VERSION="$(awk -F ' = ' '/^version = / { gsub(/"/, "", $2); print $2; exit }' "$UI_DIR/Cargo.toml")"
SIGNING_MODE="${MACOS_SIGNING_MODE:-auto}"
SIGNING_IDENTITY="${MACOS_CODESIGN_IDENTITY:-}"
NOTARYTOOL_PROFILE="${MACOS_NOTARYTOOL_PROFILE:-${APPLE_NOTARYTOOL_PROFILE:-}}"
APPLE_ID="${APPLE_ID:-}"
APPLE_TEAM_ID="${APPLE_TEAM_ID:-}"
APPLE_APP_SPECIFIC_PASSWORD="${APPLE_APP_SPECIFIC_PASSWORD:-}"
APP_BUNDLE="$STAGING_DIR/${APP_NAME}.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
RESOURCE_BIN_DIR="$RESOURCES_DIR/bin"
ICONSET_DIR="$BUILD_DIR/${APP_ICON_NAME}.iconset"
ICON_PATH="$RESOURCES_DIR/${APP_ICON_NAME}.icns"
INFO_TEMPLATE="$UI_DIR/packaging/macos/Info.plist.template"
INFO_PLIST="$CONTENTS_DIR/Info.plist"
ZIP_PATH="$STAGING_DIR/cscs-key-ui-macos-v${APP_VERSION}.zip"
DIST_ZIP_PATH="$DIST_DIR/cscs-key-ui-macos-v${APP_VERSION}.zip"

cleanup() {
  rm -rf "$STAGING_DIR"
}

trap cleanup EXIT

find_installed_identity() {
  local pattern="$1"

  security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*"\(.*\)"/\1/p' \
    | grep -F "$pattern" \
    | head -n 1 || true
}

resolve_signing_configuration() {
  case "$SIGNING_MODE" in
    auto)
      if [[ -z "$SIGNING_IDENTITY" ]]; then
        SIGNING_IDENTITY="$(find_installed_identity "Developer ID Application:")"
      fi

      if [[ -n "$SIGNING_IDENTITY" ]]; then
        SIGNING_MODE="developer-id"
      elif command -v codesign >/dev/null 2>&1; then
        SIGNING_MODE="adhoc"
      else
        SIGNING_MODE="none"
      fi
      ;;
    developer-id)
      if [[ -z "$SIGNING_IDENTITY" ]]; then
        SIGNING_IDENTITY="$(find_installed_identity "Developer ID Application:")"
      fi

      if [[ -z "$SIGNING_IDENTITY" ]]; then
        echo "Developer ID signing requested, but no 'Developer ID Application:' identity was found." >&2
        echo "Install the certificate in Keychain or set MACOS_CODESIGN_IDENTITY explicitly." >&2
        exit 1
      fi
      ;;
    adhoc|none)
      ;;
    *)
      echo "Unsupported MACOS_SIGNING_MODE: $SIGNING_MODE" >&2
      echo "Expected one of: auto, developer-id, adhoc, none" >&2
      exit 1
      ;;
  esac
}

sign_file() {
  local target="$1"

  case "$SIGNING_MODE" in
    developer-id)
      codesign --force --timestamp --options runtime --sign "$SIGNING_IDENTITY" "$target"
      ;;
    adhoc)
      codesign --force --sign - "$target"
      ;;
    none)
      ;;
  esac
}

can_notarize() {
  if [[ "$SIGNING_MODE" != "developer-id" ]]; then
    return 1
  fi

  if ! command -v xcrun >/dev/null 2>&1 || ! xcrun --find notarytool >/dev/null 2>&1; then
    return 1
  fi

  if [[ -n "$NOTARYTOOL_PROFILE" ]]; then
    return 0
  fi

  [[ -n "$APPLE_ID" && -n "$APPLE_TEAM_ID" && -n "$APPLE_APP_SPECIFIC_PASSWORD" ]]
}

submit_for_notarization() {
  echo "Submitting archive for notarization"

  if [[ -n "$NOTARYTOOL_PROFILE" ]]; then
    xcrun notarytool submit "$ZIP_PATH" --wait --keychain-profile "$NOTARYTOOL_PROFILE"
  else
    xcrun notarytool submit "$ZIP_PATH" --wait --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" --password "$APPLE_APP_SPECIFIC_PASSWORD"
  fi

  echo "Stapling notarization ticket"
  xcrun stapler staple "$APP_BUNDLE"
  xcrun stapler validate "$APP_BUNDLE"
}

mkdir -p "$DIST_DIR" "$BUILD_DIR"
rm -rf "$APP_BUNDLE" "$DIST_DIR/${APP_NAME}.app" "$ICONSET_DIR" "$ZIP_PATH" "$DIST_ZIP_PATH"
mkdir -p "$MACOS_DIR" "$RESOURCE_BIN_DIR"

resolve_signing_configuration

echo "Signing mode: $SIGNING_MODE"
if [[ -n "$SIGNING_IDENTITY" ]]; then
  echo "Signing identity: $SIGNING_IDENTITY"
fi

echo "Building release binaries"
cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml"
cargo build --release --manifest-path "$UI_DIR/Cargo.toml" --bins

echo "Generating macOS icon set"
"$UI_DIR/target/release/make_macos_icon" "$ICONSET_DIR"
iconutil -c icns "$ICONSET_DIR" -o "$ICON_PATH"

echo "Copying bundled binaries"
cp "$UI_DIR/target/release/$APP_EXECUTABLE" "$MACOS_DIR/$APP_EXECUTABLE"
cp "$ROOT_DIR/target/release/$CLI_EXECUTABLE" "$RESOURCE_BIN_DIR/$CLI_EXECUTABLE"
chmod 755 "$MACOS_DIR/$APP_EXECUTABLE" "$RESOURCE_BIN_DIR/$CLI_EXECUTABLE"

echo "Writing Info.plist"
sed \
  -e "s|__APP_NAME__|$APP_NAME|g" \
  -e "s|__APP_EXECUTABLE__|$APP_EXECUTABLE|g" \
  -e "s|__APP_ICON_NAME__|$APP_ICON_NAME|g" \
  -e "s|__APP_BUNDLE_ID__|$APP_BUNDLE_ID|g" \
  -e "s|__APP_VERSION__|$APP_VERSION|g" \
  "$INFO_TEMPLATE" > "$INFO_PLIST"
plutil -lint "$INFO_PLIST" >/dev/null

if command -v xattr >/dev/null 2>&1; then
  xattr -cr "$APP_BUNDLE"
fi

if [[ "$SIGNING_MODE" != "none" ]]; then
  echo "Signing bundled binaries"
  sign_file "$RESOURCE_BIN_DIR/$CLI_EXECUTABLE"
  sign_file "$MACOS_DIR/$APP_EXECUTABLE"

  echo "Signing app bundle"
  sign_file "$APP_BUNDLE"
  codesign --verify --deep --strict "$APP_BUNDLE"
fi

echo "Creating zip archive"
ditto -c -k --sequesterRsrc --keepParent "$APP_BUNDLE" "$ZIP_PATH"

if can_notarize; then
  submit_for_notarization
  echo "Repacking stapled app"
  rm -f "$ZIP_PATH"
  ditto -c -k --sequesterRsrc --keepParent "$APP_BUNDLE" "$ZIP_PATH"
elif [[ "$SIGNING_MODE" == "developer-id" ]]; then
  echo "Developer ID signing completed without notarization credentials." >&2
  echo "Set MACOS_NOTARYTOOL_PROFILE or APPLE_ID + APPLE_TEAM_ID + APPLE_APP_SPECIFIC_PASSWORD to notarize." >&2
fi

cp "$ZIP_PATH" "$DIST_ZIP_PATH"

echo "Created zip archive: $DIST_ZIP_PATH"
echo "Unzip the archive to get ${APP_NAME}.app"