# Human and AI-agent interface investigation

**Evidence collected: 2026-10-03.** Scope: Glidebox's desktop, mobile, browser and human CLI experience, plus the structured interfaces used by AI agents and the human screens for supervising them. Audience: team leaders, contributors and external agents. The horizon is the initial product architecture and later platform expansion, not a commitment to release every client at once.

The owner requested well-structured interfaces for both humans and agents, expanded human UI scope to desktop/mobile and consideration of CLI/browser, and requested that the findings be documented. Accepted needs and proposed design rules are separated in [the interface contract](../product/INTERFACES.md) and [R60–R68](../product/REQUIREMENTS.md#human-and-agent-interfaces).

## Method and limits

Read official platform guidelines, protocol specifications, product documentation and a public MCP maintainer discussion. No competitors were installed, no workflows were operated and no accessibility, performance or agent-success measurements were made. This honors the owner's instruction that hands-on competitor testing is unnecessary for this investigation. Vendor documentation establishes advertised behavior; a maintainer discussion establishes reported friction, not its prevalence. Recommendations below are Glidebox-specific inferences.

Sources are a dated snapshot. In particular, protocol extensions, browser capabilities and mobile packaging need compatibility checks when an implementation card selects versions. This investigation does not choose a framework, upstream project, storage engine or hosting service.

## Finding

Recommend one set of project/task actions and rules, reached through several interfaces. Adapt presentation to available space and input methods. Give agents small, explicit tools and useful task context. Give people clear evidence, decisions and control over ongoing work. A chat interface can be an entry point, but task state, approvals and history need durable records that other interfaces can read.

## Human interface patterns

| Pattern | Documented evidence | Recommendation for Glidebox — proposed |
|---|---|---|
| Adapt navigation and panes to window size | Apple recommends sidebars for top-level navigation and compact alternatives when space is limited; Material describes list/detail and supporting-pane layouts across window sizes. [Apple sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars), [Material canonical layouts](https://m3.material.io/foundations/layout/canonical-examples/overview). | Desktop/wide browser: project navigation, task list and selected task detail. Phone/narrow window: one main pane with a clear back path, preserving selection and filters. Tablet: expand panes when space permits. |
| Mobile supports quick action and review | Linear documents mobile Home, Inbox, issue creation and Search, with mobile complementing its other clients. [Linear clients](https://linear.app/docs/get-the-app). | Prioritize Waiting on me, My tasks, task detail, quick capture and search. Keep evidence and the exact requested operation readable before an approval. Make full planning accessible without squeezing a desktop board onto a phone. |
| Keyboard speed with discoverable alternatives | Linear documents keyboard, command-menu and contextual-menu issue actions, and similar functionality in board/list views. [Select issues](https://linear.app/docs/select-issues), [Board layout](https://linear.app/docs/board-layout). | Desktop/browser command palette, shortcut help, search and bulk actions; retain visible buttons/menus and touch alternatives. Avoid shortcut conflicts with browsers and editable text. |
| CLI serves both people and automation | CLI Guidelines recommend useful help, structured output and noninteractive operation for scripts. GitHub CLI exposes filters, limits, selected JSON fields and browser handoff. [CLI Guidelines](https://clig.dev/), [GitHub issue list](https://cli.github.com/manual/gh_issue_list). | A readable default view for humans, explicit project selection, examples and completion; stable JSON output and no-input mode for agents/scripts. An optional open command should reach the same selected task in a graphical client. |
| Accessible controls across input methods | W3C explains reflow, minimum target size with exceptions, and an alternative pointer action for dragging. [Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html), [Target size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html), [Dragging](https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html). | Offer a Move/Change state menu alongside board dragging, visible focus and labelled controls. Task text reflows; charts have text/data alternatives. Use comfortable touch targets; the WCAG 24 CSS-pixel criterion is a minimum with exceptions, not the preferred size of every mobile button. |
| Preserve context across client handoffs | Linear documents browser-to-desktop links and client-specific capabilities. [Linear clients](https://linear.app/docs/get-the-app). | Stable project/task references, consistent terms and shared saved views. Show when a local project or session cannot be opened on the current device; links alone cannot transfer a project or launch a remote machine's process. |

## Agent interface patterns

Here, an **agent interface** means the actions, input/output formats and contextual information an agent consumes. It also includes how a human sees and controls the agent's work.

| Pattern | Documented evidence | Recommendation for Glidebox — proposed |
|---|---|---|
| Purposeful tools and precise descriptions | Anthropic recommends purposeful workflow tools, clear naming, relevant results and evaluation on realistic tasks. [Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents). | Start with task discovery, task context, claim/release, progress reporting and decision requests. Explain preconditions, identifiers and resulting changes. Separate a read from a mutation even when grouped by workflow. |
| Bounded context with deeper detail on demand | Anthropic discusses filtering, pagination, concise/detailed responses and actionable truncation. [Same guide](https://www.anthropic.com/engineering/writing-tools-for-agents). | Return a compact task brief: outcome, completion criteria, current revision, blocker, claim and relevant approvals. Keep IDs usable in the brief; fetch long narrative/history separately. Never silently omit a blocker or approval condition to save tokens. |
| Structured inputs, outputs and repairable errors | MCP tools define schemas, structured results and error handling; annotations are not trusted authority. [MCP tools, 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools). | Share versioned schemas between CLI/MCP/import validation. Return a domain error code, affected field, current state and a next action, with a human-readable explanation. Permissions remain enforced in the shared task service. |
| Small tool groups; CLI remains useful | MCP maintainers report tool overload, client-dependent discovery and cases where agents use CLI instead. The notes also ask for quantitative evidence. [August 2026 maintainer discussion](https://github.com/modelcontextprotocol/modelcontextprotocol/discussions/3264). | Expose a small useful baseline and clearly grouped capabilities. Do not assume every host supports deferred discovery, or that MCP always outperforms CLI. No universal “optimal number of tools” was established. |
| Keep human accountability visible | Linear documents delegated agent execution while the human remains the primary issue owner, with agent access managed by admins. [Agents in Linear](https://linear.app/docs/agents-in-linear). | Distinguish responsible person, current worker and agent session. Progress reported by an agent and verification accepted by a person remain separate events. Surface Waiting on me for exact decisions. |
| Rich UI inside an agent host is optional | MCP Apps provide interactive views in compatible hosts. Google's A2UI discussion distinguishes declarative components from custom embedded MCP Apps. [MCP Apps](https://modelcontextprotocol.io/extensions/apps/overview), [Google A2UI + MCP Apps](https://developers.googleblog.com/en/a2ui-and-mcp-apps/). | Later, consider a task/change-preview or decision form inside supported agent clients. Keep ordinary CLI/MCP results sufficient when a host cannot render it. These are human-facing extensions, not a replacement for structured task operations. |

Retry safety, revision checks and resumable operations below are engineering recommendations drawn from Glidebox's concurrency and agent-session needs; this research did not prove an implementation of them.

## Local and cloud implications by surface

Local-only storage and offline caching of a cloud project are different promises. Linear's documentation explicitly describes its offline mode as a failsafe, including possible overwrite behavior; it does not establish Glidebox's account-free local requirement. [Linear clients](https://linear.app/docs/get-the-app).

| Surface | Local-only approach to evaluate | Cloud and cross-device approach to evaluate | Main constraint |
|---|---|---|---|
| Installed desktop | Local application runtime accesses the selected project store; GUI, CLI and MCP reach the same project authority. | Signed-in cloud connection; explicit transfer/sync if adopted. | File permissions, upgrades and background runner behavior need design. |
| Browser on a computer | Either a UI served by a local companion, or browser-owned storage with import/export and capability-tested file access. | Hosted web client signs in for cloud projects. | Browser-owned storage is not automatically the computer's project folder or the CLI's store. |
| Mobile app or installed web app | A project stored on that device, using its permitted app/browser storage; no cloud sign-in for local tracking. | Signed-in cloud access for shared projects; explicit transfer for local copies. | A phone cannot inherently read a desktop-local project. Background execution and local runner controls need separate feasibility work. |
| Human CLI and agent CLI/MCP | Headless local access, using the same validation and history rules as the GUI. | Scoped authenticated cloud access; exact permissions checked by the service. | Must work without a GUI and avoid prompts in noninteractive calls. |

Browser storage has important differences: the **origin private file system (OPFS)** is private to a site origin, is subject to quota, and is deleted when site data is cleared. Direct user-visible file access is a separate capability with browser-dependent support and permission requirements. [MDN OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system), [Chrome File System Access](https://developer.chrome.com/docs/capabilities/web-apis/file-system-access). Therefore, an OPFS prototype does not demonstrate a portable Git-folder workflow; provide export/backup and clearly state where local data lives.

## Packaging options — comparison, not selection

| Option | Why consider it | Cost or unknown |
|---|---|---|
| Responsive web UI + local companion + optional desktop shell | Share much visual code between desktop/browser/mobile web; companion can serve GUI and headless agents from the same local store. | Installation, local connection security and lifecycle; phone access needs an explicit route. |
| Responsive web/PWA with browser-owned local storage | Lowest installation friction for initial browser/mobile access. PWA installation flows vary by platform. [web.dev installation](https://web.dev/learn/pwa/installation/). | Browser persistence/export, selected-folder compatibility and desktop-agent access; installation alone does not grant native capabilities or guarantee offline behavior. |
| Web UI in a cross-platform app framework | Tauri documents desktop/mobile apps using web frontends and native backend integration. [Tauri overview](https://v2.tauri.app/start/). | Shared frontend does not establish plugin parity, working Git on mobile or agent-runner support. Rust/native integration and distribution add work. |
| Separate native clients sharing a command/API contract | Allows each platform to follow its own conventions closely. | More presentation code and consistency work; shared business rules and compatibility still needed. This is an architecture option, not a measured superiority claim. |

Proposed starting point: define the common task operations and a responsive task list/detail/review flow, then validate the local runtime with CLI/MCP. Evaluate desktop packaging around that flow; retain a phone-friendly browser path while deciding whether native mobile benefits justify its cost. This sequence does not remove the requested desktop/mobile scope or amend milestones without owner review.

## Example shared workflow

An agent obtains a task brief, requests a claim, performs authorized work, then reports changes and evidence against the revision it read. A person sees the same task and evidence on desktop, browser or mobile. If the next step needs an exact owner decision, the agent requests it and the person resolves it in Waiting on me. A resumed session reads the recorded outcome and next dependency through CLI/MCP. UI choice does not create a second task record.

The leader's prompt → external agent → generated files workflow fits the same design: validate and preview the bundle, then adopt it through shared project rules. See [AI-generated projects](../product/AI-GENERATED-PROJECTS.md). It remains usable without an in-app chat or runner.

## Implementation questions retained

Which desktop/mobile platforms ship first; browser-only versus companion-based local storage; native versus installed web mobile; final task commands and schema; agent host/protocol compatibility; interrupted-operation and offline-claim behavior. Storage decision [0001](../decisions/0001-database-source-of-truth.md) remains accepted and Git candidate [0007](../decisions/0007-git-like-project-sync.md) remains proposed. Neither a mobile app nor this interface research resolves that choice.

Implementation checks to put on future cards: the same mutation has the same validated result from GUI/CLI/MCP; local tracking survives restart without network/sign-in; narrow-screen review preserves evidence; stale updates expose a conflict; repeating a request does not duplicate events or launches; an agent cannot approve its own restricted operation; unsupported client capabilities have a useful explanation. These are proposed acceptance scenarios, not tests run in this investigation.
