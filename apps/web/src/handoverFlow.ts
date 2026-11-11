// Hand over to another agent… (Task-A01-minnow200): a person asks the session holding a task to
// hand it over. The dialog has three steps: choose the new agent; ask the holding session, which
// writes its handoff and releases the task; once it has, open a new session that claims the task
// and goes on on its working branch. So two sessions never work on one branch at once. The request
// is written here, in the interface language. Kept apart from React so it can be tested with Node.

import type { Language } from "./i18n";
import type { HandoverRequest, Task } from "./types";

/** The request still in force: the task is not finished and its holder is the session it names
 * (the same rule as `allomos_core::handover::pending`). */
export function pendingHandover(task: Task): HandoverRequest | null {
  const r = task.handover_request;
  if (!r || task.state === "done" || task.state === "dropped" || task.holder !== r.holder) return null;
  return r;
}

export type HandoverStep = "choose" | "wait" | "open" | "gone";

/** The dialog's step: `asked` is the holder asked in this dialog (or by an earlier one), null before
 * asking. Released: go on to the new session. Finished, or held by someone else now: nothing to do. */
export function handoverStep(task: Task, asked: string | null): HandoverStep {
  if (asked === null) return "choose";
  if (task.state === "done" || task.state === "dropped") return "gone";
  if (!task.holder) return "open";
  return task.holder === asked ? "wait" : "gone";
}

const parse = (stamp: string): number => new Date(stamp.replace(/T(\d\d:\d\d)Z$/, "T$1:00Z")).getTime();

/** Whether the holder has had a full heartbeat interval to see the request and has not released
 * the task: time to suggest Take over…. */
export function handoverLate(request: HandoverRequest | null, intervalMinutes: number, now: Date): boolean {
  if (!request) return false;
  const at = parse(request.at);
  return !Number.isNaN(at) && now.getTime() - at > intervalMinutes * 60_000;
}

/** What the person sends to the holding session to have it hand over at once. It already knows how
 * it runs `alm`, so the request names no taskbag. */
export function handoverAskRequest(task: Task, language: Language): string {
  const id = task.id;
  if (language === "zh-CN") {
    return `请把任务 ${id}（“${task.title}”）交给另一个智能体：这是我在「换肩」里要求的。

1. 做完手头这一小步，不要再开始新的工作。
2. 写好交接：alm handoff ${id} --as <你的会话名> --text "<做到哪里、下一步是什么、在等谁>"。
3. 把工作提交并推送到工作分支。
4. 释放任务：alm release ${id} --as <你的会话名> --note "<下一位需要知道的，以及工作分支>"，并把这个改动分享出去。
5. 之后停止做这个任务：会有新会话接着做。`;
  }
  if (language === "ja") {
    return `タスク ${id}（「${task.title}」）を別のエージェントに渡してください。私が「肩代」で頼んだものです。

1. 今の小さな作業を終え、新しいことは始めないでください。
2. 引き継ぎを書く：alm handoff ${id} --as <あなたのセッション名> --text "<どこまで進んだか、次の一歩、誰を待っているか>"。
3. 作業を作業ブランチにコミットしてプッシュしてください。
4. タスクを手放す：alm release ${id} --as <あなたのセッション名> --note "<次の人に必要なこと、作業ブランチ>"。その変更を共有してください。
5. そのあとはこのタスクの作業をやめてください：新しいセッションが続きを行います。`;
  }
  return `Please hand task ${id} ("${task.title}") over to another agent: I asked for this in Allomos.

1. Finish the small step you are on and start nothing new on it.
2. Write your handoff: alm handoff ${id} --as <your session name> --text "<where it stopped, the one next step, who it waits on>".
3. Commit and push your work to the working branch.
4. Release the task: alm release ${id} --as <your session name> --note "<what the next one needs, and the working branch>", and share that change.
5. Then stop work on it: a new session takes it from there.`;
}

/** The request Allomos types in for the new session, once `holder` has released the task. `taskbag`
 * as in `proceedRequest`; `working` is the task's working branch, null when none is recorded. */
export function handoverClaimRequest(task: Task, taskbag: string, language: Language, holder: string, working: string | null): string {
  const id = task.id;
  const tb = `--taskbag "${taskbag}"`;
  const w = working;
  const flag = w ? ` --working ${w}` : "";
  if (language === "zh-CN") {
    return `请接着做这个项目任务袋里的任务 ${id}（“${task.title}”）。每条 alm 命令都加上 ${tb}。

这是我在「换肩」里要求的换手：${holder} 已经写好交接并释放了它。

1. 阅读：alm show ${id}（它的说明和最新的交接）。
2. 按 AGENTS.md 的说法认领它（alm claim ${id} --as <你的会话名> --branch <你的分支>${flag} ...），并把认领分享出去。
3. 在原工作分支${w ? ` ${w}` : "（交接说明里写着）"}上继续：为它开一个你自己的工作树（git worktree add <文件夹> ${w ?? "<工作分支>"}，不带 -b）。
4. 只有推送过的东西才算数：从工作分支上的提交和交接说明接着做。前一个会话的工作树不要改动、提交或删除。
5. 用 alm label 报告进度；做完用 alm done，中途停下用 alm release --note。`;
  }
  if (language === "ja") {
    return `このプロジェクトのタスクバッグにあるタスク ${id}（「${task.title}」）の続きをお願いします。alm コマンドはすべて ${tb} を付けて実行してください。

私が「肩代」で頼んだ交代です：${holder} は引き継ぎを書いて、タスクを手放しました。

1. 読む：alm show ${id}（説明と最新の引き継ぎ）。
2. AGENTS.md のとおりに引き受け（alm claim ${id} --as <あなたのセッション名> --branch <あなたのブランチ>${flag} ...）、その引き受けを共有してください。
3. 元の作業ブランチ${w ? ` ${w}` : "（引き継ぎメモに書かれています）"}で続けてください：自分専用のワークツリーを作ります（git worktree add <フォルダー> ${w ?? "<作業ブランチ>"}、-b なし）。
4. プッシュされたものだけが引き継げます：作業ブランチのコミットと引き継ぎメモから続けてください。前のセッションのワークツリーは変更・コミット・削除しないでください。
5. 進み具合は alm label で知らせ、終わったら alm done、途中でやめるなら alm release --note を使ってください。`;
  }
  return `Go on with task ${id} ("${task.title}") in this project's taskbag. Run every alm command with ${tb}.

I asked for this change of hands in Allomos: ${holder} has written its handoff and released the task.

1. Read it: alm show ${id} (its description and latest handoff note).
2. Claim it as AGENTS.md says (alm claim ${id} --as <your session name> --branch <your branch>${flag} ...), and share the claim.
3. Go on on its working branch${w ? `, ${w}` : " (the handoff note names it)"}: open a worktree of your own for it (git worktree add <folder> ${w ?? "<working branch>"}, without -b).
4. Only what was pushed counts: go on from the commits on the working branch and the handoff note. Do not change, commit or delete anything in the former session's worktree.
5. Report progress with alm label; finish with alm done, or stop with alm release --note.`;
}
