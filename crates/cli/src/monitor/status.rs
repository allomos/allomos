//! Text shared by monitor status and list.
use super::*;

/// One run as a line of `status` and `list`.
pub(super) fn entry_text(e: &Entry, with_project: bool) -> String {
    match e {
        Entry::Missing { pid, id, .. } => format!(
            "{}  running (record missing)  process {pid}: an `alm monitor run` with no run file; `alm monitor stop --run {pid}` ends it\n",
            id.as_deref().unwrap_or("(no ID)")
        ),
        Entry::Running(r) => {
            let run = &r.run;
            let scope = match &run.scope {
                Scope::All => "every ready task".to_string(),
                Scope::Iteration { id } => format!("iteration {id}"),
            };
            let agents = agents_text(run);
            let working: Vec<String> = run
                .working
                .iter()
                .map(|w| {
                    format!(
                        "{} ({}{})",
                        w.id,
                        agent_word(&w.agent),
                        if w.seen_held { "" } else { ", not claimed yet" }
                    )
                })
                .collect();
            let state = if run.stopping {
                "stopping: finishing the tasks at work".to_string()
            } else if run.free_agents().is_empty() && run.paused.is_empty() {
                "every agent at work".to_string()
            } else if run.free_agents().is_empty() {
                "no agent free now".to_string()
            } else {
                format!(
                    "next check {}",
                    runner::local_hhmm(run.next_check().max(r.alive))
                )
            };
            let mut text = format!(
                "{}  running  process {}{}\n  tasks: {scope}; agents: {}; every {} min; {} mode\n  working: {}; {state}\n",
                r.id,
                r.pid,
                if with_project {
                    format!("  {}", r.taskbag.display())
                } else {
                    String::new()
                },
                agents.join(", "),
                run.every,
                if run.quiet { "quiet" } else { "normal" },
                if working.is_empty() {
                    "none".to_string()
                } else {
                    working.join(", ")
                },
            );
            for w in run.stalled(runner::now_ms()) {
                text.push_str(&format!("  {}: task lost; it may not finish automatically. To exit monitoring only: alm monitor stop --run {} --now\n", w.id, r.id));
            }
            if !run.no_task.is_empty() {
                let words: Vec<&str> = run.no_task.iter().map(|a| agent_word(a)).collect();
                text.push_str(&format!(
                    "  no ready task in the levels of: {} (no session starts for them)\n",
                    words.join(", ")
                ));
            }
            for l in &run.let_go {
                text.push_str(&format!(
                    "  let go: {} was claimed by {} (another session); its slot is free\n",
                    l.id, l.holder
                ));
            }
            // A03-otter944: agents given no session for now, and sessions that did not start.
            for (agent, p) in &run.paused {
                let why = match p.why {
                    allomos_monitor::flow::PauseWhy::Quota => format!(
                        "quota used up{}; read again at {}",
                        p.resets
                            .as_deref()
                            .map(|r| format!(", back at {r}"))
                            .unwrap_or_default(),
                        runner::local_hhmm(p.until)
                    ),
                    allomos_monitor::flow::PauseWhy::NoClaim => format!(
                        "its last session did not start; tried again at {}",
                        runner::local_hhmm(p.until)
                    ),
                };
                text.push_str(&format!("  paused: {} ({why})\n", agent_word(agent)));
            }
            if !run.not_started.is_empty() {
                let list: Vec<String> = run
                    .not_started
                    .iter()
                    .map(|n| {
                        format!(
                            "{} ({}, {} at {})",
                            n.id,
                            agent_word(&n.agent),
                            match n.why {
                                allomos_monitor::flow::PauseWhy::Quota => "quota used up",
                                allomos_monitor::flow::PauseWhy::NoClaim => "never claimed",
                            },
                            runner::local_hhmm(n.found)
                        )
                    })
                    .collect();
                text.push_str(&format!(
                    "  did not start (given out again): {}\n",
                    list.join(", ")
                ));
            }
            if !run.taken.is_empty() {
                text.push_str(&format!(
                    "  sessions started for: {}\n",
                    run.taken.join(", ")
                ));
            }
            text
        }
    }
}
