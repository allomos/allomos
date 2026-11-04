// Work on it… (Task-A03-galley051, approver 2026-11-03; named Proceed… until the approver renamed it,
// 2026-11-04): a task no session holds can be handed to an
// AI agent from its row. Allomos opens the agent with a request typed in: claim the task and work
// it, or, when it cannot start now (blocked, waiting), tell the person why and ask what to do. A
// backlog task is approved by choosing Open, so its request makes it ready first. The request is
// written here, in the interface language; the person may edit it for this one time.

import type { Language } from "./i18n";
import type { Task } from "./types";

/** Whether the row offers Work on it…: no session holds the task, and it is not finished. */
export function canProceed(task: Task): boolean {
  return !task.holder && task.state !== "done" && task.state !== "dropped";
}

/** Blocked or waiting: the agent cannot start, so it asks the person what to do instead. */
export function stopped(task: Task): boolean {
  return task.state === "blocked" || task.state === "waiting";
}

/** `items` joined as a sentence lists them: "A", "A and B", "A, B and C" (in each language). */
function joined(items: string[], language: Language): string {
  const and = { en: " and ", "zh-CN": "和", ja: "と" }[language];
  const comma = { en: ", ", "zh-CN": "、", ja: "、" }[language];
  if (items.length <= 1) return items.join("");
  return items.slice(0, -1).join(comma) + and + items[items.length - 1];
}

/** The request Allomos types in for `task`; `taskbag` is the taskbag's path as the agent, working
 * in the project folder, should pass it to `--taskbag`. */
