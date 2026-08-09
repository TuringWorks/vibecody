"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.splitThinking = splitThinking;
exports.isReasoningOnly = isReasoningOnly;
/** Namespace prefix, e.g. the `mm:` in minimax-m3's `<mm:think>`. */
const NS = String.raw `(?:[A-Za-z][\w.-]*:)?`;
const BLOCK = new RegExp(`<${NS}think(?:ing)?>([\\s\\S]*?)</${NS}think(?:ing)?>`, "g");
const ORPHAN_CLOSE = new RegExp(`^([\\s\\S]*?)</${NS}think(?:ing)?>`);
const UNCLOSED = new RegExp(`<${NS}think(?:ing)?>([\\s\\S]*)$`);
function splitThinking(text) {
    const reasoning = [];
    const keep = (body) => {
        const trimmed = body.trim();
        if (trimmed)
            reasoning.push(trimmed);
        return "";
    };
    // Balanced blocks first, so the unbalanced passes below only ever see the
    // genuinely malformed leftovers.
    let visible = text.replace(BLOCK, (_m, body) => keep(body));
    // Orphan close: the provider already ate the opening tag.
    const orphan = visible.match(ORPHAN_CLOSE);
    if (orphan) {
        keep(orphan[1]);
        visible = visible.slice(orphan[0].length);
    }
    // Never closed: the stream was cut mid-thought.
    visible = visible.replace(UNCLOSED, (_m, body) => keep(body));
    return { reasoning, visible: visible.trim() };
}
/** True when the turn carried reasoning but nothing else — the model thought out loud. */
function isReasoningOnly(split) {
    return split.reasoning.length > 0 && split.visible.length === 0;
}
