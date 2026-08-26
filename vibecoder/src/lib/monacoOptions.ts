/**
 * Editor options every Monaco instance in the app must carry.
 *
 * # Why this exists
 *
 * Monaco renders hovers, the suggest widget and the context menu as *overflow
 * widgets*: absolutely-positioned children of the editor's own DOM. That is
 * fine while they fit inside the editor, and wrong the moment they do not —
 * `.editor-container` is `overflow: hidden` (App.css), so a widget wider than
 * the editor pane is **clipped**, not repositioned.
 *
 * The visible symptom is a diagnostic hover cut off mid-word against the right
 * edge of the editor:
 *
 *     Cannot find module 'child_process'. Did you me…
 *     'moduleResolution' option to 'nodenext', or to…
 *
 * The text wraps to the hover's own width, and the hover is then sliced by the
 * container — so the wrapping hides the truncation. It reads like a message
 * that ends there rather than one that has been cut, and the part naming the
 * fix is the part that is missing.
 *
 * `fixedOverflowWidgets` moves those widgets into a `position: fixed` layer, so
 * they escape the clip and overlay whatever is beside the editor, which is what
 * VS Code itself does.
 *
 * # The constraint this depends on
 *
 * `position: fixed` is relative to the viewport **only** while no ancestor
 * establishes a containing block — `transform`, `filter`, `backdrop-filter`,
 * `perspective` and `will-change` all do. If one is ever added to
 * `.main-container`, `.editor-area` or `.editor-container`, hovers will start
 * being clipped again and nothing will fail: no error, no type, no test that
 * does not render at a real size. Checked when this was written — the only
 * `backdrop-filter` rules are on `.header` and `.tour-tooltip`, neither of
 * which is an editor ancestor.
 */
export const MONACO_OVERFLOW_OPTIONS = {
  fixedOverflowWidgets: true,
} as const;
