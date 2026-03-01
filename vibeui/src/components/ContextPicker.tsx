import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./ContextPicker.css";

interface ContextFileEntry {
    path: string;
    name: string;
}

interface ContextPickerProps {
    /** The fragment the user typed after `@`, e.g. "src/ma" or "git" */
    query: string;
    onSelect: (insertion: string) => void;
    onClose: () => void;
}

const SPECIAL_ITEMS = [
    { label: "@git", description: "Inject current git branch, changed files, and diff" },
    { label: "@web:", description: "Fetch a web page and inject its content" },
    { label: "@docs:", description: "Fetch library docs (e.g. @docs:tokio, @docs:py:requests, @docs:react)" },
    { label: "@folder:", description: "Inject all files in a folder" },
    { label: "@terminal", description: "Inject the last 200 lines of terminal output" },
    { label: "@symbol:", description: "Inject source code of a named symbol (function, struct, etc.)" },
    { label: "@codebase:", description: "Semantic search over the codebase" },
    { label: "@github:", description: "Fetch a GitHub issue or PR (e.g. @github:owner/repo#42)" },
    { label: "@jira:", description: "Fetch a Jira issue (e.g. @jira:PROJ-123) — needs JIRA_BASE_URL env" },
    { label: "@html-selected", description: "Inject the last HTML element selected in the Browser panel" },
];

export function ContextPicker({ query, onSelect, onClose }: ContextPickerProps) {
    const [files, setFiles] = useState<ContextFileEntry[]>([]);
    const [selected, setSelected] = useState(0);
    const containerRef = useRef<HTMLDivElement>(null);

    // Fetch matching files whenever query changes
    useEffect(() => {
        // Skip file search for special prefixes that have their own dynamic items
        if (
            query.startsWith("web:") ||
            query.startsWith("docs:") ||
            query.startsWith("folder:") ||
            query.startsWith("symbol:") ||
            query.startsWith("codebase:") ||
            query.startsWith("github:") ||
            query.startsWith("jira:") ||
            query.startsWith("html-selected")
        ) {
            setFiles([]);
            return;
        }
        const isSpecialPrefix = SPECIAL_ITEMS.some(
            (s) => s.label.replace("@", "").startsWith(query.toLowerCase())
        ) && query.length <= 8;
        if (isSpecialPrefix && query.length <= 8) {
            setFiles([]);
            return;
        }
        invoke<ContextFileEntry[]>("search_files_for_context", { query })
            .then(setFiles)
            .catch(() => setFiles([]));
    }, [query]);

    // Build the combined list: specials first, then files
    let specials: { label: string; description?: string }[];
    if (query.startsWith("web:")) {
        // Show a single dynamic item for the URL being typed
        const urlPart = query.slice(4);
        specials = [{
            label: `@web:${urlPart}`,
            description: urlPart ? `Fetch ${urlPart}` : "Type a URL...",
        }];
    } else if (query.startsWith("docs:")) {
        const docsPart = query.slice(5);
        specials = [{
            label: `@docs:${docsPart}`,
            description: docsPart
                ? `Fetch docs for "${docsPart}" (prefix rs:, py:, or npm:)`
                : "Type a package name (e.g. tokio, py:requests, npm:react)...",
        }];
    } else if (query.startsWith("folder:")) {
        // Show a single dynamic item for the folder path being typed
        const folderPart = query.slice(7);
        specials = [{
            label: `@folder:${folderPart}`,
            description: folderPart ? `Inject all files in ${folderPart}` : "Type a folder path...",
        }];
    } else if (query.startsWith("symbol:")) {
        const symPart = query.slice(7);
        specials = [{
            label: `@symbol:${symPart}`,
            description: symPart ? `Inject source of symbol "${symPart}"` : "Type a symbol name...",
        }];
    } else if (query.startsWith("codebase:")) {
        const cbPart = query.slice(9);
        specials = [{
            label: `@codebase:${cbPart}`,
            description: cbPart ? `Search codebase for "${cbPart}"` : "Type a search query...",
        }];
    } else if (query.startsWith("github:")) {
        const ghPart = query.slice(7);
        specials = [{
            label: `@github:${ghPart}`,
            description: ghPart
                ? `Fetch GitHub issue/PR: @github:${ghPart}`
                : "Type owner/repo#N (e.g. torvalds/linux#1234)...",
        }];
    } else if (query.startsWith("jira:")) {
        const jiraPart = query.slice(5);
        specials = [{
            label: `@jira:${jiraPart}`,
            description: jiraPart
                ? `Fetch Jira issue: ${jiraPart}`
                : "Type issue key (e.g. PROJ-123)...",
        }];
    } else {
        specials = SPECIAL_ITEMS.filter(
            (s) => query === "" || s.label.toLowerCase().includes(query.toLowerCase())
        );
    }
    const allItems: { label: string; description?: string }[] = [
        ...specials,
        ...files.map((f) => ({ label: `@file:${f.path}`, description: f.name })),
    ];

    // Keyboard navigation
    useEffect(() => {
        const handler = (e: KeyboardEvent) => {
            if (e.key === "ArrowDown") {
                e.preventDefault();
                setSelected((s) => Math.min(s + 1, allItems.length - 1));
            } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setSelected((s) => Math.max(s - 1, 0));
            } else if (e.key === "Enter") {
                e.preventDefault();
                if (allItems[selected]) {
                    onSelect(allItems[selected].label);
                }
            } else if (e.key === "Escape") {
                onClose();
            }
        };
        window.addEventListener("keydown", handler);
        return () => window.removeEventListener("keydown", handler);
    }, [allItems, selected, onSelect, onClose]);

    // Reset selection when items change
    useEffect(() => {
        setSelected(0);
    }, [allItems.length]);

    if (allItems.length === 0) return null;

    return (
        <div className="context-picker" ref={containerRef}>
            <div className="context-picker-header">@ Context</div>
            {allItems.map((item, idx) => (
                <div
                    key={item.label}
                    className={`context-picker-item ${idx === selected ? "selected" : ""}`}
                    onMouseEnter={() => setSelected(idx)}
                    onClick={() => onSelect(item.label)}
                >
                    <span className="context-picker-label">{item.label}</span>
                    {item.description && (
                        <span className="context-picker-desc">{item.description}</span>
                    )}
                </div>
            ))}
            <div className="context-picker-hint">↑↓ navigate · Enter select · Esc close</div>
        </div>
    );
}
