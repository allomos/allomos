#!/bin/sh
# Wrap a built Allomos.app into Allomos_<version>_aarch64.pkg (Task-A07-12): the installer puts the
# app in /Applications, then pkg/postinstall links /usr/local/bin/alm to the alm inside it.
# Unsigned, like the .dmg. Run from anywhere, after `tauri build` has made the app:
#   apps/desktop/scripts/build-mac-pkg.sh [path/to/Allomos.app] [output folder]
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)              # apps/desktop
repo=$(cd "$here/../.." && pwd)
app=${1:-$repo/target/release/bundle/macos/Allomos.app}
out=${2:-$repo/target/release/bundle/pkg}
[ -x "$app/Contents/MacOS/alm" ] || { echo "No alm inside $app; build the app with tauri build first." >&2; exit 1; }
version=$(plutil -extract CFBundleShortVersionString raw "$app/Contents/Info.plist")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/root" "$work/scripts" "$out"
ditto "$app" "$work/root/Allomos.app"
cp "$here/pkg/postinstall" "$work/scripts/postinstall"
chmod +x "$work/scripts/postinstall"
# Installed where it is meant to go, even if an Allomos.app exists elsewhere on the Mac.
pkgbuild --analyze --root "$work/root" "$work/component.plist" >/dev/null
plutil -replace 0.BundleIsRelocatable -bool NO "$work/component.plist"
pkg="$out/Allomos_${version}_aarch64.pkg"
pkgbuild --quiet --root "$work/root" --component-plist "$work/component.plist" \
  --install-location /Applications --scripts "$work/scripts" \
  --identifier io.allomos.pkg --version "$version" "$pkg"
# Check what was built (Task-A07-13): Installer must put the app in /Applications and never move
# it onto another Allomos.app it finds (no <relocate> entry), and postinstall must be the one here.
pkgutil --expand "$pkg" "$work/expanded"
info="$work/expanded/PackageInfo"
grep -q 'install-location="/Applications"' "$info" || { echo "$pkg: install-location is not /Applications" >&2; exit 1; }
grep -q '<relocate>' "$info" && { echo "$pkg: Allomos.app is relocatable" >&2; exit 1; }
cmp -s "$work/expanded/Scripts/postinstall" "$here/pkg/postinstall" || { echo "$pkg: postinstall differs from pkg/postinstall" >&2; exit 1; }
echo "$pkg"
