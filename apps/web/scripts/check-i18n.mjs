// Translation check (R94). Fails when a language file differs from English in its keys, has an
// empty text or different {placeholders}, or when Rust can send a problem code without words.
// Run: npm --prefix apps/web run check:i18n

import { readFileSync } from "node:fs";

const dir = new URL("../src/i18n/", import.meta.url);
const load = (name) => JSON.parse(readFileSync(new URL(name, dir), "utf8"));

const en = load("en.json");
const others = { "zh-CN": load("zh-CN.json"), ja: load("ja.json") };
const codes = load("codes.json"); // written by Rust: code -> names of its params

const problems = [];
const placeholders = (text) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(",");

for (const [key, text] of Object.entries(en)) {
  if (typeof text !== "string" || text.trim() === "") problems.push(`en: "${key}" is empty`);
}

for (const [lang, words] of Object.entries(others)) {
  for (const key of Object.keys(en)) {
    if (!(key in words)) problems.push(`${lang}: missing "${key}"`);
  }
  for (const [key, text] of Object.entries(words)) {
    if (!(key in en)) {
      problems.push(`${lang}: "${key}" is not in en.json`);
    } else if (typeof text !== "string" || text.trim() === "") {
      problems.push(`${lang}: "${key}" is empty`);
    } else if (placeholders(text) !== placeholders(en[key])) {
      problems.push(`${lang}: "${key}" uses {${placeholders(text)}} but English uses {${placeholders(en[key])}}`);
    }
  }
}

for (const [code, params] of Object.entries(codes)) {
  const key = `problem.${code}`;
  if (!(key in en)) {
    problems.push(`en: Rust code "${code}" has no words; add "${key}" to every language file`);
    continue;
  }
  for (const name of placeholders(en[key]).split(",").filter(Boolean)) {
    if (!params.includes(name)) problems.push(`en: "${key}" uses {${name}}, which Rust does not send`);
  }
}
for (const key of Object.keys(en)) {
  if (key.startsWith("problem.") && !(key.slice("problem.".length) in codes)) {
    problems.push(`en: "${key}" is not a Rust code (see codes.json)`);
  }
}

if (problems.length > 0) {
  console.error(`Translation check failed (${problems.length}):\n  ${problems.join("\n  ")}`);
  process.exit(1);
}
console.log(`Translation check passed: ${Object.keys(en).length} texts in en, zh-CN and ja; ${Object.keys(codes).length} Rust codes.`);
