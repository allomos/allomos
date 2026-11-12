// Prepare a handoff… (Task-A01-kind858, replacing A01-minnow200's Hand over to another agent…): a
// person has the session holding a task get ready for the next one. The dialog shows a prompt
// (write the handoff, commit and push, release the task, then stop), the person copies it, opens
// the holding session and pastes it there. Nothing is written to the taskbag; who goes on with
// the task is started afterwards with another button (Work on it… once released, Take over… when
// the session is gone). Kept apart from React so it can be tested with Node.

import type { Language } from "./i18n";
import type { Task } from "./types";

/** Whether Open its session can open the holder's session, and if not, why:
 * - `no_app`: this window cannot open agent sessions (the web and mobile companions);
 * - `not_recorded`: the claim recorded no session to reopen (older claims, other agents);
 * - `other_session`: the recorded session is not the holder's (its claim passed none). */
export type HandoverOpen = { can: true } | { can: false; why: "no_app" | "not_recorded" | "other_session" };

export function handoverOpen(task: Task, canOpenSession: boolean): HandoverOpen {
  if (!canOpenSession) return { can: false, why: "no_app" };
  const s = task.session;
  if (!s?.app_session_id?.trim() && !s?.id?.trim()) return { can: false, why: "not_recorded" };
  const by = s.by?.trim();
  if (by && task.holder && by !== task.holder) return { can: false, why: "other_session" };
  return { can: true };
}

/** What the person pastes into the holding session to have it get ready for a handoff. It already
 * knows how it runs `alm`, so the prompt names no taskbag. */
export function handoverAskRequest(task: Task, language: Language): string {
  const id = task.id;
  if (language === "zh-CN") {
    return `请为任务 ${id}（“${task.title}”）做好交接准备：这是我在「换肩」里要求的。

1. 做完手头这一小步，不要再开始新的工作。
2. 写好交接：alm handoff ${id} --as <你的会话名> --text "<做到哪里、下一步是什么、在等谁>"。
3. 把工作提交并推送到工作分支。
4. 释放任务：alm release ${id} --as <你的会话名> --note "<下一位需要知道的，以及工作分支>"，并把这个改动分享出去。
5. 之后停止做这个任务：由谁接着做，我会另外安排。`;
  }
  if (language === "ja") {
    return `タスク ${id}（「${task.title}」）の引き継ぎの準備をしてください。私が「肩代」で頼んだものです。

1. 今の小さな作業を終え、新しいことは始めないでください。
2. 引き継ぎを書く：alm handoff ${id} --as <あなたのセッション名> --text "<どこまで進んだか、次の一歩、誰を待っているか>"。
3. 作業を作業ブランチにコミットしてプッシュしてください。
4. タスクを手放す：alm release ${id} --as <あなたのセッション名> --note "<次の人に必要なこと、作業ブランチ>"。その変更を共有してください。
5. そのあとはこのタスクの作業をやめてください：誰が続けるかは私が別に決めます。`;
  }
  return `Please get task ${id} ("${task.title}") ready for a handoff: I asked for this in Allomos.

1. Finish the small step you are on and start nothing new on it.
2. Write your handoff: alm handoff ${id} --as <your session name> --text "<where it stopped, the one next step, who it waits on>".
3. Commit and push your work to the working branch.
4. Release the task: alm release ${id} --as <your session name> --note "<what the next one needs, and the working branch>", and share that change.
5. Then stop work on it: I will start whoever goes on with it separately.`;
}
