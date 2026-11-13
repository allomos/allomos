# 0025 — Records people read: shown by the task's state, written for people first

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** (direction; each piece is its own task) | 2026-11-12 | 2026-11-13 | will.w.hui |

Card: task `A01-cup419`. Discussion: [2026-11-13-readable-records.md](../discussions/2026-11-13-readable-records.md).

## In plain words

What Allomos records for a task (handoff note, description, evidence, AI decisions, human check steps) is tiring to read. The main trouble is not one bad field: there is simply **too much, and nothing tells a person which part matters for the task's state right now**. A task waiting for a check needs the check steps first; a task with AI decisions to review needs only the decisions a user would see; a lost task needs the handoff for the next agent.

**Decided:**

1. **Agents write for people first** (shared rules in AGENTS.md and `alm guide`). Each handoff note, AI decision and set of check steps starts with one plain sentence a person can act on, then what the person must do, and only then technical detail (commits, commands, IDs). No data format change.
2. **The app then shows records by the task's state.** The task panel and the review lists put first the part that the task's current state calls for, and fold the rest. No data format change.
3. **AI decisions come in two kinds** — *visible to users* (wording, look, behaviour a person meets) and *implementation detail*. Both are recorded; only the first kind goes to *To review: AI decisions*. **This changes the data format** (a kind on each AI decision), approved here.
4. **Fixed words are shown in the task language**: labels, states and similar fixed terms appear translated wherever people read them; the stored values stay as they are.

## The approver's words

On 2026-11-13, in the conversation, the approver chose "先改写法再改显示" (rules first, then the app's display), agreed that AI decisions are split into two kinds with only the visible kind to review ("可以，分两种"), and chose to show labels in the task language. Asked which record is most tiring, they answered: "很难说。感觉是内容太多了。也许首要的问题是，没有办法按照当前任务所处的状态，快速找到/决定需要看的部分。" — that answer is why point 2 is the centre of this decision.

## Safe to build now

All four points. Older AI decisions have no kind: until a task re-records them they count as *visible to users*, so nothing drops out of review unseen.
