# Task-A00-25-quoted-name-in-chinese-docs

*Card type: docs. Area A00 (Planning). Written 2026-10-12 at the owner's request. **Not started: the owner asked for the card only.***

## What gets built (plain words)

Since 2026-10-12 the app's Chinese texts write the product's name in a sentence as 「换肩」, in corner brackets, and without them where the name stands alone (top bar, labels, headings) — rule in [CODING_RULES § Interface languages](../workflow/CODING_RULES.md#interface-languages), from [`A05-03`](./Task-A05-03-taskbag-setup-wizard.md). The project's own documents still write 换肩 without brackets in about 31 places (counted 2026-10-12 with `git grep -n 换肩 -- docs`).

After this card, the documents follow the same rule.

## Details for the worker

- **Outcome.** Every 换肩 in a Chinese sentence in `docs/` is 「换肩」; names standing alone, quoted English names ("换肩, アロモス") and dated history stay as they are.
- **Decisive unknowns.** Whether the published road show deck (`docs/pitch/roadshow-zh.html`, republished as an artifact) is in scope (owner).
- **Files owned.** `docs/**` files that contain 换肩.
- **External operations.** Republishing the deck, if in scope, needs the owner's approval.
- **Done means mechanically.** `git grep -n 换肩 -- docs` lists only bracketed uses, standalone names and the cases the owner excluded; links and `git diff --check` clean.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-12 — Card written at the owner's request; not to be started yet.
