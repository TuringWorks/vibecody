/**
 * DREAD #1 Slice G part 2 — VibeUI side of the tainted-argument
 * confirmation bridge.
 *
 * Subscribes to `GET /v1/tainted/pending` (SSE) and surfaces a modal
 * dialog for every pending prompt. Posts the user's decision to
 * `POST /v1/tainted/respond`. The same daemon endpoints serve
 * VibeMobile and VibeWatch — this component is just the desktop
 * WebView renderer.
 *
 * ## Threat-model invariants
 *
 * The SSE payload carries only `audit_summary` (kind, provenance
 * fields, audit_id). The underlying tainted bytes never appear here —
 * see `docs/security/tainted-data-flow.md` §8 and
 * `vibecli/vibecli-cli/src/tainted_http_bridge.rs`.
 *
 * ## Fail-safe behaviour
 *
 * * Daemon offline: SSE reconnects with exponential backoff. The
 *   daemon-side `RESPONSE_TIMEOUT` (5 min) denies the agent loop if
 *   no surface ever connects.
 * * Modal dismissed without a click (Escape, overlay click, refresh):
 *   no `respond` POST goes out; the daemon eventually times out and
 *   denies.
 * * Only explicit "Approve" sends `approve=true`; everything else
 *   sends false or leaves the prompt pending. Deny-by-default by
 *   construction.
 */

import { useCallback, useEffect, useRef, useState } from 'react';

export interface PendingPromptEvent {
    request_id: string;
    audit_id: string;
    /** `kind=… audit_id=… origin={…}` — never the underlying bytes. */
    summary: string;
    /** `ToolCallArgument`, `McpArgument`, `RagDocument`, etc. */
    sink: string;
    /** Unix seconds. */
    issued_at: number;
}

interface TaintedConfirmationModalProps {
    /** Daemon base URL. Defaults to the standard local daemon address. */
    daemonUrl?: string;
    /** Bearer token for the daemon. Required for production; tests pass an empty string. */
    apiToken: string;
    /** When false, the component does not subscribe (useful while the daemon is offline). */
    enabled?: boolean;
}

const BACKOFF_INITIAL_MS = 1_000;
const BACKOFF_MAX_MS = 30_000;