export function proceedRequest(task: Task, taskbag: string, language: Language): string {
  const id = task.id;
  const blockers = task.blocked_by ?? [];
  const tb = `--taskbag "${taskbag}"`;
  if (language === "zh-CN") {
    if (task.state === "blocked") {
      const by = joined(blockers, language);
      return `请看看这个项目任务袋里的任务 ${id}（“${task.title}”）。每条 alm 命令都加上 ${tb}。

它现在不能开始：它受阻于 ${by}。先不要认领它，也先不要改动它。

1. 阅读：${["alm show " + id, ...blockers.map((b) => "alm show " + b)].join("、")}。
2. 用平常的话告诉我它为什么停下，以及可以怎么处理（例如：先做 ${by}；如果 ${by} 已经完成，就解除 ${id} 的受阻）。
3. 问我想让你怎么处理 ${id}，在我回答之前不要改动任何东西。`;
    }
    if (task.state === "waiting") {
      return `请看看这个项目任务袋里的任务 ${id}（“${task.title}”）。每条 alm 命令都加上 ${tb}。

它现在不能开始：它在等待：${task.waiting_on ?? ""}。先不要认领它，也先不要改动它。

1. 阅读：alm show ${id}。
2. 用平常的话告诉我它为什么停下，以及可以怎么处理（例如：回答它在等的问题，或去问它在等的人）。
3. 问我想让你怎么处理 ${id}，在我回答之前不要改动任何东西。`;
    }
    const backlog = task.state === "backlog";
    const n = backlog ? 3 : 2;
    return `请执行这个项目任务袋里的任务 ${id}（“${task.title}”）。每条 alm 命令都加上 ${tb}。

1. 阅读：alm show ${id}（它的说明和最新的交接）。
${backlog ? `2. 它在储备中。我在「换肩」里选了「执行」，这就是批准：先把它设为就绪（alm ready ${id}）。\n` : ""}${n}. 按 AGENTS.md 的说法认领它（alm claim ${id} --as <你的会话名> --branch <你的分支> ...），并把认领分享出去。
${n + 1}. 在它自己的工作树里做，工作树从认领时列出的基础分支切出。
${n + 2}. 用 alm label 报告进度；做完用 alm done，中途停下用 alm release --note。`;
  }
  if (language === "ja") {
    if (task.state === "blocked") {
      const by = joined(blockers, language);
      return `このプロジェクトのタスクバッグにあるタスク ${id}（「${task.title}」）を見てください。alm コマンドはすべて ${tb} を付けて実行してください。

今は始められません：${by} に止められています。まだ引き受けず、変更もしないでください。

1. 読む：${["alm show " + id, ...blockers.map((b) => "alm show " + b)].join("、")}。
2. なぜ止まっているのか、どうすればよいか（例：先に ${by} を進める。${by} がもう終わっていれば ${id} の止まりを解く）を、わかりやすい言葉で教えてください。
3. ${id} をどうしてほしいか私に聞き、私が答えるまで何も変更しないでください。`;
    }
    if (task.state === "waiting") {
      return `このプロジェクトのタスクバッグにあるタスク ${id}（「${task.title}」）を見てください。alm コマンドはすべて ${tb} を付けて実行してください。

今は始められません：次のことを待っています：${task.waiting_on ?? ""}。まだ引き受けず、変更もしないでください。

1. 読む：alm show ${id}。
2. なぜ止まっているのか、どうすればよいか（例：待っている質問に答える。待っている相手に聞く）を、わかりやすい言葉で教えてください。
3. ${id} をどうしてほしいか私に聞き、私が答えるまで何も変更しないでください。`;
    }
    const backlog = task.state === "backlog";
    const n = backlog ? 3 : 2;
    return `このプロジェクトのタスクバッグにあるタスク ${id}（「${task.title}」）を進めてください。alm コマンドはすべて ${tb} を付けて実行してください。

1. 読む：alm show ${id}（説明と最新の引き継ぎ）。
${backlog ? `2. このタスクはバックログにあります。私が「肩代」で「取りかかる」を選んだので、これは承認です：まず準備完了にしてください（alm ready ${id}）。\n` : ""}${n}. AGENTS.md のとおりに引き受け（alm claim ${id} --as <あなたのセッション名> --branch <あなたのブランチ> ...）、その引き受けを共有してください。
${n + 1}. 専用のワークツリーで作業してください。ワークツリーは、引き受けたときに表示されるベースブランチから作ります。
${n + 2}. 進み具合は alm label で知らせ、終わったら alm done、途中でやめるなら alm release --note を使ってください。`;
  }
  if (task.state === "blocked") {
    const by = joined(blockers, language);
    return `Look at task ${id} ("${task.title}") in this project's taskbag. Run every alm command with ${tb}.

It cannot start now: it is blocked by ${by}. Do not claim it and do not change it yet.

1. Read it: ${joined(["alm show " + id, ...blockers.map((b) => "alm show " + b)], language)}.
2. Tell me in plain words why it is stopped and what could be done about it (for example: work ${by} first, or unblock ${id} if ${by} ${blockers.length > 1 ? "are" : "is"} already done).
3. Ask me what I want you to do with ${id}, and wait for my answer before you change anything.`;
  }
  if (task.state === "waiting") {
    return `Look at task ${id} ("${task.title}") in this project's taskbag. Run every alm command with ${tb}.

It cannot start now: it is waiting on: ${task.waiting_on ?? ""}. Do not claim it and do not change it yet.

1. Read it: alm show ${id}.
2. Tell me in plain words why it is stopped and what could be done about it (for example: answer the question it waits on, or ask the person it waits on).
3. Ask me what I want you to do with ${id}, and wait for my answer before you change anything.`;
  }
  const backlog = task.state === "backlog";
  const n = backlog ? 3 : 2;
  return `Work on task ${id} ("${task.title}") in this project's taskbag. Run every alm command with ${tb}.

1. Read it: alm show ${id} (its description and latest handoff note).
${backlog ? `2. It is in backlog. I chose Work on it in Allomos, which approves it: make it ready (alm ready ${id}).\n` : ""}${n}. Claim it as AGENTS.md says (alm claim ${id} --as <your session name> --branch <your branch> ...), and share the claim.
${n + 1}. Work it in a worktree of its own, cut from the base branch the claim prints.
${n + 2}. Report progress with alm label; finish with alm done, or stop with alm release --note.`;
}

/** The person's edited request, or null when it says the same as Allomos's (then it is not edited). */
export function edited(text: string, original: string): string | null {
  return text === original ? null : text;
}
