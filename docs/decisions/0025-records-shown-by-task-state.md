# 0025 — Records people read: shown by the task's state, written for people first

| Status | Raised | Decided | Owner |
|---|---|---|---|
| **Accepted** (direction; each piece is its own task); point 3 corrected the same day | 2026-11-12 | 2026-11-13 | will.w.hui |

Card: task `A01-cup419`. Discussion: [2026-11-13-readable-records.md](../discussions/2026-11-13-readable-records.md).

## In plain words

What Allomos records for a task (handoff note, description, evidence, AI decisions, human check steps) is tiring to read. The main trouble is not one bad field: there is simply **too much, and nothing tells a person which part matters for the task's state right now**. A task waiting for a check needs the check steps first; a task with AI decisions to review needs only the decisions a user would see; a lost task needs the handoff for the next agent.

**Decided:**

1. **Agents write for people first** (shared rules in AGENTS.md and `alm guide`). Each handoff note, AI decision and set of check steps starts with one plain sentence a person can act on, then what the person must do, and only then technical detail (commits, commands, IDs). No data format change.
2. **The app then shows records by the task's state.** The task panel and the review lists put first the part that the task's current state calls for, and fold the rest. No data format change.
3. **AI decisions come in three kinds**, and two of them are reviewed:
   - *visible to users* — wording, look, behaviour a person meets. **Reviewed.**
   - *approach* — the principle of how something is built and why it works that way (for example: finding the former worktree by the task's working branch rather than by the folder the claim ran in). An approach can be wrong even when everything users see is right, so it is **reviewed** too.
   - *other detail* — sizes, limits, flags, names inside the code (for example: a list box 160px high, at most 200 commits). Recorded and shown in the task, but **not** put in *To review: AI decisions*.

   **This changes the data format** (a kind on each AI decision), approved here.
4. **Fixed words are shown in the task language**: labels, states and similar fixed terms appear translated wherever people read them; the stored values stay as they are.

## The approver's words

On 2026-11-13, in the conversation, the approver chose "先改写法再改显示" (rules first, then the app's display), agreed that AI decisions are split by kind ("可以，分两种"), and then corrected the split the same day: "根据我的经验，有时候实现方法也是有问题的。所以实现细节可以分成两个方面，一方面是实现方法的原理性说明。另一方面是其他细节。前者应该是要用户审核的。" (In my experience the implementation approach is sometimes wrong too, so implementation detail has two sides: the principle of the approach, which users should review, and the other details.) Point 3 is written to that correction; and chose to show labels in the task language. Asked which record is most tiring, they answered: "很难说。感觉是内容太多了。也许首要的问题是，没有办法按照当前任务所处的状态，快速找到/决定需要看的部分。" — that answer is why point 2 is the centre of this decision.

## Safe to build now

All four points. Older AI decisions have no kind: they stay in review as they are, so nothing drops out of review unseen. When an agent is unsure whether a decision is an *approach* or an *other detail*, it records it as an *approach*.
