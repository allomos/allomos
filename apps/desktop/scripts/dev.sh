#!/bin/sh
# npm run dev (Task-A07-19): on a Mac, build this checkout's alm and point Terminal's alm at it,
# then start the app with tauri dev, so the alm in Terminal matches the app being run. The latest
# run wins, in the main checkout or a worktree, as the latest install does (Task-A07-15). Asks for
# the Mac password only when the link has to change; if it cannot change it, the app starts anyway.
set -eu
root=$(cd "$(dirname "$0")/../../.." && pwd)
if [ "$(uname)" = Darwin ]; then
  cargo build -p allomos-cli --manifest-path "$root/Cargo.toml"
  alm="$root/target/debug/alm"
  link=/usr/local/bin/alm
  if [ "$(readlink "$link" 2>/dev/null || true)" != "$alm" ]; then
    echo "Linking $link -> $alm (asks for your Mac password)"
    if ! { sudo mkdir -p /usr/local/bin && sudo ln -sfn "$alm" "$link"; }; then
      echo "Note: $link was not changed; Terminal's alm may not match this app." >&2
    fi
  fi
  found=$(command -v alm || true)
  if [ "$found" != "$link" ]; then
    echo "Note: this shell finds ${found:-no alm} before $link; remove it so the link decides." >&2
  fi
  "$alm" --version
fi
exec tauri dev "$@"
