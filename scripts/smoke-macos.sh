#!/usr/bin/env bash
set -euo pipefail

target=${1:?Pass the Rust target triple}
shopt -s nullglob
images=(src-tauri/target/"$target"/release/bundle/dmg/*.dmg)
if [[ ${#images[@]} -ne 1 ]]; then
  echo 'Expected one DMG installer' >&2
  exit 1
fi
mount_dir=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/prism-relay-mount.XXXXXX")
mounted=false
cleanup() {
  if [[ $mounted == true ]]; then
    hdiutil detach "$mount_dir" >/dev/null
  fi
  rmdir "$mount_dir"
}
trap cleanup EXIT
hdiutil attach -readonly -nobrowse -noautoopen -mountpoint "$mount_dir" "${images[0]}"
mounted=true
app="$mount_dir/Prism Relay.app"
test -d "$app"
test -L "$mount_dir/Applications"
codesign --verify --strict --deep --verbose=2 "$app"
cmp "$app/Contents/Resources/LICENSE" LICENSE
cmp "$app/Contents/Resources/THIRD_PARTY_NOTICES.txt" THIRD_PARTY_NOTICES.txt
cmp "$app/Contents/Resources/icon.icns" src-tauri/icons/icon.icns
identifier=$(plutil -extract CFBundleIdentifier raw -o - "$app/Contents/Info.plist")
test "$identifier" = 'io.github.shake1227.prismrelay'
camera_description=$(plutil -extract NSCameraUsageDescription raw -o - "$app/Contents/Info.plist")
test -n "$camera_description"
case "$target" in
  aarch64-apple-darwin) expected_arch=arm64 ;;
  x86_64-apple-darwin) expected_arch=x86_64 ;;
  *) echo 'Unsupported macOS architecture' >&2; exit 1 ;;
esac
test "$(lipo -archs "$app/Contents/MacOS/prism-relay")" = "$expected_arch"
echo 'macOS installer contains the expected app, architecture, icon, license resources, and valid bundle signature'
