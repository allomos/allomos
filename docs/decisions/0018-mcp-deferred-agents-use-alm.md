# 0018 — MCP is deferred: agents use the `alm` command

| Status | Raised | Decided | Approver |
|---|---|---|---|
| **Accepted** | 2026-10-24 | 2026-10-24 | will.w.hui |

Card: task `A00-ferry295`. Decided by the approver on 2026-10-24 ("move all tasks in I03 into I02, then defer MCP", in chat with session claude/defer-mcp).

## In plain words

MCP (Model Context Protocol) is a standard way for an AI agent to discover and call a program's tools without a command line. Iteration 3 was going to add an MCP server (`alm mcp`) so that Claude Code and Codex could use Allomos through MCP tools as well as through the `alm` command.

**Decision:** we do not build it now. Agents keep using `alm`, which already does what the MCP server was meant to add. MCP comes back only when an agent that cannot run a command needs Allomos.

## Context

What Iteration 3 promised ([PLAN.md](../product/PLAN.md), owner, 2026-10-07), and where `alm` already stands:

| Promised for the MCP server | Already in `alm` |
|---|---|
| Safe retries: repeating a request does not repeat its effect | `--request-id` on every change (task `A01-58`) |
| Stale-edit checks | `--expected-version` on changes such as `describe` |
| Structured results for agents | `--json`, exit codes 0–4 |
| Instructions that tell agents when to use it | The AGENTS.md section (`alm brief`), `alm guide …`, `alm next` |

- Claude Code and Codex, the agents Allomos serves, both run shell commands, and every session on this project has worked its tasks with `alm` since v0.1.
- On 2026-10-20 the approver set that agents go through `alm` (R63).
- Iteration 3 had no open tasks; its five done tasks (releases 0.1.21 and 0.1.22 and their fixes) were planned in I02 the same day.
- MCP maintainers report tool overload, client-dependent discovery and agents choosing the CLI instead ([interface-patterns.md](../research/interface-patterns.md), row "Small tool groups; CLI remains useful").

## Decision

1. **No MCP server for now.** No `crates/mcp`, no `alm mcp`, no `.mcp.json` written by `alm init` or the app.
2. **Agents use `alm`** for every task operation; R11 is met by the CLI.
3. **Iteration 3 is closed:** renamed "MCP for agents (deferred)" and closed, with no tasks (approver, 2026-10-24). Iteration 4 (Progress view) is the active iteration. When MCP returns, it gets a new iteration.
4. **MCP comes back** (a new decision, on the approver's word) when one of these is real:
   - an agent with no shell must use Allomos: Claude or ChatGPT chat apps, phone clients, the companions (v0.9–v0.10) or a cloud project (v0.13–v0.14). That would most likely be a **remote** MCP server run by the hub or the cloud service, not `alm mcp` on the same computer;
   - per-tool permissions are needed for agents in a cloud project;
   - interactive views inside the agent's window (MCP Apps), such as a decision form or a change preview, are worth their cost.

## Alternatives considered

| Option | Why not now |
|---|---|
| Build `alm mcp` as planned | A second interface over the same commands: names, errors and docs to keep in step with `alm`, with no agent that needs it. |
| A thin MCP server that only runs `alm` | Same upkeep and setup (`.mcp.json`, approving the server in each agent), and every session loads tool schemas it rarely needs. |
| Drop MCP for good | Shell-less agents and cloud projects are on the roadmap; deferring keeps the door open at no cost. |

## Consequences

- **Plan and requirements:** R11, R62, R63, R82 and R86 say MCP is deferred; PLAN.md drops `crates/mcp` from the layout, marks Iteration 3 deferred and removes its exit check; the READMEs drop "MCP planned, v0.3".
- **Older decision records** (0001, 0005, 0009, 0011, 0014) that mention an MCP server are read with this record: where they say "CLI and MCP", the CLI alone holds until MCP returns.
- **The shared Rust code** stays the one place for project rules, so a later MCP server can reuse it as `alm` does.
- Nothing users already have is removed: no MCP server was ever shipped.
