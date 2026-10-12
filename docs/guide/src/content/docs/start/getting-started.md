---
title: Getting started
description: Open Allomos, create a taskbag for your project, and connect your AI agents.
sidebar:
  order: 2
---

This page takes you from a first start to an AI agent working from your taskbag. It takes about five minutes.

## 1. Get the app

Allomos 0.1 runs on macOS; Windows is coming in 0.2. There is no download yet: downloads will appear on the [releases page](https://github.com/mypals-ml/allomos/releases). Until then, the app is run from its source code ([instructions for developers](https://github.com/mypals-ml/allomos/blob/main/docs/workflow/CODING_RULES.md#running-the-app)).

## 2. Start

When Allomos opens with no project, it shows one button, **Start**. Press it to open the setup wizard: four short steps, numbered "Step 1 of 4" and so on.

## 3. Create the taskbag

1. **What is a taskbag?** The first step explains it in a few lines. Press **Choose a folder…** and choose your project's folder. The taskbag is created inside it as `.allomos/`.
2. **The taskbag folder.** If the folder has no taskbag yet, press **Create taskbag**. If it already has one, Allomos says so and uses it. Then press **Next**.

Allomos keeps the taskbag in Git, so its history is saved. If the folder is not in a Git repository yet, Allomos makes it one. It saves (commits) only the taskbag's own files, and it never sends anything to the internet.

## 4. Choose the project folder

The project folder holds your code and the instruction files AI agents read when a session starts (`AGENTS.md` and `CLAUDE.md`). It is usually the same folder you chose in step 3. If not, press **Choose another folder…**. Then press **Next**.

## 5. Connect with the agents

Choose one AI agent and press **Connect**. Allomos opens that agent in your project folder with a request already typed in. **Nothing is sent until you send it yourself.**

The request asks the agent to:

- add a short section about Allomos to `AGENTS.md` and `CLAUDE.md`, so every new session knows how to use the taskbag;
- look for tasks the project already keeps, such as a task board or a to-do list.

If the agent finds tasks, it shows its plan in Allomos. Choose **Go ahead** to move them into the taskbag, or **Cancel** to leave them where they are.

You can skip this step with **Not now**, and do it later with **File › Connect with the Agents…**.

:::note
Agent sessions that were already running read `AGENTS.md` and `CLAUDE.md` only when they start. Start a new session so the agent knows about Allomos.
:::

## 6. Work

In a new agent session, ask the agent to start with `alm next`. It gets the next ready task and the last handoff note, claims it, and reports its progress in the taskbag. You follow the same tasks in the Allomos window.
