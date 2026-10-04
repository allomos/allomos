# Task-W01-01-core-model

- **Outcome.** Given a project's workstreams, tasks and their event history, a developer (and later the web app and CLI) can compute per-workstream progress, a burndown series (remaining and total scope over time), and a forecast finish-date range at 50% and 85% confidence — as a pure, tested **Rust** library (`crates/core`), the one authority for project rules ([0011](../decisions/0011-desktop-framework-and-shared-code.md): Tauri 2, rules in Rust).
- **Decisive unknowns.**
  - Toolchain: a Cargo workspace with `crates/core`; pin the Rust toolchain (`rust-toolchain.toml`) and choose test helpers (worker proposes; owner reviews with the card). The TypeScript interface and Tauri app come in later cards.
  - What to learn from Glidelines' `burndown.ts` and forecast code (TypeScript) when writing the Rust version. The worker reads the files named in [REFERENCES.md](../REFERENCES.md); 0003 (reuse of Glidelines) remains open, but porting ideas to Rust is safe under every option.
  - Keep the model free of storage and network code. Since 2026-10-04, projects are Git-tracked JSON/Markdown files ([0007](../decisions/0007-git-like-project-sync.md)), so the model's records and events should map one-to-one to those files. Reading and writing files belongs to `crates/store`, not here.
  - Default throughput window (4–6 weeks) and simulation count (worker proposes; owner confirms with the result).
- **Files owned.** `Cargo.toml` (workspace), `rust-toolchain.toml`, `crates/core/**`, `docs/workflow/CODING_RULES.md` (created here: the first coding rules), this card.
- **External operations.** None (installing the Rust toolchain and crates locally is local work).
- **Done means mechanically.** `cargo test -p core` passes with cases for: counts and size-weighted burndown; scope growth; tasks without completion dates (reconstructed band); throughput Monte Carlo producing stable 50%/85% dates with a fixed seed; empty and single-task projects. `cargo clippy` and `cargo fmt --check` pass. A fixture built from the Glidelines SSR snapshot (read-only copy of its structure, not its private text) renders a burndown series in a test.

## Approvals

| Date | What was approved | By |
|---|---|---|

## Notes

- 2026-10-04 — Rewritten for Rust after the owner chose Tauri 2 (0011).
- 2026-10-04 — Still the first code card under [0008](../decisions/0008-build-our-own.md) (build our own).
- Model fields: [PLAN § 5](../product/PLAN.md#5-data-model-first-cut); forecast method: [PLAN § 7](../product/PLAN.md#7-burndown-and-forecast), [decision 0002](../decisions/0002-burndown-counts-plus-sizes.md).
