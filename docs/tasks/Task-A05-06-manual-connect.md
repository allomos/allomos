# Task-A05-06-manual-connect

*Card type: coding. Area A05 (Setup), Iteration 2. Written 2026-10-14. Builds on [`A05-03`](./Task-A05-03-taskbag-setup-wizard.md) (the setup wizard), [`A03-03`](./Task-A03-03-app-asks-agents-to-set-up.md) (the app asks AI agents to set up) and [`A07-05`](./Task-A07-05-windows-agents-no-flash.md) (Windows finds and asks agents); since [`A01-38`](./Task-A01-38-no-git-in-alm.md) the wizard runs no Git and the connected agent handles it.*

## What gets built (plain words)

In the wizard's last step, **Connect with the agents**, Allomos could only open AI agents it knows (Claude and Codex, as app or terminal) with its request typed in. Someone using another AI had no way through. This card adds a **Manual** choice under the known agents, offered even when none is found. Choosing it shows the request in a box with a **Copy** button ("Copied" shows for two seconds); the person opens their own AI in the project folder, pastes and sends. While the step is open, Allomos watches the project folder and says when the AI has written its instructions; **Done** closes the step either way.

| Piece | What it is | Where |
|---|---|---|
| The request | Exactly what automatic sending types in: the connect request, or the update request when the project is out of date, built by the same `connect_plan`, with the same "run `alm` as …" line a link-opened agent gets when the shell would find another `alm` | `apps/desktop/src-tauri/src/lib.rs` (`manual_request`), `agents.rs` (`link_request`) |
| Copy | The app puts the text on the clipboard itself (`pbcopy` on macOS, `clip.exe` on Windows, as the terminal agents already do), so it does not depend on the window's clipboard permission | `agents.rs` (`copy_text`), `lib.rs` |
| The Manual choice | A radio choice after Claude and Codex; ticked first when no known agent is found. Shows the project folder, the request, Copy, and what Allomos has noticed | `apps/web/src/AgentsPanel.tsx`, `styles.css` |
| Its decisions | Which choice comes first, which buttons show, what counts as "the AI wrote its instructions" — plain TypeScript, tested with Node | `apps/web/src/manualFlow.ts`, `apps/web/tests/manualFlow.test.ts` |
| Texts | 11 new texts and 2 changed ones ("no agent found" now points at Manual), in English, Chinese and Japanese | `apps/web/src/i18n/*.json` |

**What you can see at the end:** on a new folder, the wizard's step 4 lists Manual under the found agents; choosing it shows the request; Copy puts it on the clipboard and says Copied; pasting it into any AI and sending makes the step say the AI added Allomos's instructions to AGENTS.md / CLAUDE.md; Done closes the wizard with the project open.

## Details for the worker

- **Outcome.** A person whose AI Allomos does not know can still connect it, with the same request.
- **Choice (the card's open point, owner rule A00-29).** After a manual connect the wizard **watches for what the automatic path already watches for** — an instruction file (AGENTS.md, CLAUDE.md) appearing, or one that starts to mention Allomos — and says so in the step; **Done** is always there as the fallback and closes the step, handing the same watch to the window (the "start a new session" notice). Reason: the files are the only proof Allomos can read without trusting the AI, and some AIs (or an already-connected project) change nothing visible, so the person must be able to finish anyway.
- **Choice.** The step does not close by itself when the files appear: the person may still be reading the AI's answer or its plan for existing tasks.
- **Choice.** "Add the instructions for me" stays when no agent is found, now a plain button beside Done.
- **Files owned.** `apps/desktop/src-tauri/src/{agents,lib}.rs`, `apps/web/src/{AgentsPanel.tsx,manualFlow.ts,messenger.ts,styles.css}`, `apps/web/src/i18n/{en,zh-CN,ja}.json`, `apps/web/tests/manualFlow.test.ts`, this card.
- **External operations.** None. Nothing is sent by Allomos.
- **Done means mechanically.** The named checks pass; a Rust test shows the Manual request equals the automatic one (connect and update, three languages); TS tests cover the first choice, the buttons and the detection; the debug app shows Manual on a new folder.

## Approvals

| Date | What was approved | By |
|---|---|---|
| 2026-10-14 | The task's open point (how the wizard knows the agent finished) is a routine choice for the worker under the owner's rule A00-29 (origin 2026-10-14), recorded above | owner, relayed by the coordinating session |

## Notes

- 2026-10-14 — Built by `claude/a05-06-manual-connect`. Evidence below.

## Result (2026-10-14)

**In plain words.** The wizard's last step now has **Manual** under Claude and Codex, and it is the choice ticked first when Allomos finds no agent it knows. It shows the request Allomos would have typed in for a known agent, a **Copy** button, and a line that changes to "✓ The agent has added Allomos's instructions to …" when AGENTS.md or CLAUDE.md appears or starts to mention Allomos. **Done** closes the step and leaves the window watching for new instruction files, as after an automatic connect.

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` (new: `manual_shows_the_request_an_agent_would_get`, connect and update requests, en / zh-CN / ja) | PASS |
| `npm --prefix apps/web run typecheck`, `build` | PASS |
| `npm --prefix apps/web run check:i18n` (349 texts) | PASS |
| `npm --prefix apps/web test` (68 tests; new `manualFlow.test.ts`: first choice, buttons, detection) | PASS |
| Debug app built (`tauri build --debug --bundles app`) | see the session's report |
| Manual on screen, Copy, detection, Done in the built app on macOS | owner check |
| Windows | untested (the code paths are shared; Copy uses `clip.exe` as the terminal agents already do) |
