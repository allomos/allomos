# Coding rules

A **contract** for code in this repository. Created by task `A01-01`. Extend it in the card that needs a new rule. A card is a task in Allomos's taskbag: its detail, and the evidence of each check (PASS / FAIL / NOT RUN, numbers, logs) under an **Evidence** heading, go in the task's description (`alm describe`); older tasks also keep a card file beside the task in the taskbag (`tasks/<id>/card.md`). There are no card files in this repository.

## Layout

| Where | What | Language |
|---|---|---|
| `crates/core` | Project rules: model and validation. **No disk, network or UI code.** | Rust |
| `crates/store` | Reading and writing taskbags on disk. Never runs `git` by itself. | Rust |
| `crates/cli` | The `allomos` / `alm` command: reads options, calls the crates, prints results. **No rules of its own.** | Rust |
| `apps/desktop/src-tauri` | The Tauri desktop app: thin commands that call the crates. | Rust |
| `apps/web` | The interface. Talks to Rust **only through `src/messenger.ts`**. | TypeScript + React |

Project rules live in Rust. The interface displays results and sends requests; it never decides whether a change is valid.

## Versions

- **Rust 1.85.1**, pinned in `rust-toolchain.toml`. **Tauri 2.11.x**: Tauri 2.12 needs Rust 1.90, so moving up is its own card.
- Cargo uses resolver 3 with `rust-version = "1.85"`, so dependencies are chosen to fit that Rust. `Cargo.lock` is committed.
- JavaScript packages are pinned to exact versions. `@tauri-apps/api` must match the Tauri crate's minor version (2.11), and plugin packages must match their crates. npm is used; there is no pnpm.
- **Command and locking packages** (task `A01-03`): `clap` =4.6.7 (MIT or Apache-2.0; needs Rust 1.85) reads `alm`'s options; `fs4` =1.1.0 (MIT or Apache-2.0; needs Rust 1.75) gives the file lock that Rust's own `File::lock` gives from 1.89. Switch to `File::lock` when Rust moves up.
- Never upgrade a dependency on the side. Upgrade it on purpose, in a card, with its licence noted.

## Checks before a commit

