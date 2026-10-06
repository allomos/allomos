# Rebuild every app icon from icons/source.svg (48 px and up) and icons/source-small.svg (16–32 px).
# Run from the repository root: bash apps/desktop/scripts/make-icons.sh
# Needs rsvg-convert and ImageMagick (Homebrew: librsvg, imagemagick) and macOS iconutil.
set -e
I=apps/desktop/src-tauri/icons; T=$(mktemp -d)
r() { rsvg-convert -w $2 -h $2 $1 -o $3; }
r $I/source.svg 1024 $I/source.png
npx --prefix apps/desktop tauri icon $I/source.png -o $I >/dev/null
rm -rf $I/android $I/ios
r $I/source-small.svg 32 $I/32x32.png; r $I/source-small.svg 30 $I/Square30x30Logo.png
mkdir $T/icon.iconset
r $I/source-small.svg 16 $T/icon.iconset/icon_16x16.png; r $I/source-small.svg 32 $T/icon.iconset/icon_16x16@2x.png
r $I/source-small.svg 32 $T/icon.iconset/icon_32x32.png; r $I/source.svg 64 $T/icon.iconset/icon_32x32@2x.png
for n in 128 256 512; do r $I/source.svg $n $T/icon.iconset/icon_${n}x${n}.png; r $I/source.svg $((n*2)) $T/icon.iconset/icon_${n}x${n}@2x.png; done
iconutil -c icns $T/icon.iconset -o $I/icon.icns
for n in 16 24 32; do r $I/source-small.svg $n $T/ico$n.png; done
for n in 48 64 256; do r $I/source.svg $n $T/ico$n.png; done
magick $T/ico16.png $T/ico24.png $T/ico32.png $T/ico48.png $T/ico64.png $T/ico256.png $I/icon.ico
rm -rf $T
