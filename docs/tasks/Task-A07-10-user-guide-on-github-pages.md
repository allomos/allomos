# Task-A07-10-user-guide-on-github-pages

*Card type: docs. Area A07 (Platform & release). Written 2026-10-12 at the owner's request. **Not started.***

## What gets built (plain words)

Allomos has no user guide yet. Everything in `docs/` today is for the people and agents building Allomos (cards, decisions, workflow). This card starts a guide for the people who *use* it, and puts it on GitHub's free website hosting, **GitHub Pages**.

| Piece | What it is | Where it lives |
|---|---|---|
| User guide | Markdown pages written for users, kept apart from the developer docs | `docs/guide/` |
| First pages | "What Allomos is" and "Getting started" (install, open a folder, the setup wizard, the first task) | `docs/guide/index.md`, `docs/guide/getting-started.md` |
| Site builder | A small static-site tool that turns the Markdown into a website | its config file at the repository root (tool chosen in the card's work) |
| Publish workflow | A GitHub Actions workflow (a script GitHub runs on every push to `main`) that builds the site and publishes it | `.github/workflows/guide.yml` |

At the end the owner can open the guide in a browser locally, and, once they approve the hosted step, at `https://mypals-ml.github.io/allomos/`.

## Details for the worker

- **Outcome.** A user can read how to start with Allomos on a public web page, and an agent updates that page in the same commit as the feature it describes.
- **Decisive unknowns.**
  - Which site builder: recommended **Astro Starlight** (built-in language switcher; runs on Node, which `apps/web` and `apps/desktop` already use). Waits on the owner's choice. Pin its version in this card.
  - Languages: **English, Simplified Chinese and Japanese**, like the README (owner, 2026-10-12). Every page exists in all three.
  - Offline help inside the app: **not wanted** (owner, 2026-10-12). The guide lives only on the website; the app may link to it.
- **Files owned.** `docs/guide/**`, the site builder's config file, `.github/workflows/guide.yml`; one line in `docs/README.md` pointing to the guide.
- **External operations** (each needs the owner's exact approval in the table below before it runs):
  - Turn on GitHub Pages for `mypals-ml/allomos` with "GitHub Actions" as the source. Cleanup: turn Pages off in the repository settings.
  - The first push of `.github/workflows/guide.yml` to `main`, which publishes the site. Cleanup: delete the workflow and turn Pages off.
- **Done means mechanically.**
  - The site builds locally with no warnings, and the two pages open in a browser with working links.
  - After approval: the workflow run succeeds on GitHub, and `https://mypals-ml.github.io/allomos/` shows both pages.
  - `git diff --check` is clean.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | The card (local work only; Pages and the publish workflow still need their own approval) | owner |

## Notes

- 2026-10-12 — Card written at the owner's request after a talk about which GitHub features suit user documentation: Markdown in the repository plus GitHub Pages (chosen), the wiki (not used: no review, edited apart from the code), Releases for release notes, and Discussions and issue templates later.
- 2026-10-12 — Owner approved the card and chose three languages: English, Chinese and Japanese.
- 2026-10-12 — Owner decided against help pages built into the app: the guide is online only.
