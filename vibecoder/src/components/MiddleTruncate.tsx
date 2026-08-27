/**
 * Text that loses its middle, not its end, when the space runs out.
 *
 * `text-overflow: ellipsis` only ever cuts the tail, which for a file path
 * removes the one part that identifies it — a column of
 * `docs/content/ai/AI_FEATURES_SUMMARY.md` and `docs/content/ai/AI_NOTES.md`
 * both collapse to `docs/content/ai/AI_FE…`. Keeping the last characters means
 * a truncated row is still recognisably the row you were looking for.
 *
 * Two spans rather than measuring the text: the head shrinks and ellipsises,
 * the tail never shrinks. That reflows at any container width with no
 * ResizeObserver, no font metrics, and nothing to recompute on a resize — which
 * matters in a panel the user drags.
 *
 * The full string is still in the DOM, split across the two spans, so a screen
 * reader reads the whole path and the ellipsis stays purely visual.
 */
export function MiddleTruncate({
  text,
  /** Characters kept on the right. Enough for an extension plus a little of
   *  what precedes it — the part that tells two long paths apart. */
  tail = 12,
  className,
  style,
}: {
  text: string;
  tail?: number;
  className?: string;
  style?: React.CSSProperties;
}) {
  // Short enough to fit whole, or too short to be worth splitting: one span,
  // no ellipsis machinery. Splitting a 10-character name into 0 + 10 would put
  // an ellipsis in front of text that was never truncated.
  const split = text.length > tail + 4 ? text.length - tail : text.length;
  const head = text.slice(0, split);
  const rest = text.slice(split);

  return (
    <span
      className={className}
      title={text}
      style={{ display: 'flex', minWidth: 0, overflow: 'hidden', ...style }}
    >
      <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
        {head}
      </span>
      {rest && <span style={{ flex: 'none', whiteSpace: 'pre' }}>{rest}</span>}
    </span>
  );
}