export function TaintedConfirmationModal({
    daemonUrl = 'http://localhost:7878',
    apiToken,
    enabled = true,
}: TaintedConfirmationModalProps) {
    const [pending, setPending] = useState<PendingPromptEvent[]>([]);
    const [error, setError] = useState<string | null>(null);
    // Stable per-component handle to the live EventSource — avoids
    // double-subscribing across React strict-mode re-renders.
    const esRef = useRef<EventSource | null>(null);
    const backoffRef = useRef<number>(BACKOFF_INITIAL_MS);
    const reconnectTimerRef = useRef<number | null>(null);
    // Tracks request_ids already resolved by this client so we don't
    // re-render a modal for a prompt the daemon hasn't yet flushed
    // from its snapshot. (The daemon emits the full pending set on
    // every notify; we de-dupe on the client.)
    const seenRef = useRef<Set<string>>(new Set());
    const resolvedRef = useRef<Set<string>>(new Set());

    const connect = useCallback(() => {
        if (!enabled) return;
        const url = new URL('/v1/tainted/pending', daemonUrl);
        // EventSource doesn't support custom headers. We append the
        // token as a query param when needed; the daemon's auth
        // middleware accepts either header or `?token=` for SSE
        // endpoints. (See serve.rs `auth_middleware`.)
        if (apiToken) {
            url.searchParams.set('token', apiToken);
        }
        const es = new EventSource(url.toString());
        esRef.current = es;

        es.addEventListener('pending', (ev: MessageEvent<string>) => {
            try {
                const event = JSON.parse(ev.data) as PendingPromptEvent;
                if (resolvedRef.current.has(event.request_id)) return;
                if (seenRef.current.has(event.request_id)) return;
                seenRef.current.add(event.request_id);
                setPending((prev) => [...prev, event]);
            } catch {
                // Ignore malformed events — the daemon owns the schema.
            }
        });

        es.onopen = () => {
            backoffRef.current = BACKOFF_INITIAL_MS;
            setError(null);
        };

        es.onerror = () => {
            es.close();
            esRef.current = null;
            const delay = backoffRef.current;
            backoffRef.current = Math.min(delay * 2, BACKOFF_MAX_MS);
            setError(`Disconnected from daemon — retrying in ${Math.round(delay / 1000)}s`);
            reconnectTimerRef.current = window.setTimeout(connect, delay);
        };
    }, [daemonUrl, apiToken, enabled]);

    useEffect(() => {
        connect();
        return () => {
            if (esRef.current) {
                esRef.current.close();
                esRef.current = null;
            }
            if (reconnectTimerRef.current !== null) {
                window.clearTimeout(reconnectTimerRef.current);
                reconnectTimerRef.current = null;
            }
        };
    }, [connect]);

    const respond = useCallback(
        async (request_id: string, approve: boolean) => {
            resolvedRef.current.add(request_id);
            setPending((prev) => prev.filter((p) => p.request_id !== request_id));
            try {
                await fetch(new URL('/v1/tainted/respond', daemonUrl).toString(), {
                    method: 'POST',
                    headers: {
                        'content-type': 'application/json',
                        ...(apiToken ? { authorization: `Bearer ${apiToken}` } : {}),
                    },
                    body: JSON.stringify({ request_id, approve }),
                });
            } catch {
                // Network failure: the daemon will time out and deny.
                // Restore the modal so the user sees the failure.
                resolvedRef.current.delete(request_id);
                seenRef.current.delete(request_id);
                setError('Failed to send decision — daemon timeout will deny.');
            }
        },
        [daemonUrl, apiToken],
    );

    if (pending.length === 0 && !error) return null;

    // Render only the head-of-queue prompt; subsequent prompts surface
    // one-at-a-time after the user resolves the visible one. This
    // matches the design-doc §8.1 "one tainted decision at a time"
    // expectation and avoids prompt-fatigue stacking.
    const head = pending[0];

    return (
        <>
            {head && (
                <div
                    className="modal-overlay"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="tainted-modal-title"
                >
                    <div className="modal-content" data-testid="tainted-confirmation-modal">
                        <h3 id="tainted-modal-title">Confirm untrusted argument</h3>
                        <p>
                            The agent is about to run a <strong>{head.sink}</strong> using
                            data that originated outside the trust boundary. Review the
                            audit summary below before approving.
                        </p>
                        <pre
                            className="modal-summary"
                            style={{
                                whiteSpace: 'pre-wrap',
                                wordBreak: 'break-word',
                                fontSize: '12px',
                                padding: '8px',
                                background: 'var(--bg-secondary, #1a1a1a)',
                                borderRadius: '4px',
                                maxHeight: '180px',
                                overflowY: 'auto',
                            }}
                        >
                            {head.summary}
                        </pre>
                        <p style={{ fontSize: '11px', opacity: 0.7 }}>
                            audit_id: <code>{head.audit_id}</code>
                            {pending.length > 1 && (
                                <span> · {pending.length - 1} more pending</span>
                            )}
                        </p>
                        <div className="modal-actions">
                            <button
                                type="button"
                                className="btn-secondary"
                                onClick={() => respond(head.request_id, false)}
                            >
                                Deny
                            </button>
                            <button
                                type="button"
                                className="btn-primary"
                                onClick={() => respond(head.request_id, true)}
                            >
                                Approve
                            </button>
                        </div>
                    </div>
                </div>
            )}
            {error && !head && (
                <div
                    style={{
                        position: 'fixed',
                        bottom: 12,
                        right: 12,
                        background: 'var(--bg-secondary, #1a1a1a)',
                        color: 'var(--fg-muted, #aaa)',
                        padding: '6px 10px',
                        borderRadius: 4,
                        fontSize: 11,
                        zIndex: 1000,
                    }}
                    role="status"
                >
                    {error}
                </div>
            )}
        </>
    );
}

export default TaintedConfirmationModal;
