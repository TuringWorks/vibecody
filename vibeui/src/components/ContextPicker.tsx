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
];

export function ContextPicker({ query, onSelect, onClose }: ContextPickerProps) {
    const [files, setFiles] = useState<ContextFileEntry[]>([]);
    const [selected, setSelected] = useState(0);
    const containerRef = useRef<HTMLDivElement>(null);

    // Fetch matching files whenever query changes
    useEffect(() => {
        const isGitPrefix = "git".startsWith(query.toLowerCase()) || query === "";
        if (isGitPrefix && query.length <= 3) {
            setFiles([]);
            return;
        }
        invoke<ContextFileEntry[]>("search_files_for_context", { query })
            .then(setFiles)
            .catch(() => setFiles([]));
    }, [query]);

    // Build the combined list: specials first, then files
    const specials = SPECIAL_ITEMS.filter(
        (s) => query === "" || s.label.toLowerCase().includes(query.toLowerCase())
    );
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
