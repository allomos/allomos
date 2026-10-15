# Task-A01-34-migrate-refused

*Card type: coding. Area A01 (Tracking). Written 2026-10-15. Builds on [`A01-32`](./Task-A01-32-guided-migration.md) (the guided migration prompt). Task text: `alm show A01-34`.*

## What gets built (plain words)

When an older taskbag is being updated, `alm migrate` prints a prompt for the agent. One sentence lists the commands the update may use (for format 3 → 4 these include `alm done`); the next sentence named `done` again among the commands that are refused until the update is finished. `alm` does allow `done` then, so the second sentence was wrong and could make an agent skip a step.

Now the refused examples are worked out from the same list as the allowed commands: any example the update allows is left out. For format 3 → 4 the prompt says "(claim, label, add …)" in English, and the same in Chinese and Japanese.

| Piece | What it is | Where |
|---|---|---|
| Refused examples | `REFUSED_EXAMPLES` (claim, label, done, add) minus the allowed commands, `refused_examples()` | `crates/store/src/versions.rs` |
| Sentence | `read_only` takes `{refused}` instead of a fixed list, in en, zh-CN, ja | `crates/store/src/versions.rs` |
| Tests | No allowed command is among the refused examples for any real starting format; the 3 → 4 prompt shows "(claim, label, add …)" in all three languages | `crates/store/src/versions.rs` |

## Details for the worker

- **Outcome.** The migration prompt never names a command as both allowed and refused.
- **Files owned.** `crates/store/src/versions.rs`; this card.
- **External operations.** None.
- **Done means mechanically.** The named checks in [CODING_RULES](../workflow/CODING_RULES.md#checks-before-a-commit) pass.

## Decisions

- Decided: an example counts as allowed when an allowed command equals it or starts with it and a space (`add` is dropped when `add task` is allowed), so a two-word command cannot slip through.
- Decided: the refused examples stay the four the sentence already named (claim, label, done, add); no new example was added, so the wording otherwise stays as it was.
- Decided: in zh-CN and ja the examples are joined with "、" as before; in English with ", ".

## Notes

- 2026-10-15 (claude/a01-34-migrate-refused): built and checked; evidence below.
- Evidence, 2026-10-15: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (198 passed, 0 failed; new test `versions::tests::refused_examples_never_name_an_allowed_command`, and the 3 → 4 prompt test now checks "(claim, label, add …)" in en, zh-CN and ja), `npm --prefix apps/web run typecheck`, `build`, `check:i18n`, `test`: all PASS. origin/main had not moved from 0fe94da.
