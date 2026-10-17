// The waiting picture: the app icon's own shapes (approver, 2026-10-17). The icon is "two hops
// through one point": a ring is where a ball was, a solid ball is where it arrived, the gold spark
// is the jump. The loop builds the start and then plays the hops: the spark alone; the jade ring
// appears, then the terracotta ring; the jade ball fills its ring, then the terracotta ball; the
// jade ball flies through the spark (which flashes) and lands on the right; the terracotta ball
// flies through it from the top and lands at the bottom; the finished icon holds; then everything
// but the spark fades out and the loop starts again. Same colours and shapes as
// `apps/desktop/src-tauri/icons/source.svg`, animated in CSS (`styles.css`, `.icon-loader`). With
// reduced motion the picture is the icon, still. Decorative: the words beside it say what is
// happening.

/** Where the `.spinner` it replaces was, inline before the words; 24 px (approver: bigger). */
export function IconLoader({ size = 24, className }: { size?: number; className?: string }) {
  return (
    <svg
      className={className ? `icon-loader ${className}` : "icon-loader"}
      width={size}
      height={size}
      viewBox="140 140 744 744"
      aria-hidden="true"
      focusable="false"
    >
      <circle className="il-ring il-ring-jade" cx="272" cy="512" r="99" fill="none" stroke="#2A9D8F" strokeWidth="38" />
      <circle className="il-ring il-ring-terra" cx="512" cy="272" r="99" fill="none" stroke="#E76F51" strokeWidth="38" />
      <circle className="il-ball il-jade" cx="272" cy="512" r="118" fill="#2A9D8F" />
      <circle className="il-ball il-terra" cx="512" cy="272" r="118" fill="#E76F51" />
      <path
        className="il-spark"
        d="M512 412 L542 482 L612 512 L542 542 L512 612 L482 542 L412 512 L482 482 Z"
        fill="#E9C46A"
      />
    </svg>
  );
}
