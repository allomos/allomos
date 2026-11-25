//! What `alm add task` prints after its Git commands (Task-A03-33, Task-A01-root343): the next
//! steps for the agent who added the task, in the task language. Moved out of `guide.rs`
//! (Task-A00-wheat560) to keep that file under its line limit.

/// What `alm add task` prints after its Git commands (Task-A03-33), in the task language
/// (`en`, `zh-CN`, `ja`; English otherwise): check the iteration it went into (`planned_in`, the
/// active one or the one asked for, Task-A01-root343), or plan it in one from `iterations` (one
/// line each, as `alm list` heads them) when it went into none, then make it ready unless
/// something in it is the person's to decide. alm tells; the agent decides.
pub fn added_task_steps(
    language: &str,
    id: &str,
    iterations: &[String],
    planned_in: Option<&str>,
) -> String {
    let listed: String = iterations
        .iter()
        .flat_map(|i| ["     ", i.as_str(), "\n"])
        .collect();
    match language {
        "zh-CN" => {
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
{step1}2. 检查任务里有没有该由人先定的事：数据格式、删掉东西、权限和安全、改变计划或其他任务的事、有花费或对外影响的事（用词和样子不算：做的人决定，发布前审）。没有：把它设为就绪，`alm ready {id}`。有：留在储备，在描述的备注里写明每个待定问题和你的提案，并告诉那个人。就绪的任务不带未决问题。
"
            )
        }
        "ja" => {
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
{step1}2. 人が先に決めるべきことがタスクにないか確かめます：データの形式、何かを削除すること、権限とセキュリティ、計画やほかのタスクを変えること、費用や外への影響があること（言葉や見た目は含みません：作る人が決め、リリース前に見直します）。なければ準備完了にします：`alm ready {id}`。あれば backlog のまま残し、説明の備考に未決の問いとあなたの案を一つずつ書いて、その人に伝えます。準備完了のタスクは未決の問いを持ちません。
"
            )
        }
        _ => {
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
{step1}2. Check the task for anything that is the person's to decide first: data formats, removing things, permissions and security, anything that changes the plan or another task, anything with a cost or an outside effect (wording and look are not: whoever works it decides them, reviewed before release). Nothing: make it ready, `alm ready {id}`. Something: leave it in backlog, name each open question with your proposal in the description's Notes, and tell the person. A ready task carries no open questions.
"
            )
        }
    }
}
