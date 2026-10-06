# Road show deck / 路演幻灯片

A ten-slide pitch deck for FlipHopper, in English and Simplified Chinese, organized by the three sets of [0014](../decisions/0014-main-purpose-and-three-sets.md): the main set (switch between AI agents and teammates and keep the project), AI skills, and progress management; then pricing (the local app is free, people pay only for the cloud) and the roadmap. Made 2026-10-04 ([`Task-A00-16`](../tasks/Task-A00-16-road-show-deck.md)); renamed to FlipHopper and published at new addresses 2026-10-06 ([`Task-A00-17`](../tasks/Task-A00-17-rename-to-crossjump.md)), because the first addresses were not reachable from the publishing session's account. A new main message is planned in [`Task-A00-18`](../tasks/Task-A00-18-positioning-and-deck.md).

| Version | Published address (private claude.ai Artifact) | Source in this repo |
|---|---|---|
| English | https://claude.ai/artifact/MGmzN64hzvRqWAMmvt1qyU | [roadshow-en.html](./roadshow-en.html) |
| 中文 | https://claude.ai/artifact/XVe1UxXRUVt7WytcDgcABU | [roadshow-zh.html](./roadshow-zh.html) |

## Using it

- Open an address in any browser. Click **Present ▸** (中文版：**演示 ▸**) or press **P** for full-screen slides. Use the arrow keys or swipe to move, and **Esc** to exit.
- The pages are **private**: only the owner can open them until they are shared through the page's **Share** menu.

## Updating it

1. Edit the HTML file here, and keep both language versions in step.
2. Republish to the **same address**: call the Artifact tool's publish action with this file and pass the address above as `url`. Publishing without `url` creates a new address. Then update this table.
3. Commit and push the source change.

## Rules for the content

- **Claim only what is decided.** The cloud price says "to be announced". Free phone access is promised **at home** only, until remote access is decided ([0012](../decisions/0012-free-local-app-paid-cloud.md)). The product is marked as early development. Set 2 (AI skills) is marked as being designed until its contents are decided ([0014](../decisions/0014-main-purpose-and-three-sets.md)).
- **Charts are drawn to scale from their own numbers.** The burndown example uses weekly throughput 3, 5, 3, 6, 5, 4 with 34 tasks left. A 10,000-run simulation gives a 50% chance of finishing by week 14 and an 85% chance by week 15. If the example data changes, recompute these weeks.
- **Forecast labels state the chance together with the date**, never a bare percentage ([PLAN § 7](../product/PLAN.md#7-burndown-and-forecast)).
