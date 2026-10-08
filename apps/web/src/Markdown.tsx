// A small Markdown reader for task descriptions (Task-A01-16): headings, paragraphs, lists,
// code blocks, and inline code, bold, italics and links. It builds React elements, never HTML
// text, so a description cannot inject markup. Links are handed to `onLink`, which asks the
// desktop app to open them; the app decides what may be opened.

import type { ReactNode } from "react";

type Block =
  | { kind: "heading"; level: number; text: string }
  | { kind: "para"; text: string }
  | { kind: "list"; ordered: boolean; items: string[] }
  | { kind: "code"; text: string };

function blocks(source: string): Block[] {
  const out: Block[] = [];
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (!line.trim()) {
      i++;
    } else if (line.startsWith("```")) {
      const code: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) code.push(lines[i++]);
      i++;
      out.push({ kind: "code", text: code.join("\n") });
    } else if (/^#{1,6}\s/.test(line)) {
      const level = line.match(/^#+/)![0].length;
      out.push({ kind: "heading", level, text: line.slice(level).trim() });
      i++;
    } else if (/^\s*([-*+]|\d+[.)])\s+/.test(line)) {
      const ordered = /^\s*\d/.test(line);
      const items: string[] = [];
      while (i < lines.length && /^\s*([-*+]|\d+[.)])\s+/.test(lines[i])) {
        let item = lines[i++].replace(/^\s*([-*+]|\d+[.)])\s+/, "");
        // Indented lines continue the item.
        while (i < lines.length && /^\s{2,}\S/.test(lines[i]) && !/^\s*([-*+]|\d+[.)])\s+/.test(lines[i])) {
          item += " " + lines[i++].trim();
        }
        items.push(item);
      }
      out.push({ kind: "list", ordered, items });
    } else {
      const para: string[] = [];
      while (
        i < lines.length &&
        lines[i].trim() &&
        !lines[i].startsWith("```") &&
        !/^#{1,6}\s/.test(lines[i]) &&
        !/^\s*([-*+]|\d+[.)])\s+/.test(lines[i])
      ) {
        para.push(lines[i++].trim());
      }
      out.push({ kind: "para", text: para.join(" ") });
    }
  }
  return out;
}

/** Inline code, links, bold and italics, in that order of precedence. */
const INLINE = /`([^`]+)`|\[([^\]]+)\]\(([^)\s]+)\)|\*\*([^*]+)\*\*|\*([^*]+)\*|_([^_]+)_/g;

function inline(text: string, onLink: (href: string) => void, key: string): ReactNode[] {
  const out: ReactNode[] = [];
  let last = 0;
  let n = 0;
  for (const m of text.matchAll(INLINE)) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const k = `${key}-${n++}`;
    if (m[1] !== undefined) out.push(<code key={k}>{m[1]}</code>);
    else if (m[2] !== undefined) {
      const href = m[3];
      out.push(
        <a
          key={k}
          href={href}
          title={href}
          onClick={(e) => {
            e.preventDefault();
            onLink(href);
          }}
        >
          {inline(m[2], onLink, k)}
        </a>,
      );
    } else if (m[4] !== undefined) out.push(<strong key={k}>{inline(m[4], onLink, k)}</strong>);
    else out.push(<em key={k}>{m[5] ?? m[6]}</em>);
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

export function Markdown({ source, onLink }: { source: string; onLink: (href: string) => void }) {
  return (
    <div className="markdown">
      {blocks(source).map((b, i) => {
        const k = String(i);
        switch (b.kind) {
          case "heading": {
            // Inside the pane, a description's headings sit below the pane's own title.
            const Tag = (["h3", "h4", "h5", "h6", "h6", "h6"] as const)[b.level - 1];
            return <Tag key={k}>{inline(b.text, onLink, k)}</Tag>;
          }
          case "list": {
            const Tag = b.ordered ? "ol" : "ul";
            return (
              <Tag key={k}>
                {b.items.map((item, j) => (
                  <li key={j}>{inline(item, onLink, `${k}-${j}`)}</li>
                ))}
              </Tag>
            );
          }
          case "code":
            return (
              <pre key={k}>
                <code>{b.text}</code>
              </pre>
            );
          default:
            return <p key={k}>{inline(b.text, onLink, k)}</p>;
        }
      })}
    </div>
  );
}
