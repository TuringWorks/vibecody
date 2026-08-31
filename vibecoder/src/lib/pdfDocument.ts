/**
 * pdfDocument — the PDF renderer, behind one small interface.
 *
 * The viewer used to hand the file to an `<iframe>` and let the platform draw
 * it. That works, and it is the reason the viewer had no page navigation, no
 * two-page spread and no way to match the rest of the editor: everything inside
 * that frame belongs to the browser. Rendering here instead means the pages are
 * ours to lay out.
 *
 * PDF.js runs its parser in a worker; the worker file is bundled by Vite rather
 * than fetched, so this works with no network and inside the app's CSP, which
 * allows `worker-src 'self' blob:`.
 *
 * Everything asynchronous returns through this module so the component never
 * touches `pdfjs-dist` directly — which is also what lets the viewer be tested
 * without a canvas implementation.
 */
import * as pdfjs from "pdfjs-dist";
import workerSrc from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjs.GlobalWorkerOptions.workerSrc = workerSrc;

/** An open document. Call `close` when the viewer goes away. */
export interface PdfHandle {
  pageCount: number;
  /** A page's size at 100%, which is what "fit to the pane" is measured from. */
  naturalSize(page: number): Promise<PageSize>;
  /**
   * Draw a page into a canvas at `scale`, sized for the display's pixel ratio.
   *
   * Returns the CSS size the canvas was given, or `null` when a later call for
   * the same canvas superseded this one — a resize or a page turn arriving
   * mid-draw is ordinary, and it is not a failure.
   */
  renderPage(
    page: number,
    canvas: HTMLCanvasElement,
    scale: number,
  ): Promise<PageSize | null>;
  close(): void;
}

export interface PageSize {
  width: number;
  height: number;
}

/** Decode base64 file content into the bytes PDF.js wants. */
export function decodeBase64(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/**
 * The draw currently running on each canvas.
 *
 * PDF.js refuses two renders into one canvas — "Cannot use the same canvas
 * during multiple render() operations" — and a resize or a page turn arriving
 * while a page is still drawing is the normal case, not an edge one. The old
 * draw is cancelled rather than raced.
 */
const inFlight = new WeakMap<HTMLCanvasElement, pdfjs.RenderTask>();

/** Open a PDF from the bytes the backend read. */
export async function openPdf(base64: string): Promise<PdfHandle> {
  const task = pdfjs.getDocument({ data: decodeBase64(base64) });
  const document = await task.promise;

  return {
    pageCount: document.numPages,

    async naturalSize(pageNumber) {
      const page = await document.getPage(pageNumber);
      const { width, height } = page.getViewport({ scale: 1 });
      return { width, height };
    },

    async renderPage(pageNumber, canvas, scale) {
      const page = await document.getPage(pageNumber);
      const viewport = page.getViewport({ scale });
      // Draw at device resolution and let CSS scale it back down, or the page
      // is soft on every display made in the last fifteen years.
      const ratio = window.devicePixelRatio || 1;
      canvas.width = Math.floor(viewport.width * ratio);
      canvas.height = Math.floor(viewport.height * ratio);
      canvas.style.width = `${Math.floor(viewport.width)}px`;
      canvas.style.height = `${Math.floor(viewport.height)}px`;

      const context = canvas.getContext("2d");
      if (!context) throw new Error("this browser gave no 2D canvas context");
      context.setTransform(ratio, 0, 0, ratio, 0, 0);

      inFlight.get(canvas)?.cancel();
      const draw = page.render({ canvas, canvasContext: context, viewport });
      inFlight.set(canvas, draw);
      try {
        await draw.promise;
      } catch (error) {
        if (isCancelled(error)) return null;
        throw error;
      } finally {
        if (inFlight.get(canvas) === draw) inFlight.delete(canvas);
        page.cleanup();
      }
      return { width: viewport.width, height: viewport.height };
    },

    close() {
      // The loading task owns the worker; destroying the document proxy alone
      // would leave it running for the life of the window.
      void task.destroy();
    },
  };
}

/** Whether a rejected render was simply superseded. */
function isCancelled(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    (error as { name?: string }).name === "RenderingCancelledException"
  );
}
