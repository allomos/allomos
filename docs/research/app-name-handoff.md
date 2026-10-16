# Research — renaming FlipHopper to "Handoff" or something like it

**Evidence** for task `A00-20`. Checked by the agent on 2026-10-10. Not a legal trademark search.

## In plain words

**"Handoff" itself is a poor choice.** Three things count against it:
- **It is a macOS feature.** Apple's Handoff is built into every Mac, iPhone and iPad (System Settings → General → AirDrop & Handoff), and the Mac is our first platform. Searching "Handoff Mac app" finds Apple, not us.
- **It is already the everyday word for what we do.** OpenAI's Agents SDK calls passing work between agents "handoffs", and at least five small tools that move work between Claude Code and Codex already call themselves handoff something.
- **Every name and address is taken:** the `.com`, `.io`, `.dev` and `.app` domains, the crates.io and npm packages, and the GitHub account. An App Store app is already called "Handoff".

The same holds for the plain words around it: **handover, batonpass, passdown, tagteam**.

The word "handoff" is better used to **describe** the product than to **name** it. If you do want a new name in this family, coined names are free:

| Name | crates.io / npm / GitHub | `.io` / `.dev` / `.app` | `.com` | Products found |
|---|---|---|---|---|
| **Batonhop** | free / free / free | free / free / free | registered 2025-12-26 | none |
| **Handhop** | free / free / free | free / free / free | registered 2017 | none |
| **HandoffHub** | free / free / free | free / free / free | registered | none named so, but searches land on the many "Handoff" products |
| Handoffly | free / free / free | free / free / taken | registered | none found |
| Relayhop, Passhop | free / free / taken | free / free / free | registered | none |

**Recommendation:**
1. **Keep FlipHopper,** and use "handoff" in the line that says what it does: *"FlipHopper: handoffs between AI agents and teammates, without losing the plan."* This costs nothing and keeps a name that is free everywhere except as a `.com`.
2. **If you want "handoff" in the name itself: Batonhop.** A *baton* is what relay runners hand off. *Hop* keeps the link to FlipHopper's picture (hopping between agents). Everything we would use is free except the `.com`. In Chinese, something with 接力 (relay) fits, for example 接力跳; in Japanese, バトンホップ.

## What a rename would touch

A product rename is bigger than the taskcase rename, and it is done once. It covers:
- the name in every screen and document;
- the `fliphopper` / `fh` command;
- the hidden folder `.fliphopper/` in every project, and every `AGENTS.md` that mentions it;
- the app ID `io.fliphopper`;
- the format tag `fliphopper/1`;
- the GitHub repository `mypals-ml/fliphopper` (an external operation that needs approval);
- the local folder;
- the decks.

This is why the taskcase rename (task `A01-20`) waits for this answer: if the app's name changes, `.fliphopper/taskcase/` would change again, and every project would move twice.

## Evidence

