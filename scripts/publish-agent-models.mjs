#!/usr/bin/env node
// Publish the list of supported agents, models and reasoning efforts (task A03-coral174).
// The source is data/agent-models.json; this script checks it and writes it to
// site/agents/models.json, served at https://www.allomos.com/agents/models.json.
// It only changes local files: going online is a push to main (Cloudflare Pages publishes
// site/), which needs the approver's approval each time. Run with no arguments:
//   node scripts/publish-agent-models.mjs
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

export const FORMAT_VERSION = 1;
const ID = /^[a-z][a-z0-9_]*$/;
const DATE = /^\d{4}-\d{2}-\d{2}$/;

/** Every problem in the list, as plain sentences; an empty array means it is fine. */
export function check(data) {
  const problems = [];
  const isObject = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const text = (v) => typeof v === "string" && v.trim() !== "";
  if (!isObject(data)) return ["the list must be a JSON object"];
  const known = new Set(["version", "updated", "agents"]);
  for (const key of Object.keys(data)) if (!known.has(key)) problems.push(`unknown field "${key}"`);
  if (data.version !== FORMAT_VERSION) problems.push(`"version" must be ${FORMAT_VERSION}`);
  if (!(typeof data.updated === "string" && DATE.test(data.updated) &&
        !Number.isNaN(Date.parse(`${data.updated}T00:00:00Z`)) &&
        new Date(`${data.updated}T00:00:00Z`).toISOString().startsWith(data.updated))) {
    problems.push(`"updated" must be a date such as 2026-11-17`);
  }
  if (!Array.isArray(data.agents) || data.agents.length === 0) {
    problems.push(`"agents" must list at least one agent`);
    return problems;
  }
  const ids = new Set();
  data.agents.forEach((agent, i) => {
    const where = `agent ${i + 1}`;
    if (!isObject(agent)) { problems.push(`${where} must be an object`); return; }
    const agentKnown = new Set(["id", "name", "models", "efforts"]);
    for (const key of Object.keys(agent)) if (!agentKnown.has(key)) problems.push(`${where}: unknown field "${key}"`);
    const name = typeof agent.id === "string" ? `agent "${agent.id}"` : where;
    if (!(typeof agent.id === "string" && ID.test(agent.id))) {
      problems.push(`${where}: "id" must be lower case letters, digits and _ (such as claude_cli)`);
    } else if (ids.has(agent.id)) {
      problems.push(`${name} is listed twice`);
    } else ids.add(agent.id);
    if (!text(agent.name)) problems.push(`${name}: "name" must not be empty`);
    if (!Array.isArray(agent.models) || agent.models.length === 0) {
      problems.push(`${name}: "models" must list at least one model`);
    } else {
      const values = new Set();
      agent.models.forEach((m, j) => {
        if (!isObject(m) || !text(m.value) || !text(m.label) || Object.keys(m).length !== 2) {
          problems.push(`${name}: model ${j + 1} must have exactly a "value" and a "label", both not empty`);
        } else if (values.has(m.value)) {
          problems.push(`${name}: model "${m.value}" is listed twice`);
        } else values.add(m.value);
      });
    }
    if (!Array.isArray(agent.efforts) || agent.efforts.length === 0) {
      problems.push(`${name}: "efforts" must list at least one reasoning effort`);
    } else {
      const seen = new Set();
      agent.efforts.forEach((e, j) => {
        if (!text(e)) problems.push(`${name}: effort ${j + 1} must be a word such as medium`);
        else if (seen.has(e)) problems.push(`${name}: effort "${e}" is listed twice`);
        else seen.add(e);
      });
    }
  });
  return problems;
}

/** The published file's text: the checked data, in one fixed layout. */
export function render(data) {
  return JSON.stringify(data, null, 2) + "\n";
}

/**
 * Check `source` and write it to `target`. Returns { changed, problems }; nothing is written
 * when there are problems or when the target already holds the same text.
 */
export function publish(source, target) {
  let data;
  try {
    data = JSON.parse(readFileSync(source, "utf8"));
  } catch (err) {
    return { changed: false, problems: [`cannot read ${source}: ${err.message}`] };
  }
  const problems = check(data);
  if (problems.length) return { changed: false, problems };
  const out = render(data);
  if (existsSync(target) && readFileSync(target, "utf8") === out) return { changed: false, problems };
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, out);
  return { changed: true, problems };
}

function main() {
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const source = join(root, "data", "agent-models.json");
  const target = join(root, "site", "agents", "models.json");
  const shown = (p) => relative(process.cwd(), p) || p;
  const { changed, problems } = publish(source, target);
  if (problems.length) {
    console.error(`${shown(source)} has ${problems.length} problem(s); nothing was written:`);
    for (const p of problems) console.error(`  - ${p}`);
    process.exit(1);
  }
  if (!changed) {
    console.log(`${shown(target)} is up to date with ${shown(source)}; nothing changed.`);
    return;
  }
  console.log(`Wrote ${shown(target)} from ${shown(source)}.`);
  console.log("Next, to publish it at https://www.allomos.com/agents/models.json:");
  console.log("  1. Ask the approver to approve this deployment (every push to main publishes site/).");
  console.log(`  2. With approval: git add -- ${shown(source)} ${shown(target)}, commit, and push to main.`);
  console.log("  3. After Cloudflare Pages has published it, open the address and check the list.");
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) main();
