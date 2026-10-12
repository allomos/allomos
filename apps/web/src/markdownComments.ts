// Hiding Markdown comments, `<!-- … -->`, before Allomos shows a text (Task-A03-11): the hints in
// the handoff template, and any in a task's description or an agent's plan. Kept apart from React
// so it can be tested with Node alone (`npm --prefix apps/web test`). Only type imports here.

/** Stands in for a removed comment until the lines are tidied. */
const MARK = "\u0000";

/** `source` without its comments. Code blocks and inline code keep theirs, as Markdown does; an
 * unclosed `<!--` is left as written rather than hiding the rest. A line that held only comments
 * is dropped, so it doesn't split a list or a paragraph. */
export function stripComments(source: string): string {
  if (!source.includes("<!--")) return source;
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const out: string[] = [];
  let prose: string[] = [];
  const flush = () => {
    if (prose.length > 0) out.push(...tidy(removeComments(prose.join("\n")).split("\n")));
    prose = [];
  };
  let inFence = false;
  for (const line of lines) {
    if (line.startsWith("```")) {
      if (!inFence) flush();
      inFence = !inFence;
      out.push(line);
    } else if (inFence) out.push(line);
    else prose.push(line);
  }
  flush();
  return out.join("\n");
}

/** Comments outside inline code replaced by MARK. Inline code ends at a line's end. */
function removeComments(text: string): string {
  let out = "";
  let inCode = false;
  let i = 0;
  while (i < text.length) {
    const c = text[i];
    if (c === "\n") inCode = false;
    else if (c === "`") inCode = !inCode;
    else if (!inCode && text.startsWith("<!--", i)) {
      const end = text.indexOf("-->", i + 4);
      if (end < 0) return out + text.slice(i);
      out += MARK;
      i = end + 3;
      continue;
    }
    out += c;
    i++;
  }
  return out;
}

/** Lines left empty by a removed comment go; elsewhere the mark and the spaces before it go. */
function tidy(lines: string[]): string[] {
  return lines
    .filter((l) => !(l.includes(MARK) && !l.split(MARK).join("").trim()))
    .map((l) => (l.includes(MARK) ? l.split(MARK).join("").replace(/\s+$/, "") : l));
}
