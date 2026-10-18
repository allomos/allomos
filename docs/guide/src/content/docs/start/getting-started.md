---
title: Getting started
description: Open Allomos, create a taskbag for your project, and connect your AI agents.
sidebar:
  order: 2
---

This page takes you from a first start to an AI agent working from your taskbag. It takes about five minutes.

## 1. Get the app

Download the newest version from the [releases page](https://github.com/allomos/allomos/releases/latest):

- **macOS** (Apple Silicon Macs: M1 or newer): `Allomos_<version>_aarch64.pkg`. Open it and follow the installer; it asks for your Mac password once, puts Allomos in Applications and installs the `alm` command too. (Or `Allomos_<version>_aarch64.dmg`: open it and drag Allomos into Applications; Allomos then offers **Install the alm command**.)
- **Windows**: `Allomos_<version>_x64-setup.exe`. Run it; it needs no administrator rights, and it installs the `alm` command too.

Until version 1.0 these downloads are not signed, so the first time you open Allomos the system warns you:

- **macOS** says it cannot check Allomos (or its installer) for malware. Choose **Done**, open **System Settings › Privacy & Security**, scroll down and choose **Open Anyway** next to Allomos.
- **Windows** shows "Windows protected your PC". Choose **More info**, then **Run anyway**.

On macOS, `alm` is a link to the copy inside Allomos (`/usr/local/bin/alm`), so updating Allomos updates `alm` too. Most people want the installer above. The `alm-only` files hold the `alm` command alone, for a computer without the app; they are not installers:

- **macOS**: `alm-only_<version>_macos_arm64.tar.gz`. Open it, move `alm` into a folder on your PATH, such as `/usr/local/bin`, then run `alm` from a terminal.
- **Windows**: `alm-only_<version>_windows_x64.zip`. Unpack it, put `alm.exe` in a folder on your PATH, then run `alm` from a terminal (double-clicking it only flashes a window).

## 2. Start

When Allomos opens with no project, it shows one button, **Start**. Press it to open the setup wizard: four short steps, numbered "Step 1 of 4" and so on.

## 3. Create the taskbag

1. **What is a taskbag?** The first step explains it in a few lines. Press **Choose a folder…** and choose your project's folder. The taskbag is created inside it as `.allomos/`.
2. **The taskbag folder.** If the folder has no taskbag yet, press **Create taskbag**. If it already has one, Allomos says so and uses it. Then press **Next**.

Allomos itself never runs Git. The AI agent you connect in the last step keeps the taskbag in Git, so its history is saved: if the folder is not in a Git repository yet, the agent asks you whether to make it one, then commits only the taskbag's own files. Nothing is sent to the internet unless you say so.

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
