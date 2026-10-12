# Task-A01-24-taskbag-on-github

*Card type: operations. Area A01 (Tracking). Written 2026-10-12 at the owner's request. Since the owner kept the move of 2026-10-12 ([`docs/allomos-conversion.md`](../allomos-conversion.md)), Allomos's own taskbag is the separate repository `../allomos-taskbag`, which had no remote: all task status lived only on one Mac.*

## What gets built (plain words)

The taskbag repository gets a private home on GitHub, `mypals-ml/allomos-taskbag`, as its `origin`, and its history is pushed there. Then `alm … --push` works for it as it does for any project, and status is no longer lost if this Mac is.

## Details for the worker

- **Outcome.** `git -C ../allomos-taskbag remote -v` shows `origin https://github.com/mypals-ml/allomos-taskbag.git`; `master` is pushed; `alm … --push` changes reach GitHub.
- **Decisive unknowns.** None.
- **Files owned.** None in this repository besides this card and the handoff's Live state.
- **External operations.** Create the private repository `mypals-ml/allomos-taskbag` on GitHub and push `../allomos-taskbag` to it (approved below). Cleanup if undone: delete that repository (owner's decision).
- **Done means mechanically.** The remote is set, `git status -sb` in the taskbag shows no "ahead", and `gh repo view mypals-ml/allomos-taskbag` reports a private repository.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-12 | "create a private GitHub repository `mypals-ml/allomos-taskbag` and push it" | owner (chat) |

## Notes

- 2026-10-12 — Card written; approval recorded.
