---
title: What Allomos is
description: Allomos keeps a project's tasks, progress and handoff notes in your own repository, so any AI agent or teammate can pick up the work.
sidebar:
  order: 1
---

When you build software with AI agents such as Claude Code and Codex, the work is spread over many sessions. A session can stop without warning when an account's quota runs out, and the next one starts knowing nothing. Handing work to a teammate has the same problem.

Allomos fixes this by keeping the project's state in one place that every agent and every person reads: the **taskbag**.

## What it does

- **Switching.** Hand the project from one AI agent or account to another, or to a teammate, and pick up exactly where it stopped.
- **Task management.** The whole vibe-coding process is on the record: every task and its status in one place, not scattered across chats. That makes the work easy to review by hand and easy to maintain, while you build and for years after.
- **AI skills** *(in development)*. Work with your agents in a way that gets better over time.
- **Progress management** *(in development)*. See where the project stands and when it will land.

## The taskbag

A taskbag is a folder, `.allomos/taskbag/`, inside your project. It holds plain text files:

- the **tasks**: what is planned and what each one should achieve;
- **who holds** each task: a person or an agent session;
- **how far** each task has got, as a label such as *implemented* or *locally verified*;
- what a task is **waiting on**;
- the **handoff notes** one session leaves for the next.

Because these are ordinary files in your repository, Git keeps their history, and they travel with the project wherever it goes.

## Who uses it, and how

| Who | How |
|---|---|
| You | The Allomos desktop app shows the tasks by area, who holds what, and the handoff notes. |
| AI agents | The `alm` command: `alm next` gives the next task and its handoff note, `alm claim` takes it, `alm release` hands it back with a note. |
| Teammates | They open the same repository. Everyone sees the same tasks. |

When one agent's quota runs out, it writes where it stopped. The next agent, on another account or from another company, asks `alm next` and carries on.

## Free on your computer

The desktop app is free and needs no account. Your tasks stay in your repository unless you choose to share them. A paid cloud option for teams is *planned*.
