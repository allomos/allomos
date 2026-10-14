// Paths the interface works out itself (Task-A07-03). Rust gives paths with the system's own
// separator: `/` on macOS, `\` on Windows; both are accepted everywhere here.

/** The project folder the user picked, from a taskbag root (`<folder>/.allomos/taskbag`). */
export function projectFolder(root: string): string {
  return root.replace(/[\\/]\.allomos[\\/]taskbag[\\/]?$/, "");
}
