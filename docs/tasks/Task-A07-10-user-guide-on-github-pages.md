# Task-A07-10-user-guide-on-github-pages

*Card type: docs. Area A07 (Platform & release). Written 2026-10-12 at the owner's request.*

## What gets built (plain words)

Allomos has no user guide yet. Everything in `docs/` today is for the people and agents building Allomos (cards, decisions, workflow). This card starts a guide for the people who *use* it, and puts it on GitHub's free website hosting, **GitHub Pages**.

| Piece | What it is | Where it lives |
|---|---|---|
| User guide | Markdown pages written for users, kept apart from the developer docs | `docs/guide/` |
| First pages | A home page, "What Allomos is" and "Getting started" (get the app, the setup wizard, connecting an agent, the first task), each in English, Chinese and Japanese | `docs/guide/src/content/docs/` (English), `…/zh-cn/`, `…/ja/` |
| Site builder | Astro Starlight, a free tool that turns the Markdown into a website with a menu, search and a language switch | `docs/guide/astro.config.mjs`, `docs/guide/package.json` |
| Publish workflow | A GitHub Actions workflow (a script GitHub runs on every push to `main`) that builds the site and publishes it | `.github/workflows/guide.yml` |

At the end the owner can open the guide in a browser locally, and, once they approve the hosted step, at `https://mypals-ml.github.io/allomos/`.

## Details for the worker

- **Outcome.** A user can read how to start with Allomos on a public web page, and an agent updates that page in the same commit as the feature it describes.
- **Decisive unknowns.**
  - Site builder: **Astro Starlight** (owner, 2026-10-12): built-in language switch; runs on Node, which `apps/web` and `apps/desktop` already use. Pinned: `@astrojs/starlight` 0.42.4, `astro` 7.3.5 (both released 2026-09-24).
  - Languages: **English, Simplified Chinese and Japanese**, like the README (owner, 2026-10-12). Every page exists in all three.
  - Offline help inside the app: **not wanted** (owner, 2026-10-12). The guide lives only on the website; the app may link to it.
- **Files owned.** `docs/guide/**`, `.github/workflows/guide.yml`; one line in `docs/README.md` and `.astro/` in `.gitignore`.
- **External operations** (each needs the owner's exact approval in the table below before it runs):
  - Turn on GitHub Pages for `mypals-ml/allomos` with "GitHub Actions" as the source. Cleanup: turn Pages off in the repository settings.
  - Merging this card's branch into `main`: the first push of `.github/workflows/guide.yml` to `main` runs it and publishes the site. Pushing the card's own branch does not run it (it runs only for `main`). Cleanup: delete the workflow and turn Pages off.
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
- 2026-10-12 — Owner chose Astro Starlight and confirmed where the files and the site live.
- 2026-10-12 — Built locally, in branch `claude/a07-10-user-guide`. Plain words: the guide's three pages exist in all three languages, the site builds on this Mac without warnings, and every link works; nothing is online yet.
  Evidence: `npm --prefix docs/guide run build` → 10 pages, no warnings or errors (a `404.md` with `draft: true` and an empty `i18n` collection silence Starlight's two default warnings). Link check over `docs/guide/dist`: 75 internal links, 0 broken; the two GitHub links return 200. Preview at `http://localhost:4321/allomos/` checked in the browser (home in English, Getting started in Chinese, sidebar and language switch). Node 22.14 here gives one install notice (`undici` asks for 22.19); the build is unaffected, and the workflow uses the newest Node 22.
