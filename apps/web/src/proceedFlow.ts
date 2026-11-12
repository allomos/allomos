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

/** Whether Take over… can be offered (Task-A01-rocket260, decision 0022 § 6): a session holds the
 * task and it is not finished, lost or not. */
export function canTakeOver(task: Task): boolean {
  return !!task.holder && task.state !== "done" && task.state !== "dropped";
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

/** Take over… (Task-A01-voyage603): what the request needs besides the task. */
export interface TakeOver {
  /** The lost limit that applied, in minutes; null when the holder is not lost and the person asks
   * for the takeover anyway (Task-A01-rocket260). */
  lostAfter: number | null;
  /** The task's working branch, as the history records it; null when none is recorded. */
  working: string | null;
  /** Set when the person chose to have the former worktree's changes committed and pushed first
   * (Task-A01-flag101): that worktree as the dialog listed it. */
  save?: FormerWorktree | null;
}

/** The former holder's worktree on this computer and what it has not pushed (Task-A01-flag101),
 * as the desktop app reads it (`former_worktree`). */
export interface FormerWorktree {
  path: string;
  files: { kind: "modified" | "added" | "untracked" | "deleted"; path: string }[];
  commits: { hash: string; subject: string }[];
}

/** Whether the former worktree holds anything not pushed. */
export function hasUnpushed(w: FormerWorktree): boolean {
  return w.files.length > 0 || w.commits.length > 0;
}

/** How many files the request names before it says how many more there are. */
const NAMED_FILES = 30;

/** The files the request names, joined: at most `NAMED_FILES`, then how many more. */
function fileList(w: FormerWorktree, language: Language): string {
  const names = w.files.slice(0, NAMED_FILES).map((f) => f.path);
  const more = w.files.length - names.length;
  const sep = language === "en" ? ", " : "、";
  const tail = more > 0 ? { en: ` and ${more} more`, "zh-CN": `，另外还有 ${more} 个`, ja: `、ほか ${more} 件` }[language] : "";
  return names.join(sep) + tail;
}

/** The step that commits and pushes what the former worktree holds (Task-A01-flag101), numbered `n`;
 * empty when nothing is to be saved. */
function saveStep(n: number, id: string, holder: string, working: string, w: FormerWorktree, language: Language): string {
  const at = `git -C "${w.path}"`;
  const files = w.files.length > 0;
  const commits = w.commits.length;
  if (language === "zh-CN") {
    const commit = files
      ? `把没提交的文件（${fileList(w, language)}）作为一个 WIP 提交（${at} add -A，然后 ${at} commit -m "WIP: 接手前替 ${holder} 保存的改动（${id}）"，带上你的署名行），`
      : "";
    const with_ = commits > 0 ? `连同已提交但没推送的 ${commits} 个提交，` : "";
    return `${n}. 我在「换肩」里选了「先替它提交并推送这些改动」，所以这一次你可以改动前一个持有者的工作树：${w.path}。在那里${commit}${with_}把工作分支 ${working} 推送到 origin（${at} push origin ${working}），并确认没有「ahead」（${at} status -sb）。此外不要改动、删除那里的任何东西。你看到的改动和我在「换肩」里看到的不一样时，先问我。
`;
  }
  if (language === "ja") {
    const commit = files
      ? `コミットされていないファイル（${fileList(w, language)}）を一つの WIP コミットにし（${at} add -A、続けて ${at} commit -m "WIP: 引き継ぎ前に ${holder} の代わりに保存した変更（${id}）"、あなたの署名行を付けて）、`
      : "";
    const with_ = commits > 0 ? `コミット済みでまだプッシュされていない ${commits} 件のコミットとともに、` : "";
    return `${n}. 私が「肩代」で「先にこれらの変更を代わりにコミットしてプッシュする」を選んだので、今回は前の持ち主のワークツリーを変更してかまいません：${w.path}。そこで${commit}${with_}作業ブランチ ${working} を origin にプッシュし（${at} push origin ${working}）、「ahead」がないことを確かめてください（${at} status -sb）。それ以外は、そこにあるものを変更・削除しないでください。見つかった変更が私が「肩代」で見たものと違うときは、先に私に聞いてください。
`;
  }
  const commit = files
    ? `commit the files it has not committed (${fileList(w, language)}) as one WIP commit (${at} add -A, then ${at} commit -m "WIP: changes saved for ${holder} before the takeover (${id})", with your attribution line), then `
    : "";
  const with_ = commits > 0 ? ` with the ${commits} commit${commits > 1 ? "s" : ""} it committed but did not push` : "";
  return `${n}. I chose "Commit and push these changes for it first" in Allomos, so this once you may change the former holder's worktree: ${w.path}. There, ${commit}push the working branch ${working}${with_} to origin (${at} push origin ${working}), and make sure it is not "ahead" (${at} status -sb). Apart from that, change or delete nothing there. If the changes you find differ from what I saw in Allomos, ask me first.
`;
}

/** The request Allomos types in to take a lost task over (Task-A01-voyage603): claiming a lost task
 * is the takeover (Task-A01-sign613); the work goes on on the task's working branch; the former
 * holder's worktree may hold changes it never committed, which the new session looks at but leaves
 * alone. A holder that is not lost (Task-A01-rocket260) needs `--take`, which the request says the
 * person asked for, and may still be at work, so the new session is told to expect its pushes.
 * `taskbag` as in `proceedRequest`. */
export function takeOverRequest(task: Task, taskbag: string, language: Language, take: TakeOver): string {
  const id = task.id;
  const holder = task.holder ?? "";
  const tb = `--taskbag "${taskbag}"`;
  const w = take.working;
  const held = take.lostAfter === null;
  const flag = held ? " --take" : "";
  // Saving needs the working branch and something to save (Task-A01-flag101).
  const save = w && take.save && hasUnpushed(take.save) ? take.save : null;
  const n = save ? 4 : 3;
  if (language === "zh-CN") {
    return `请接手这个项目任务袋里${held ? "的" : "失联的"}任务 ${id}（“${task.title}”）。每条 alm 命令都加上 ${tb}。

${held ? `它由 ${holder} 持有，没有失联：原会话可能还在工作。这次接手是我在「换肩」里要求的，所以可以用 --take。` : `它由 ${holder} 持有，但超过 ${take.lostAfter} 分钟没有心跳，也没有未到期的忙碌窗口：它失联了。`}

1. 阅读：alm show ${id}（它的说明和最新的交接）。
2. 按 AGENTS.md 的说法认领它${held ? "，带上 --take" : ""}，这就是接手（alm claim ${id}${flag} --as <你的会话名> --branch <你的分支> ...），并把认领分享出去。
${save ? saveStep(3, id, holder, w!, save, language) : ""}${n}. 在原工作分支${w ? ` ${w}` : "（认领时列出）"}上继续：为它开一个你自己的工作树（git worktree add <文件夹> ${w ?? "<工作分支>"}，不带 -b）。
${n + 1}. 只有推送过的东西才算数：从工作分支上的提交和交接说明接着做。${save ? `前一个会话的工作树里没推送的改动，第 3 步已经推送；除了第 3 步，不要改动、提交或删除那里的东西。` : `前一个会话的工作树（认领时列出）里可能有它没提交的改动，但那个文件夹只在它所在的电脑上有用：在同一台电脑上可以去看，但不要改动、提交或删除那里的东西；在另一台电脑上就看工作分支和交接说明。`}拿不准就问我。
${held ? `${n + 2}. 原会话要到下一次运行 alm 才知道任务被接手，在这之前它可能还会往这个分支推送：提交前先取最新的改动；如果它的提交和你的冲突，告诉我。\n${n + 3}` : `${n + 2}`}. 用 alm label 报告进度；做完用 alm done，中途停下用 alm release --note。`;
  }
  if (language === "ja") {
    return `このプロジェクトのタスクバッグにある${held ? "" : "応答なしの"}タスク ${id}（「${task.title}」）を引き継いでください。alm コマンドはすべて ${tb} を付けて実行してください。

${held ? `${holder} が引き受けていて、応答なしではありません：元のセッションはまだ作業中かもしれません。この引き継ぎは私が「肩代」で頼んだものなので、--take を使ってかまいません。` : `${holder} が引き受けていますが、${take.lostAfter} 分を超えてハートビートがなく、作業中の期間もありません：応答なしです。`}

1. 読む：alm show ${id}（説明と最新の引き継ぎ）。
2. AGENTS.md のとおりに${held ? "、--take を付けて" : ""}引き受けてください。それが引き継ぎになります（alm claim ${id}${flag} --as <あなたのセッション名> --branch <あなたのブランチ> ...）。その引き受けを共有してください。
${save ? saveStep(3, id, holder, w!, save, language) : ""}${n}. 元の作業ブランチ${w ? ` ${w}` : "（引き受けたときに表示されます）"}で続けてください：自分専用のワークツリーを作ります（git worktree add <フォルダー> ${w ?? "<作業ブランチ>"}、-b なし）。
${n + 1}. プッシュされたものだけが引き継げます：作業ブランチのコミットと引き継ぎメモから続けてください。${save ? `前のセッションのワークツリーにあったまだプッシュされていない変更は、手順 3 でプッシュ済みです。手順 3 のほかは、そこにあるものを変更・コミット・削除しないでください。` : `前のセッションのワークツリー（引き受けたときに表示されます）にはコミットされていない変更があるかもしれませんが、そのフォルダーが役に立つのは同じコンピューターの上だけです。同じコンピューターでは見てもかまいませんが、そこにあるものを変更・コミット・削除しないでください。別のコンピューターでは作業ブランチと引き継ぎメモを見てください。`}迷ったら私に聞いてください。
${held ? `${n + 2}. 元のセッションは次に alm を実行するまで引き継がれたことを知らないので、それまでこのブランチにプッシュするかもしれません：コミットの前に最新の変更を取り込み、そのコミットとぶつかったら私に教えてください。\n${n + 3}` : `${n + 2}`}. 進み具合は alm label で知らせ、終わったら alm done、途中でやめるなら alm release --note を使ってください。`;
  }
  return `Take over ${held ? "" : "the lost "}task ${id} ("${task.title}") in this project's taskbag. Run every alm command with ${tb}.

${held ? `${holder} holds it and is not lost: its session may still be at work. I asked for this takeover in Allomos, so you may use --take.` : `${holder} holds it, but has sent no heartbeat for over ${take.lostAfter} minutes and has no busy window running: it is lost.`}

1. Read it: alm show ${id} (its description and latest handoff note).
2. Claim it as AGENTS.md says${held ? ", with --take" : ""}; that is the takeover (alm claim ${id}${flag} --as <your session name> --branch <your branch> ...), and share the claim.
${save ? saveStep(3, id, holder, w!, save, language) : ""}${n}. Go on on its working branch${w ? `, ${w}` : " (the claim prints it)"}: open a worktree of your own for it (git worktree add <folder> ${w ?? "<working branch>"}, without -b).
${n + 1}. Only what was pushed counts: go on from the commits on the working branch and the handoff note. ${save ? `Step 3 pushed what the former session's worktree had not; apart from step 3, do not change, commit or delete anything there.` : `The former session's worktree (the claim prints where) may hold changes it never committed, but that folder is of use only on its own computer: on the same computer, look at them if you need to, but do not change, commit or delete anything there; on another computer, go by the working branch and the handoff note.`} If unsure, ask me.
${held ? `${n + 2}. That session learns of the takeover only the next time it runs alm, so it may still push to the branch until then: get the latest changes before you commit, and if its commits collide with yours, tell me.\n${n + 3}` : `${n + 2}`}. Report progress with alm label; finish with alm done, or stop with alm release --note.`;
}
