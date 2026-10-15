#!/bin/sh
# Check pkg/postinstall on a fake volume, never the real Mac (Task-A07-13). Each case builds a folder
# that stands for the disk Installer installs on (`$3`), with /Applications/Allomos.app and, in
# some cases, another Allomos.app elsewhere, then runs postinstall the way Installer does
# (`$1` the package, `$2` a folder, `$3` the volume) and checks where usr/local/bin/alm leads.
#   apps/desktop/pkg/test-postinstall.sh [scratch folder]   (default: a new temporary folder)
set -eu
script=$(cd "$(dirname "$0")" && pwd)/postinstall
scratch=${1:-$(mktemp -d)}
want=/Applications/Allomos.app/Contents/MacOS/alm
fails=0

fake_app() { # fake_app <volume> <folder holding Allomos.app>
  mkdir -p "$1$2/Allomos.app/Contents/MacOS"
  printf '#!/bin/sh\n' >"$1$2/Allomos.app/Contents/MacOS/alm"
  chmod +x "$1$2/Allomos.app/Contents/MacOS/alm"
}

# case <name> <what is at the link: none|other-app|dev|file> <$2> [no-app]
case_() {
  vol="$scratch/$1"
  rm -rf "$vol"
  mkdir -p "$vol/usr/local/bin"
  [ "${4:-}" = no-app ] || fake_app "$vol" /Applications
  fake_app "$vol" /Users/me/allomos/target/release/bundle/macos
  other=/Users/me/allomos/target/release/bundle/macos/Allomos.app/Contents/MacOS/alm
  case $2 in
    other-app) ln -s "$other" "$vol/usr/local/bin/alm" ;;
    dev) ln -s /Users/me/.cargo/bin/alm "$vol/usr/local/bin/alm" ;;
    file) printf 'old\n' >"$vol/usr/local/bin/alm" ;;
  esac
  before=$(readlink "$vol/usr/local/bin/alm" 2>/dev/null || true)
  sh "$script" /tmp/Allomos.pkg "$3" "$vol" / || { echo "FAIL $1: postinstall exited non-zero"; fails=$((fails + 1)); return; }
  got=$(readlink "$vol/usr/local/bin/alm" 2>/dev/null || echo "(not a link)")
  expect=$want
  [ "${4:-}" = no-app ] && expect=${before:-(not a link)}
  if [ "$got" = "$expect" ]; then echo "PASS $1: alm -> $got"; else echo "FAIL $1: alm -> $got, wanted $expect"; fails=$((fails + 1)); fi
}

case_ empty-other-copy-in-2 none /Users/me/allomos/target/release/bundle/macos
case_ link-to-other-copy other-app /Users/me/allomos/target/release/bundle/macos
case_ link-to-other-copy-2-is-root other-app /
case_ link-to-other-copy-2-is-applications other-app /Applications
case_ link-to-dev-build dev ""
case_ real-file file /Applications
case_ app-missing-keeps-link other-app /Applications no-app

# Volume given with a trailing slash, as Installer may pass it.
vol="$scratch/trailing-slash"
rm -rf "$vol"; mkdir -p "$vol"; fake_app "$vol" /Applications
sh "$script" /tmp/Allomos.pkg /Applications "$vol/" /
got=$(readlink "$vol/usr/local/bin/alm" || true)
if [ "$got" = "$want" ]; then echo "PASS trailing-slash: alm -> $got"; else echo "FAIL trailing-slash: alm -> $got"; fails=$((fails + 1)); fi

[ "$fails" -eq 0 ] && echo "All postinstall cases passed ($scratch)" || { echo "$fails case(s) failed ($scratch)"; exit 1; }