Registry checks (HTTP status: 200 = taken, 404 = free; crates.io, npm and GitHub APIs; domains by each registry's RDAP service, checked against controls such as `fliphopper.com` (free) and `google.com` (taken)):

| Name | crates.io | npm | GitHub | .com | .io | .dev | .app |
|---|---|---|---|---|---|---|---|
| handoff | taken | taken | taken | taken | taken | taken | taken |
| handover | free | taken | taken | taken | taken | taken | taken |
| batonpass | free | free | free | taken | taken | taken | taken |
| baton | taken | taken | taken | — | — | — | — |
| tagteam | free | taken | taken | taken | taken | taken | taken |
| passdown | free | free | taken | taken | taken | taken | taken |
| handoffs | free | taken | free | taken | taken | taken | taken |
| handon | free | free | taken | taken | taken | free | free |
| handova | free | free | taken | taken | taken | free | taken |
| handoffly | free | free | free | taken | free | free | taken |
| handoffhub | free | free | free | taken | free | free | free |
| handoffkit | taken | free | free | taken | free | free | free |
| handhop | free | free | free | taken (2017) | free | free | free |
| batonhop | free | free | free | taken (2025-12-26) | free | free | free |
| relayhop | free | free | taken | taken | free | free | free |
| passhop | free | free | taken | taken | free | free | free |
| shiftover | free | free | taken | taken | free | free | free |

Products and uses found by web search:
- **Apple Handoff**, part of Continuity on macOS, iOS and iPadOS since 2014 ([Apple support](https://support.apple.com/en-gb/102426), [Computerworld](https://www.computerworld.com/article/1392191/devs-can-handoff-ios-apps-to-browser-based-uis-on-macs.html)).
- **"Handoffs" in OpenAI's Agents SDK**, its word for one agent passing control to another ([guide](https://claw.aguidetocloud.com/openai/agents-sdk/concepts/), [DEV Community](https://dev.to/aicomag/implementing-handoff-protocols-in-openai-agents-sdk-for-retail-je)).
- **Tools in our own space:**
  - `handoff-cli` on PyPI (dazuiba/handoff), which sends coding tasks from Claude Code to other agents ([PyPI](https://pypi.org/project/handoff-cli/4.0.0/));
  - **ai-handoff**, which carries Claude Code conversations into Codex with an AGENTS.md ([LinkedIn](https://il.linkedin.com/in/tamirnauman));
  - **catchup**, which hands session context between coding agents ([gitblind](https://gitblind.noratr.app/wilbeibi/catchup));
  - **showagent** ([glama](https://glama.ai/mcp/servers/aytzey/showagent));
  - **devin-handoff** ([ecosyste.ms](https://awesome.ecosyste.ms/projects/github.com%2Fclub-cog%2Fdevin-handoff));
  - "handoff" skills for Claude Code ([tessl](https://tessl.io/registry/skills/github/EpicenterHQ/epicenter/handoff), [skillselion](https://skillselion.com/skills/softaworks/agent-toolkit/dist/plugins/session-handoff)).
- **Apps named Handoff:** the contractor-estimates app at handoff.ai on the App Store and Google Play ([App Store](https://apps.apple.com/app/id6470666320), [help](https://help.handoff.ai/en/articles/9564137-is-handoff-available-on-the-app-stores)); and an older Mac-to-iOS link sender ([MacStories](https://www.macstories.net/reviews/handoff-pushes-web-pages-from-your-computer-to-any-ios-device/)).
- **Batonhop, Relayhop, Handhop:** no product found ([search](https://apps.apple.com/tj/app/bhop-multiplayer-game/id6444131055?mt=12) found only unrelated "bhop" games).

**Limits:** registries and web search only, US results. No trademark database was searched directly. Before selling under a new name, a proper search in Japan, China and the US is advised (as in task `A00-17`).

## Addendum 2026-10-10: "…Hub" names

The owner liked **HandoffHub** and asked for other "Hub" names. "Hub" also matches how FlipHopper is built: the desktop app is the **hub** that the web and phone companions connect to ([0009](../decisions/0009-desktop-hub-with-web-and-mobile-companions.md)).

| Name | What it says | crates / npm / GitHub | .com / .io / .dev / .app | Products found |
|---|---|---|---|---|
| **HandoffHub** | where work is handed off | free / free / free | taken / free / free / free | none by this name; "Handoff" searches are crowded |
| **HandoverHub** | the same, in British/Asian English ("handover") | free / free / free | taken / free / free / free | none; a "Handover" reporting product exists ([Capterra](https://www.capterra.ie/software/1094908/Handover)) |
| **BatonHub** | where the relay baton is passed | free / free / free | taken / free / free / free | none; SofaBaton's remote "hub" is nearby in searches ([SofaBaton](https://www.sofabaton.com/?p=250)) |
| **HopperHub** | keeps FlipHopper's "hopper" | free / free / free | taken / free / free / free | none checked beyond registries |
| **CarryOnHub** | carry on where the last one stopped | free / free / free | taken / free / free / free | none checked beyond registries |
| **TagTeamHub** | partners tagging in and out | free / free / free | **free** / free / free / free | none checked beyond registries |
| **BatonPassHub** | the pass of the baton | free / free / free | **free** / free / free / free | long |
| SwapHub, HandHub | swap / hand over | free / free / taken | taken / free / free / taken | — |
| RelayHub, HopHub, SwitchHub, PassHub, ShiftHub, CrewHub, AgentHub, PickupHub | — | GitHub taken | all four domains taken | — |

### Task and project "…Hub" names (2026-10-10)

The owner asked about **TaskHub, ProjectHub** and similar. All of these have their GitHub account taken; the plain ones have every domain taken too, and both TaskHub and ProjectHub are already used by several to-do and project apps ([TaskHub on the App Store](https://apps.apple.com/ec/app/taskhub-plan-your-tasks/id6740157355), [TaskHub on quickhub.ai](https://www.quickhub.ai/feature), [TaskHub, Bubble](https://forum.bubble.io/t/taskhub-project-management-app/353304), [Taskhub on GitHub](https://github.com/zhangxijing97/Taskhub), [ProjectHub on GitHub](https://github.com/Shinagawa-monkey/ProjectHub-App)).

| Name | crates / npm / GitHub | .com / .io / .dev / .app |
|---|---|---|
| TaskHub, ProjectHub, PlanHub, CardHub, TrackHub, SprintHub | free / free / taken | all taken |
| WorkHub | free / taken / taken | all taken |
| BoardHub | free / free / taken | taken / taken / free / taken |
| TodoHub, ProgressHub, MilestoneHub | free / free / taken | taken / free / free / taken |
| BacklogHub, OutcomeHub | free / free / taken | taken / free / free / free |

They also name what every task tracker does, not what FlipHopper does differently: moving a project between AI agents and teammates without losing its plan ([0014](../decisions/0014-main-purpose-and-three-sets.md)).

### Allomos trademark search (2026-10-10)

See `Task-A00-21` Notes: no ALLOMOS, 换肩, 換肩 or アロモス mark in WIPO's Global Brand Database (US, Japan, EU, UK, Madrid and others; China's national register is not in it and needs a CNIPA account). Similar software marks: ALLMOS (US, class 9), ALOMOS (SG/MY/ID, class 9), ALTOMOS (Japan, classes 9 and 42), allmos electronic (DE/EU, class 9). Source: [WIPO Global Brand Database](https://branddb.wipo.int/en/similarname), [coverage](https://branddb.wipo.int/en/coverage).
