# Task-A07-12-mac-package-installs-app-and-alm

*Card type: coding. Area A07 (Platform & release). Written 2026-10-13 at the owner's request ("we might need a installable package for mac to install the app and the alm").*

## What gets built (plain words)

On a Mac today, Allomos and `alm` are installed separately: the app from a `.dmg` (drag to Applications), and `alm` by building it from the source (`cargo install`) or downloading it alone. Someone who installs only the app has no `alm`, so AI agents asked by Allomos cannot work. On Windows the installer already brings `alm` along ([`A07-06`](./Task-A07-06-windows-installer-installs-alm.md)).

After this card, both ways of installing on a Mac bring `alm` (owner's choice, 2026-10-13: both):

| Piece | What it is in everyday words | Where |
|---|---|---|
| `alm` inside the app | The app carries its own `alm`, next to the app's program inside `Allomos.app` | `apps/desktop/src-tauri/tauri.macos.conf.json` (or the build script), `.gitignore` |
| Mac installer package | `Allomos_<version>_aarch64.pkg`: double-click, enter the Mac password once; it puts Allomos in Applications and makes `alm` work in every new terminal | a build script under `apps/desktop/scripts/`, `.github/workflows/release.yml` |
| The app offers `alm` | When `alm` is missing from the terminal, or is not the app's own one, the app offers **Install the alm command** (asks for the Mac password); so a `.dmg` install, or an old `alm` from `cargo install`, is fixed with one click | `apps/desktop/src-tauri/src/`, `apps/web/src/` (three languages) |

**How `alm` is made available:** a link `/usr/local/bin/alm` → `/Applications/Allomos.app/Contents/MacOS/alm`. `/usr/local/bin` is on every Mac terminal's PATH. Because it is a link, updating the app updates `alm` too; they can never drift apart. *Decided (owner, 2026-10-13): this way.*

**What you can see at the end:** on a Mac without `alm`, open the `.pkg`, install, open a new Terminal, and `alm --version` answers with the app's version. Or install from the `.dmg`, start Allomos, click **Install the alm command**, and the same works.

## Details for the worker

- **Outcome.** A Mac user who installs Allomos, by `.pkg` or by `.dmg`, gets a working `alm` that matches the app, with at most one click.
- **Decisive unknowns.**
  - The `.pkg` is unsigned, like the `.dmg` (right-click › Open the first time); signing is a separate card.
  - Whether Tauri can place an extra program inside `Contents/MacOS` (`bundle.macOS.files` or `externalBin`), or the build script copies it in: found by building.
  - The app's offer needs administrator rights for `/usr/local/bin` (macOS password prompt via `osascript … with administrator privileges`). If `/usr/local/bin` is missing (some new Apple Silicon Macs), the installer and the offer create it.
  - **The app never replaces `alm` by itself** (owner asked, 2026-10-13). It does not copy `alm` out; it only makes the link, and only when the person clicks. At start it finds one of three cases: the link to the app's own `alm` is there → nothing to do (updating the app updates `alm`); no `alm` → offer; another `alm` found first (not the link, or a different version) → offer, naming the one found.
  - A real file (not a link) already at `/usr/local/bin/alm` is never overwritten without the person's yes, naming that file.
  - An `alm` earlier on the PATH (such as `~/.cargo/bin/alm`) still wins after the link is made, so the offer explains that and has **Don't ask again** (remembered in Allomos's per-user settings), so it does not come back at every start.
  - This Mac's own `~/.cargo/bin/alm` (development) comes before `/usr/local/bin` on the PATH; development keeps using `cargo install` (CODING_RULES says so).
- **Files owned.** The files in the table; `docs/workflow/CODING_RULES.md` (how `alm` is installed on a Mac); `docs/guide/` "Get the app" (three languages); this card.
- **External operations** (each needs the owner's exact approval, recorded below): merging the workflow change into `main`; a release run that attaches the `.pkg` to a release page. Local builds and handing the files to the owner need none.
- **Done means mechanically.**
  - Tests: the app's check that tells whether `alm` is missing, foreign or the app's own (unit tests over the three cases, plus a real file at `/usr/local/bin/alm` and "Don't ask again").
  - A local build produces the `.pkg` and a `.dmg` whose `Allomos.app` contains `alm`.
  - Checked by hand by the owner: on this Mac (with `~/.cargo/bin/alm` moved aside), the `.pkg` gives a working `alm` in a new Terminal; with `/usr/local/bin/alm` removed, the app offers to install it and the click works.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-13 | Start the card, in I02; merging the workflow change into `main` and a release run attaching the `.pkg` (publishing) | owner (chat) |
| 2026-10-13 | `alm` made available as a link `/usr/local/bin/alm` → the app's own `alm` | owner (chat) |
| 2026-10-13 | The idea: a Mac package installing app and `alm`; both a `.pkg` and the app offering to install `alm` (card not yet approved to start) | owner (chat) |
