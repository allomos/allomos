# Build the unsigned Windows (x64) installer and alm.exe on a Mac, into target/windows/.
# Run from the repository root: bash apps/desktop/scripts/build-windows.sh
# One-time tools: docs/workflow/CODING_RULES.md § Building for Windows.
set -e
# Homebrew's rustup and llvm are keg-only, so they are put on PATH here and nowhere else.
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:/opt/homebrew/opt/llvm/bin:$PATH"
# makensis 3.13 crashes (std::bad_alloc) when no locale is set.
export LC_ALL=en_US.UTF-8 LANG=en_US.UTF-8
T=x86_64-pc-windows-msvc; OUT=target/windows
need() { command -v "$1" >/dev/null || { echo "build-windows: $1 not found. $2" >&2; exit 1; }; }
need rustup "Install: brew install rustup"
need cargo-xwin "Install: cargo +1.85.1 install cargo-xwin --version 0.19.2 --locked"
need makensis "Install: brew install makensis"
need clang-cl "Install: brew install llvm"
rustup target list --installed | grep -qx $T || { echo "build-windows: Rust target $T missing. Install: rustup target add $T" >&2; exit 1; }
[ -d apps/web/node_modules ] || npm --prefix apps/web ci
[ -d apps/desktop/node_modules ] || npm --prefix apps/desktop ci
# alm first: the installer carries it (tauri.windows.conf.json, Task-A07-06), under the name
# Tauri expects for a bundled program.
cargo xwin build --release --target $T -p allomos-cli --bin alm
mkdir -p apps/desktop/src-tauri/binaries
cp target/$T/release/alm.exe apps/desktop/src-tauri/binaries/alm-$T.exe
npm --prefix apps/desktop run tauri build -- --runner cargo-xwin --target $T --bundles nsis
mkdir -p $OUT
cp target/$T/release/bundle/nsis/*-setup.exe target/$T/release/alm.exe $OUT/
ls -l $OUT
