# Task-A07-17-restart-to-update

*Card type: coding. Area A07 (Platform & release). Written 2026-10-15 at the approver's request, after seeing the Claude app's "Restart to Update to Claude <version>" menu item.*

## What gets built (plain words)

Today ([`A07-16`](./Task-A07-16-update-reminder.md)) Allomos tells you a newer version exists and opens its download page; you then download and install it yourself. This card makes Allomos do that itself, as the Claude app does: it downloads the new version in the background, checks that the download really comes from us, and the app menu shows **Restart to Update to Allomos <version>**. Choosing it installs the new version and starts Allomos again.

## Details for the worker

- **Outcome.** Updating Allomos takes one menu choice.
- **Likely approach.** Tauri's updater plugin (`tauri-plugin-updater`, with `tauri-plugin-process` for the restart). Each update file is signed with a private update key; the app holds the matching public key and refuses a file whose signature does not match.
- **Decisive unknowns** (approver):
  - Whether the `alm` command inside the app is updated with it on Windows and macOS, and how the `/usr/local/bin/alm` link ([`A07-15`](./Task-A07-15-one-alm-link-last-install-wins.md)) keeps leading to the right one.
  - Whether to keep A07-16's notice and Download button, or let the menu item replace them.
- **External operations** (each needs the approver's exact approval on this card): creating the update-signing key and storing it as a GitHub secret (its value never printed or committed; where it lives goes in the taskbag's `project-notes.md`); changing the release workflow to publish signed update files and a `latest.json`; the first release that carries them.
- **Dependencies.** New ones pinned deliberately in this card.
- **Done means.**
  - An installed older Allomos on macOS and on Windows finds a newer test release, downloads it, shows **Restart to Update to Allomos <version>**, and after the restart About and `alm --version` show the new version.
  - A download with a wrong signature is refused, and the app keeps running the old version.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-15 | The card, made ready (local work only; the key, the secret, the workflow change and any release still need their own approval) | approver |