From the repository root:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix apps/web run typecheck
npm --prefix apps/web run build
npm --prefix apps/web run check:i18n
npm --prefix apps/web test
```

All must pass. Report anything not run as NOT RUN.

## Writing code

- **Problems are values, not crashes.** Reading a taskbag returns what could be read plus a list of problems. Each problem names the file and field, and explains it in plain words: what is wrong and how to fix it.
- **Never overwrite user content.** Creating a taskbag into a non-empty folder is refused. Writing files keeps the format in [FILE-FORMAT.md](../product/FILE-FORMAT.md).
- **Tests use temporary folders and synthetic data only**, never the owner's real projects.
- **Test folders made by hand or by an agent go under `~/allomos-tests/`** (owner, 2026-10-09; renamed from `~/fliphopper-tests/` 2026-10-11): one folder per check, named after the card, for example `~/allomos-tests/a05-02-bakery/`. This covers folders for the owner's manual checks and agent trials, never the system's temporary folder or the repository. Automated tests (`cargo test`) keep using the system's temporary folder and remove what they create. Delete a check's folder once the card is done.
- **Interface tests** (task `A05-03`): decisions the interface makes (which step or button to show) live in plain TypeScript modules with only type imports, such as `apps/web/src/setupFlow.ts`, and are tested in `apps/web/tests/*.test.ts` with Node's own test runner (`node --experimental-strip-types --test`, Node 22.6 or newer). No test packages are installed.
- **Keep interface text short and plain**, and keep its terms consistent with the [glossary](../GLOSSARY.md): taskbag, area, state, label.

## Interface languages

The interface speaks English, Simplified Chinese and Japanese (R94). English is the reference.

- **No interface text in the code.** Every text the interface shows is a key in `apps/web/src/i18n/en.json`, `zh-CN.json` and `ja.json`, shown with `t()` (or `rich()` when a placeholder holds an element). The user's own data, file paths and field names are not translated.
- **The product's name in Chinese text** (owner, 2026-10-12): in a Chinese sentence that explains something, write it as 「换肩」, in corner brackets; where the name stands alone (the top bar, a label, a heading) it is 换肩 without them. Japanese text does the same with **肩代** (owner, 2026-10-13): 「肩代」 in a sentence, 肩代 alone.
- **Write English first, and add Chinese and Japanese in the same edit.** Keep each text short and plain, and keep the same `{placeholders}` in every language.
- **Rust sends codes, not sentences.** A problem is a `Problem` in `crates/core/src/problem.rs`: a code, its details, and one English wording for logs and `alm`. After adding or changing a code, run `ALLOMOS_WRITE_CODES=1 cargo test -p allomos-core` to update `apps/web/src/i18n/codes.json`, then add `problem.<code>` to every language file.
- **The checks catch slips:** `typecheck` fails when a language file misses a key; `check:i18n` also fails on extra or empty texts, different placeholders, and Rust codes without words; `cargo test` fails when `codes.json` is out of date.
- The computer's language is read in Rust (`sys-locale` 0.3.2, MIT or Apache-2.0), so it does not depend on what the macOS window reports. The user's own choice is kept in the window's storage.
- **Formatting:** `cargo fmt` defaults for Rust; two-space indentation and double quotes for TypeScript.

## Build output (`target/`)

Rust puts everything it builds in `target/`, one per checkout (the main checkout and each worktree). A Tauri app's `target/` reaches several gigabytes, so these rules keep disk use to about one `target/` per active card (owner, 2026-10-06).

- **Development builds keep line numbers only.** The workspace `Cargo.toml` sets `[profile.dev] debug = "line-tables-only"`: crash reports and test failures still name the file and line, but the full information a debugger needs is not kept. When you really need a debugger (`lldb`), turn it on for that one build only, `CARGO_PROFILE_DEV_DEBUG=true cargo build`, and do not commit a change to the setting.
- **One `target/` per checkout.** Do not set a shared `CARGO_TARGET_DIR`: parallel sessions would wait on each other's builds and keep rebuilding over each other's changes.
- **No routine `cargo clean`.** It throws away work that the next build must redo. Run it only in your own worktree, and only when builds behave strangely after a Rust or Tauri upgrade, or when the disk is short of space.
- **Remove a worktree when its card is merged.** This deletes its `target/` with it. `alm guide workflow` says when (step 2, Branches).
- Build in the checkout you work in: the main checkout by default, or your card's worktree when the task has one (`alm claim` says which branch it starts from and merges into).

## How `alm` behaves

A **contract** for code that changes `alm` (task `A01-03`); `allomos` and `alm` are the same program. What each command does is told by `alm` itself (`alm guide commands`), not repeated here.

- **Version check before every command** (task `A01-25`): except `init`, `brief`, `guide`, `confirm` and `move-in`, a taskbag in a newer format stops the command (`taskbag_newer`, exit 4), and one written by an older Allomos, in any older format, refuses every command, reading ones too, and changes nothing (`taskbag_to_move_in`, exit 1: its tasks are moved in from the app, or with `alm move-in`'s request; task `A01-45`); once an agent is connected, an out-of-date Allomos section in the project's `AGENTS.md` / `CLAUDE.md` adds the warning `instructions_outdated`. The section's heading carries its version, `## Tasks: Allomos (v14)` (`SECTION_VERSION` in `crates/store/src/versions.rs`): raise it whenever the section's wording changes.
- **Holder rules.** `label`, `wait`, `done` and `release` are refused (`task_held`) on a task someone else holds; an unheld task may be changed by anyone. A finished or dropped task counts as held by nobody: the holder it keeps is history (approver, 2026-10-16, task `A00-30`).
- **`alm` never runs or reads Git; it prints the Git commands** ([0017](../decisions/0017-agents-run-git-not-alm.md), task `A01-38`; `--commit` / `--push` were removed, and are refused as unknown options). After every command that changes the taskbag, `alm` prints "Save this change in your version control tool and share it. Changed files:", the files, then "For Git:" and three commands (general wording since task `A01-60`) naming **exactly the files that change wrote, added or removed** (relative to the taskbag; the store records them, `allomos_store::with_written_files`): `git -C "<taskbag>" add -- <files>`, `git -C "<taskbag>" commit -m "alm: <command> <id>" -m "Agent: <holder>" -- <files>` and `git -C "<taskbag>" push`, then "Save only these files; leave other changes as they are." The agent runs them. `--json` gives `changed_files`, `git_commands` and `git_instruction`. `alm claim` says to push now, before the work starts, because until then other sessions do not see the claim. A change that changes nothing names no files and no commands. `alm init` prints none. Changes made in the app since the last `alm` change (its events) come with their own commands, to commit as the person's (`-m "By: <name> (Allomos app)"`; `--json`: `app_changes`), in the next changing command's output and in `alm next`.
- **`--json`** prints exactly one line on standard output and nothing on the error stream: `{"schema":"alm/1","ok":true,"command":…,"taskbag":…,"task"|"area"|"areas":…,"warnings":[…],"issues":[…]}`, or `{"schema":"alm/1","ok":false,"command":…,"error":{"code","params","message"}}`. Problems use the codes in `crates/core/src/problem.rs`; `issues` lists problems found in the taskbag's files. A new field is an addition; any other change makes it `alm/2`.
- **Exit codes:** `0` done · `1` refused by a rule · `2` wrong usage (`bad_usage`) · `3` the taskbag is busy or could not be read or written · `4` the taskbag needs a newer Allomos (`taskbag_newer`).
- **Never waits for typed input.** Messages are English (Iteration 1); their codes are worded in all three interface languages.
- `ALLOMOS_LOCK_WAIT_MS` changes how long a writer waits for the lock (default 5000); tests use it.
- **Install on a Mac:** the app carries `alm` (`Allomos.app/Contents/MacOS/alm`, put there by `tauri.macos.conf.json`, which builds it before bundling), and `/usr/local/bin/alm` is a link to it, so updating the app updates `alm`. **One link, and the latest install wins** (task `A07-15`): the `.pkg` points it at `/Applications/Allomos.app/Contents/MacOS/alm` whatever is there, a fixed path that nothing Installer passes can change, even with another Allomos.app on the Mac (`apps/desktop/pkg/postinstall`, checked on fake volumes by `apps/desktop/pkg/test-postinstall.sh`; `build-mac-pkg.sh` refuses a package that is relocatable or not installed to `/Applications`, task `A07-13`); after a `.dmg` install, or when the link leads elsewhere, the app's banner says which `alm` is in use (none, a development build, another program), shows that path and the app's own, and offers **Install the alm command** or **Use this Allomos's alm** (asks for the Mac password; "Don't ask again" is remembered) (task `A07-12`). **Developing:** `scripts/install-dev-alm.sh` runs `cargo install --locked --path crates/cli` (`allomos` and `alm` into `~/.cargo/bin`) and points `/usr/local/bin/alm` at `~/.cargo/bin/alm` (asks for the Mac password only when the link changes); installing the app again points it back. Keep no other `alm` link ahead of it on `PATH` (such as `~/.local/bin/alm`): `which -a alm` should list `/usr/local/bin/alm` first; the script and the app's banner name one that comes earlier. **Agents the app opens use the app's own `alm`** (task `A03-16`): a terminal window it opens puts the folder of the `alm` beside the app first in `PATH` (`target/debug` under `tauri dev`, so build `cargo build -p allomos-cli` first; with none there, `PATH` stays as it is), and a request sent through a link names that `alm` by full path when the shell would find another. **On Windows the installer installs `alm`** (task `A07-06`): `alm.exe` goes beside the app, and the app's folder is added to the person's own PATH (removed again on uninstall), so `alm` works in terminals opened after installing.

## Running the app

```bash
npm --prefix apps/web install
npm --prefix apps/desktop install
npm --prefix apps/desktop run dev
```

`npm --prefix apps/desktop run dev` starts the interface and the desktop window together and reloads on changes. On a Mac it first builds this checkout's `alm` (`target/debug/alm`) and points `/usr/local/bin/alm` at it, so Terminal's `alm` matches the app being run; the latest run wins, in the main checkout or a worktree, and the Mac password is asked only when the link changes (`apps/desktop/scripts/dev.sh`, task `A07-19`). `npm --prefix apps/desktop run tauri dev` starts the app without touching the link. To open a folder directly, pass it at start-up: `ALLOMOS_OPEN=/path/to/folder npm --prefix apps/desktop run dev`, or `open Allomos.app --args /path/to/folder` for a built app. The folder is the first argument that is not an option; macOS options such as `-AppleLanguages '(ja)'` may come first. A given path that is not a folder is reported in the window instead of reopening the last folder (task `A01-04`).

While a taskbag is open, the window checks about once a second whether its files changed (a fingerprint of names, sizes and change times, `taskbag_stamp`) and reloads when they did, so changes made with `alm` show without reopening the folder. Polling was chosen over a folder watcher because it needs no new package or thread, and inside the desktop app it is a cheap local call. **Companions are different:** over the network, the desktop app pushes a change notice to them; they never ask on a timer (owner, 2026-10-06).

To build a local, unsigned app: `npm --prefix apps/desktop run tauri build -- --debug --bundles app`. The result is `target/debug/bundle/macos/Allomos.app`. Run it through `npm run` as shown, the same way GitHub's release build does: `npm run` starts Tauri inside `apps/desktop`, whereas `npx --prefix apps/desktop tauri …` starts it in the repository's root, where Tauri can pick the user guide's `docs/guide/package.json` and fail looking for `docs/web/package.json` (seen 2026-10-13).

## Building for Windows

**Releases are built by GitHub** (task `A07-11`, 2026-10-13): pushing a version tag such as `v0.2.0` runs `.github/workflows/release.yml`, which builds the macOS (Apple Silicon) `.dmg` and `.pkg` (made from the built app by `apps/desktop/scripts/build-mac-pkg.sh`, which also works on this Mac), the Windows installer and `alm` for both on GitHub's computers and attaches them, unsigned, to that version's release. The tag must match `version` in `tauri.conf.json` and `Cargo.toml`, or the run stops. GitHub charges no build time since the repository became public; it was private when task `A07-02` chose to build on the Mac (owner, 2026-10-11).

The Mac script below stays as a backup (owner, 2026-10-13). The Mac can only *build* the Windows version, not run it: nothing is tested on Windows this way.

```bash
bash apps/desktop/scripts/build-windows.sh
```

It leaves `Allomos_<version>_x64-setup.exe` (the unsigned installer; it needs no administrator rights, and it installs `alm` too) and `alm.exe` in `target/windows/`. The installer's Windows-only settings are in `apps/desktop/src-tauri/tauri.windows.conf.json` and `windows/hooks.nsh`; the script copies `alm.exe` to `apps/desktop/src-tauri/binaries/` first, which a Windows `cargo` check of the app also needs. Windows warns that the publisher is unknown until builds are signed at v1.0. The first build downloads Microsoft's Windows SDK libraries into a local cache and takes a few minutes.

One-time tools (the script names any that are missing):

```bash
brew install rustup makensis llvm
/opt/homebrew/opt/rustup/bin/rustup toolchain install 1.85.1 --profile minimal -c rustfmt,clippy -t x86_64-pc-windows-msvc
/opt/homebrew/opt/rustup/bin/cargo +1.85.1 install cargo-xwin --version 0.19.2 --locked
```

Homebrew's `rustup` is keg-only: it does not replace Homebrew's `rust`, and only the script puts it on PATH. `cargo-xwin` 0.19.2 is the last release that works with Rust 1.85. The script sets `LC_ALL=en_US.UTF-8` because `makensis` 3.13 crashes (`std::bad_alloc`) when no locale is set, as in agent shells.
