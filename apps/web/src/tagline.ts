// The slogan split into phrases for the About window (Task-A01-aunt631).
/** The slogan in two phrases: up to and including its first comma or colon, and the rest. */
export function taglinePhrases(tagline: string): string[] {
  const match = /^(.*?[，、:])\s*(.+)$/u.exec(tagline);
  return match ? [match[1], match[2]] : [tagline];
}
