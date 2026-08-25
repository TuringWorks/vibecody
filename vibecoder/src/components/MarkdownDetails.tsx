/**
 * `<details>` rendered as a real disclosure.
 *
 * Generated study material hides its answers behind `<details>`; nothing in
 * the markdown pipeline renders raw HTML, so the reader used to get the tags
 * and the answer, spoiled, in one go. `splitDetails` lifts the balanced blocks
 * out of the source and this renders them as native disclosures — closed until
 * clicked, keyboard-operable because they are the real element.
 *
 * How the markdown inside is rendered is the caller's business: each surface
 * passes its own `renderBlock`, so a disclosure looks like the document it
 * sits in rather than like this file's idea of one.
 */
import { Fragment, useMemo, useState, type ReactNode } from "react";
import { splitDetails } from "../lib/markdownHtml";
import "./MarkdownDetails.css";

interface MarkdownWithDetailsProps {
  source: string;
  /** Render a run of markdown as document blocks. */
  renderBlock: (markdown: string) => ReactNode;
  /** Render a summary label inline; defaults to `renderBlock`. */
  renderInline?: (markdown: string) => ReactNode;
}

/**
 * The open state lives in React rather than in the `open` attribute alone: a
 * parent re-render (the markdown editor re-renders on every keystroke) would
 * otherwise re-apply `open={false}` and snap a disclosure the reader opened
 * shut again.
 */
function Disclosure({
  summary,
  initiallyOpen,
  children,
}: {
  summary: ReactNode;
  initiallyOpen: boolean;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(initiallyOpen);
  return (
    <details
      className="md-details"
      open={open}
      onToggle={(e) => setOpen(e.currentTarget.open)}
    >
      <summary className="md-details__summary">{summary}</summary>
      <div className="md-details__body">{children}</div>
    </details>
  );
}

export function MarkdownWithDetails({
  source,
  renderBlock,
  renderInline = renderBlock,
}: MarkdownWithDetailsProps) {
  // Keyed by ordinal *among the disclosures*, not by position in the document:
  // typing a line above a disclosure shifts every index below it, and a changed
  // key remounts the element — which would shut an answer the reader had opened.
  const keyed = useMemo(() => {
    let disclosures = 0;
    return splitDetails(source).map((segment, i) => ({
      segment,
      key: segment.kind === "details" ? `details-${disclosures++}` : `markdown-${i}`,
    }));
  }, [source]);

  return (
    <>
      {keyed.map(({ segment, key }) =>
        segment.kind === "markdown" ? (
          <Fragment key={key}>{renderBlock(segment.text)}</Fragment>
        ) : (
          <Disclosure
            key={key}
            initiallyOpen={segment.open}
            summary={segment.summary.trim() ? renderInline(segment.summary) : "Details"}
          >
            <MarkdownWithDetails
              source={segment.body}
              renderBlock={renderBlock}
              renderInline={renderInline}
            />
          </Disclosure>
        ),
      )}
    </>
  );
}
