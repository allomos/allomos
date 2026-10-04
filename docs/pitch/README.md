# Road show deck / 路演幻灯片

A nine-slide pitch deck for glidebox, in English and Simplified Chinese. It covers four messages: manage your project while vibe coding; work anytime; teammates cooperate; the local app is free, and people pay only for the cloud. Made 2026-10-04 ([`Task-A00-16`](../tasks/Task-A00-16-road-show-deck.md)).

| Version | Published address (private claude.ai Artifact) | Source in this repo |
|---|---|---|
| English | https://claude.ai/artifact/7vW8C5xNAqJGW9sLf26hAW | [roadshow-en.html](./roadshow-en.html) |
| 中文 | https://claude.ai/artifact/XWsYFzucFMs2vQqNtixwV8 | [roadshow-zh.html](./roadshow-zh.html) |

## Using it

- Open an address in any browser. Click **Present ▸** (中文版：**演示 ▸**) or press **P** for full-screen slides. Use the arrow keys or swipe to move, and **Esc** to exit.
- The pages are **private**: only the owner can open them until they are shared through the page's **Share** menu.

## Updating it

1. Edit the HTML file here, and keep both language versions in step.
2. Republish to the **same address**: call the Artifact tool's publish action with this file and pass the address above as `url`. Publishing without `url` creates a new address. Then update this table.
3. Commit and push the source change.

## Rules for the content

- **Claim only what is decided.** The cloud price says "to be announced". Free phone access is promised **at home** only, until remote access is decided ([0012](../decisions/0012-free-local-app-paid-cloud.md)). The product is marked as early development.
- **Charts are drawn to scale from their own numbers.** The burndown example uses weekly throughput 3, 5, 3, 6, 5, 4 with 34 tasks left. A 10,000-run simulation gives a 50% chance of finishing by week 14 and an 85% chance by week 15. If the example data changes, recompute these weeks.
- **Forecast labels state the chance together with the date**, never a bare percentage ([PLAN § 7](../product/PLAN.md#7-burndown-and-forecast)).
