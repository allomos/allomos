//! What `alm add task` prints after its Git commands (Task-A03-33, Task-A01-root343): the next
//! steps for the agent who added the task, in the task language. Moved out of `guide.rs`
//! (Task-A00-wheat560) to keep that file under its line limit.

/// What `alm add task` prints after its Git commands (Task-A03-33), in the task language
/// (`en`, `zh-CN`, `ja`; English otherwise): check the iteration it went into (`planned_in`, the
/// active one or the one asked for, Task-A01-root343), or plan it in one from `iterations` (one
/// line each, as `alm list` heads them) when it went into none, then make it ready unless
/// something in it is the person's to decide. With the project's setting `wait_for_approval` on
/// (`alm new-task-settings`, Task-A00-wheat560), it stays in backlog until the approver agrees,
/// except in a goal run. alm tells; the agent decides.
pub fn added_task_steps(
    language: &str,
    id: &str,
    iterations: &[String],
    planned_in: Option<&str>,
    wait_for_approval: bool,
) -> String {
    let listed: String = iterations
        .iter()
        .flat_map(|i| ["     ", i.as_str(), "\n"])
        .collect();
    match language {
        "zh-CN" => {
            let zh_ready = if wait_for_approval {
                format!(
                    "有：在描述的备注里写明每个待定问题和你的提案。有没有都留在储备：这个项目的新任务要等批准者同意才就绪（`alm new-task-settings`）。把任务编号 {id} 告诉批准者，等他同意后再 `alm ready {id}`；不用停下等，接着做别的。例外：目标运行（`alm guide goal`）里照旧自己设为就绪，因为提交目标已事先批准了计划。"
                )
            } else {
                format!(
                    "没有：把它设为就绪，`alm ready {id}`。有：留在储备，在描述的备注里写明每个待定问题和你的提案，并告诉那个人。就绪的任务不带未决问题。"
                )
            };
            let list = if iterations.is_empty() {
                "   还没有迭代：先用 `alm add iteration` 添加一个。\n".to_string()
            } else {
                listed
            };
            let step1 = match planned_in {
                Some(i) => {
                    format!("1. 它已放进迭代 {i}。如果它属于别的迭代：`alm plan {id} <迭代>`。\n")
                }
                None => format!(
                    "1. 没有进行中的迭代（也没有计划中的迭代），所以它不在任何迭代里。把它计划进合适的迭代：`alm plan {id} <迭代>`（有进行中的迭代时，新任务会自动放进去：标为进行中的那个，没有就是第一个计划中的迭代）：\n{list}"
                ),
            };
            format!(
                "接下来（给添加它的智能体）：
{step1}2. 检查任务里有没有该由人先定的事：数据格式、删掉东西、权限和安全、改变计划或其他任务的事、有花费或对外影响的事（用词和样子不算：做的人决定，发布前审）。{zh_ready}
"
            )
        }
        "ja" => {
            let ja_ready = if wait_for_approval {
                format!(
                    "あれば説明の備考に未決の問いとあなたの案を一つずつ書きます。どちらでも backlog のまま残します：このプロジェクトでは新しいタスクは承認者が同意するまで準備完了にしません（`alm new-task-settings`）。タスクの ID {id} を承認者に伝え、同意を得てから `alm ready {id}` にします。止まって待たず、ほかの作業を続けます。例外：ゴールの実行（`alm guide goal`）では今まで通り自分で準備完了にします。ゴールを出したことで計画は前もって承認されています。"
                )
            } else {
                format!(
                    "なければ準備完了にします：`alm ready {id}`。あれば backlog のまま残し、説明の備考に未決の問いとあなたの案を一つずつ書いて、その人に伝えます。準備完了のタスクは未決の問いを持ちません。"
                )
            };
            let list = if iterations.is_empty() {
                "   イテレーションがまだありません：先に `alm add iteration` で追加します。\n"
                    .to_string()
            } else {
                listed
            };
            let step1 = match planned_in {
                Some(i) => format!(
                    "1. イテレーション {i} に入れました。別のイテレーションのものなら：`alm plan {id} <イテレーション>`。\n"
                ),
                None => format!(
                    "1. 進行中のイテレーションがない（計画中のイテレーションもない）ので、どのイテレーションにも入っていません。合うものに計画します：`alm plan {id} <イテレーション>`（進行中のイテレーションがあれば、新しいタスクは自動でそこに入ります：進行中にしたもの、なければ最初の計画中のもの）：\n{list}"
                ),
            };
            format!(
                "次に（追加したエージェントへ）：
{step1}2. 人が先に決めるべきことがタスクにないか確かめます：データの形式、何かを削除すること、権限とセキュリティ、計画やほかのタスクを変えること、費用や外への影響があること（言葉や見た目は含みません：作る人が決め、リリース前に見直します）。{ja_ready}
"
            )
        }
        _ => {
            let en_ready = if wait_for_approval {
                format!(
                    "Something: name each open question with your proposal in the description's Notes. Either way, leave it in backlog: this project keeps new tasks in backlog until the approver agrees (`alm new-task-settings`). Tell the approver its ID, {id}, and make it ready (`alm ready {id}`) only once they agree; do not stop to wait, go on with other work. The exception: in a goal run (`alm guide goal`) make it ready yourself as usual, since submitting the goal approved its plan in advance."
                )
            } else {
                format!(
                    "Nothing: make it ready, `alm ready {id}`. Something: leave it in backlog, name each open question with your proposal in the description's Notes, and tell the person. A ready task carries no open questions."
                )
            };
            let list = if iterations.is_empty() {
                "   There is no iteration yet: add one first with `alm add iteration`.\n"
                    .to_string()
            } else {
                listed
            };
            let step1 = match planned_in {
                Some(i) => format!(
                    "1. It went into iteration {i}. If it belongs in another: `alm plan {id} <iteration>`.\n"
                ),
                None => format!(
                    "1. No iteration is active (none is planned either), so it is in none. Plan it in the one that fits: `alm plan {id} <iteration>` (while one is active, new tasks go into it: the one marked active, or else the first planned one):\n{list}"
                ),
            };
            format!(
                "Next, for the agent who added it:
{step1}2. Check the task for anything that is the person's to decide first: data formats, removing things, permissions and security, anything that changes the plan or another task, anything with a cost or an outside effect (wording and look are not: whoever works it decides them, reviewed before release). {en_ready}
"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_the_setting_on_the_new_task_waits_for_the_approver_except_in_a_goal_run() {
        for language in ["en", "zh-CN", "ja"] {
            let off = added_task_steps(language, "A01-x1", &[], Some("I-1"), false);
            assert!(off.contains("`alm ready A01-x1`"), "{language}");
            assert!(!off.contains("alm new-task-settings"), "{language}");
            let on = added_task_steps(language, "A01-x1", &[], Some("I-1"), true);
            assert!(on.contains("alm new-task-settings"), "{language}");
            assert!(on.contains("alm guide goal"), "{language}");
            assert!(on.contains("`alm ready A01-x1`"), "{language}");
        }
        let on = added_task_steps("en", "A01-x1", &[], None, true);
        assert!(on.contains("Tell the approver its ID, A01-x1"));
        assert!(!on.contains("Nothing: make it ready"));
    }
}
