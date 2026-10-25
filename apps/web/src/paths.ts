// Paths the interface works out itself (Task-A07-03). Rust gives paths with the system's own
// separator: `/` on macOS, `\` on Windows; both are accepted everywhere here.

/** The project folder the user picked, from a taskbag root (`<folder>/.allomos/taskbag`). */
export function projectFolder(root: string): string {
  return root.replace(/[\\/]\.allomos[\\/]taskbag[\\/]?$/, "");
}

/** `path` shortened in the middle to at most `max` characters, keeping its start and its last
 * folders (`/Users/me/…/project/.allomos/taskbag`), for the taskbag settings' location
 * (Task-A05-camera758). The full path stays in the tooltip. */
export function middleEllipsis(path: string, max: number): string {
  const chars = [...path];
  if (chars.length <= max) return path;
  const keep = Math.max(2, max - 1);
  const tail = Math.ceil(keep * 0.6);
  return chars.slice(0, keep - tail).join("") + "…" + chars.slice(chars.length - tail).join("");
}
