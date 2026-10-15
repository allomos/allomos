# Task-A07-13-pkg-link

*Card type: coding. Area A07 (Platform & release). Written 2026-10-16 from the owner's report of 2026-10-14: after installing `Allomos_0.1.2_aarch64.pkg`, `alm` and the opened app were an old build in the main checkout, not the one just installed.*

## What gets built (plain words)

Installing Allomos from the Mac installer package (`.pkg`) must leave the Mac using the app it just put in Applications, even when another copy of Allomos.app sits somewhere else (for example an old build in a project folder):

| Piece | What it is in everyday words | Where |
|---|---|---|
| The link step after installing | Points `/usr/local/bin/alm` (the `alm` every Terminal finds) at the `alm` inside `/Applications/Allomos.app`. It used to build that path from a folder name the installer hands over (`$2`); now it is a fixed path, so no other copy can be picked | `apps/desktop/pkg/postinstall` |
| A check of that step | Runs the link step on pretend disks (folders under a scratch folder), with another Allomos.app present and every kind of old link, and says PASS or FAIL per case; it never touches the real Mac | `apps/desktop/pkg/test-postinstall.sh` (new) |
| A check of the package | After building the `.pkg`, the build script opens it and refuses it unless it installs to `/Applications`, may not be moved by the installer onto another Allomos.app it finds ("relocated"), and carries this postinstall | `apps/desktop/scripts/build-mac-pkg.sh` |

## Details for the worker

- **Outcome.** After installing the `.pkg` on a Mac with another Allomos.app elsewhere, `/usr/local/bin/alm` leads to `/Applications/Allomos.app/Contents/MacOS/alm`, and opening Allomos opens that app.
- **Decisive unknowns.** Why the 2026-10-14 install linked the old build: Installer moving the install onto the other copy ("relocation") despite `BundleIsRelocatable NO`, or `$2` naming another folder. Answered below from the Mac's installer log, as far as the log can show.
- **Files owned.** `apps/desktop/pkg/postinstall`, new `apps/desktop/pkg/test-postinstall.sh`, `apps/desktop/scripts/build-mac-pkg.sh`, `docs/workflow/CODING_RULES.md` (Install on a Mac, one sentence), this card.
- **External operations.** None by the worker: no install, no `sudo`, no change to `/usr/local/bin`, `/Applications` or macOS's app registry (LaunchServices). Installing the built `.pkg` is the approver's human check.
- **Done means mechanically.**
  - `test-postinstall.sh` passes every case; the same cases fail with the old postinstall whenever `$2` is not exactly `/Applications`.
  - `build-mac-pkg.sh` builds the `.pkg` from this branch's app and its own checks pass (`install-location="/Applications"`, no `<relocate>`, the same postinstall).
  - The repository's checks (CODING_RULES "Checks before a commit") pass.
  - Human check on this Mac: steps below.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-16, cause (plain words). The installer did **not** move the app onto the old copy: macOS's installer log (`/var/log/install.log`) shows every Allomos install on 2026-10-14 (00:56, 01:18, 12:50, 18:12, 18:32, 19:46) and the later ones (0.1.4 on 2026-10-14 21:10, 0.1.5 on 2026-10-15) unpacked into `/Applications` and registered `/Applications/Allomos.app` (12:50, the build from `fdec1b4`: "Shoving …/Root (1 items) to /", "Touched bundle /Applications/Allomos.app", "Registered bundle file:///Applications/Allomos.app/ for uid 501"). So `BundleIsRelocatable NO` worked, and the package's settings were right (`relocatable="false"`, A07-12). The weak point was the link step: it built the app's path from `$2`, a folder name that Installer passes to the script. Apple's documented meaning is "the install destination" (`/Applications` here), and the 19:46 install (A07-15's check, link moved to `/Applications`) agrees; but anything else in `$2` (another copy's folder, or `/`) sent the link elsewhere or left the old link in place, with no error. The script check below shows this: with the old script, three of eight cases end on the other copy. What exactly `$2` held at 12:50 cannot be read back: the script prints nothing, and macOS keeps its detailed logs for only a few days (none left for 2026-10-14).
- 2026-10-16, a second way the link reached the old build, which this card cannot remove: a copy of Allomos that is open (or that macOS chooses when you open "Allomos") checks whether Terminal's `alm` is its own, and if not offers **Install the alm command…**, which links `alm` to *itself*. If the old build in the main checkout was still running at 12:50, or macOS picked it when Allomos was opened (both copies were version 0.1.2 with the same identity `io.allomos`, so macOS may open either; an app already running is simply brought to the front), its banner would have offered exactly that link. This matches "macOS opened that old app too".
- Decided: the link names the fixed path `/Applications/Allomos.app/Contents/MacOS/alm` and ignores `$2` (the package always installs there and may not be relocated, so the path is known; nothing passed in can steer it). Only `$3`, the disk being installed on (`/` normally), is used to look for the app and the link folder on that disk.
- Decided: the installer does not unregister or remove other Allomos copies from macOS's app registry, and does not quit a running copy (both change things outside this app, on someone's Mac, and need the approver). The human check says to quit Allomos before installing.
- Decided: `build-mac-pkg.sh` checks the built package (`pkgutil --expand`: `install-location="/Applications"`, no `<relocate>` entry, postinstall identical to `pkg/postinstall`) and stops with an error otherwise, so a slip in the package settings fails the release build instead of reaching a Mac.
- Question for the approver (not blocking this card): should a copy of Allomos that is *not* in `/Applications` stop offering to take the `alm` link while `/Applications/Allomos.app` exists, and should the installer remove stale copies from macOS's app registry? Either would stop an old build from winning again; both change A07-15's "latest install wins" behaviour or the Mac's registry.
