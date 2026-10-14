#!/bin/sh
# Install alm built from this checkout, and make it the one Terminal uses (Task-A07-15).
# On a Mac there is one link, /usr/local/bin/alm, and the latest install wins: installing Allomos
# (the .pkg, or Install the alm command in the app) points it at the app's own alm; this script
# points it at ~/.cargo/bin/alm. Asks for the Mac password only when the link has to change.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
cargo install --locked --path "$root/crates/cli"
dev="${CARGO_HOME:-$HOME/.cargo}/bin/alm"
link=/usr/local/bin/alm
if [ "$(readlink "$link" 2>/dev/null || true)" != "$dev" ]; then
  echo "Linking $link -> $dev (asks for your Mac password)"
  sudo mkdir -p /usr/local/bin
  sudo ln -sfn "$dev" "$link"
fi
found=$(command -v alm || true)
if [ "$found" != "$link" ]; then
  echo "Note: this shell finds ${found:-no alm} before $link; remove it so the link decides." >&2
fi
"$dev" --version
