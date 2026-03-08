import { useState } from "react";

// ── Types ────────────────────────────────────────────────────────────────────

export interface VibeUIBlock {
    type: "buttons" | "form" | "table";
    props: Record<string, unknown>;
    rawText: string;
}

export interface VibeUIAction {
    type: "button_click" | "form_submit";
    value: unknown;
}

interface FormField {
    name: string;
    label: string;
    type: "text" | "select" | "number" | "textarea";
    options?: string[];
    placeholder?: string;
}

// ── Parser ───────────────────────────────────────────────────────────────────

const BLOCK_RE = /:::vibe-ui:(buttons|form|table)\s*\n([\s\S]*?)\n:::/g;

/**
 * Extract structured vibe-ui blocks from agent text output.
 * Each block is fenced with `:::vibe-ui:<type>` / `:::` and contains a JSON body.
 */
export function parseVibeUIBlocks(text: string): VibeUIBlock[] {
    const blocks: VibeUIBlock[] = [];
    let match: RegExpExecArray | null;
    // Reset lastIndex in case the regex was used before
    BLOCK_RE.lastIndex = 0;
    while ((match = BLOCK_RE.exec(text)) !== null) {
        const blockType = match[1] as VibeUIBlock["type"];
        const jsonStr = match[2].trim();
        try {
            const props = JSON.parse(jsonStr);
            blocks.push({ type: blockType, props, rawText: match[0] });
        } catch {
            // Malformed JSON — skip this block silently
        }
    }
    return blocks;
}

/**
 * Return the input text with all vibe-ui blocks stripped out.
 */
export function stripVibeUIBlocks(text: string): string {
    BLOCK_RE.lastIndex = 0;
    return text.replace(BLOCK_RE, "").trim();
}

// ── Shared styles ────────────────────────────────────────────────────────────

const S = {
    container: { marginTop: 8, display: "flex", flexDirection: "column" as const, gap: 10 },
    btn: {
        padding: "6px 14px",
        fontSize: 12,
        fontWeight: 500 as const,
        background: "var(--accent-color)",
        color: "var(--text-primary)",
        border: "1px solid var(--accent-color)",
        borderRadius: 4,
        cursor: "pointer",
        transition: "background 0.15s",
    },
    btnHover: { background: "var(--accent-color)" },
    label: { display: "block", fontSize: 11, color: "var(--text-secondary)", marginBottom: 2 },
    input: {
        width: "100%",
        boxSizing: "border-box" as const,
        padding: "5px 8px",
        fontSize: 12,
        background: "var(--bg-tertiary)",
        color: "var(--text-primary)",
        border: "1px solid var(--border-color)",
        borderRadius: 4,
        outline: "none",
    },
    table: {
        width: "100%",
        borderCollapse: "collapse" as const,
        fontSize: 12,
        color: "var(--text-primary)",
    },
    th: {
        textAlign: "left" as const,
        padding: "6px 8px",
        borderBottom: "2px solid var(--border-color)",
        background: "var(--bg-secondary)",
        fontWeight: 600,
        fontSize: 11,
        color: "var(--text-secondary)",
    },
    td: { padding: "5px 8px", borderBottom: "1px solid var(--border-color)" },
    evenRow: { background: "rgba(124,58,237,0.06)" },
};

// ── Sub-components ───────────────────────────────────────────────────────────

function ButtonsBlock({ props, onAction }: { props: Record<string, unknown>; onAction: (a: VibeUIAction) => void }) {
    const options = (props.options as string[]) || [];
    return (
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
            {options.map((opt) => (
                <button
                    key={opt}
                    style={S.btn}
                    onMouseEnter={(e) => { (e.currentTarget.style.opacity = "0.85"); }}
                    onMouseLeave={(e) => { (e.currentTarget.style.opacity = "1"); }}
                    onClick={() => onAction({ type: "button_click", value: opt })}
                >
                    {opt}
                </button>
            ))}
        </div>
    );
}

function FormBlock({ props, onAction }: { props: Record<string, unknown>; onAction: (a: VibeUIAction) => void }) {
    const fields = (props.fields as FormField[]) || [];
    const [values, setValues] = useState<Record<string, string>>(() => {
        const init: Record<string, string> = {};
        fields.forEach((f) => { init[f.name] = ""; });
        return init;
    });

    const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
        e.preventDefault();
        onAction({ type: "form_submit", value: { ...values } });
    };

    return (
        <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {fields.map((field) => (
                <div key={field.name}>
                    <label style={S.label}>{field.label}</label>
                    {field.type === "select" ? (
                        <select
                            value={values[field.name] || ""}
                            onChange={(e) => setValues((v) => ({ ...v, [field.name]: e.target.value }))}
                            style={S.input}
                        >
                            <option value="">-- select --</option>
                            {(field.options || []).map((o) => (
                                <option key={o} value={o}>{o}</option>
                            ))}
                        </select>
                    ) : field.type === "textarea" ? (
                        <textarea
                            value={values[field.name] || ""}
                            onChange={(e) => setValues((v) => ({ ...v, [field.name]: e.target.value }))}
                            placeholder={field.placeholder || ""}
                            rows={3}
                            style={{ ...S.input, resize: "vertical" }}
                        />
                    ) : (
                        <input
                            type={field.type || "text"}
                            value={values[field.name] || ""}
                            onChange={(e) => setValues((v) => ({ ...v, [field.name]: e.target.value }))}
                            placeholder={field.placeholder || ""}
                            style={S.input}
                        />
                    )}
                </div>
            ))}
            <button type="submit" style={{ ...S.btn, alignSelf: "flex-start" }}>Submit</button>
        </form>
    );
}

function TableBlock({ props }: { props: Record<string, unknown> }) {
    const headers = (props.headers as string[]) || [];
    const rows = (props.rows as string[][]) || [];
    return (
        <table style={S.table}>
            <thead>
                <tr>
                    {headers.map((h) => (
                        <th key={h} style={S.th}>{h}</th>
                    ))}
                </tr>
            </thead>
            <tbody>
                {rows.map((row, ri) => (
                    <tr key={ri} style={ri % 2 === 0 ? S.evenRow : undefined}>
                        {row.map((cell, ci) => (
                            <td key={ci} style={S.td}>{cell}</td>
                        ))}
                    </tr>
                ))}
            </tbody>
        </table>
    );
}

// ── Main renderer ────────────────────────────────────────────────────────────

interface AgentUIRendererProps {
    blocks: VibeUIBlock[];
    onAction: (action: VibeUIAction) => void;
}

export function AgentUIRenderer({ blocks, onAction }: AgentUIRendererProps) {
    if (blocks.length === 0) return null;
    return (
        <div style={S.container}>
            {blocks.map((block, i) => {
                switch (block.type) {
                    case "buttons":
                        return <ButtonsBlock key={i} props={block.props} onAction={onAction} />;
                    case "form":
                        return <FormBlock key={i} props={block.props} onAction={onAction} />;
                    case "table":
                        return <TableBlock key={i} props={block.props} />;
                    default:
                        return null;
                }
            })}
        </div>
    );
}
