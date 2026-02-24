import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ContextPicker } from "./ContextPicker";
import "./AIChat.css";

// ── Voice input hook ──────────────────────────────────────────────────────────

// Web Speech API types (not in lib.dom.d.ts by default in some TS configs)
type SpeechRecognitionCtor = new () => {
    continuous: boolean;
    interimResults: boolean;
    lang: string;
    onresult: ((event: SpeechRecognitionEvent) => void) | null;
    onend: (() => void) | null;
    onerror: (() => void) | null;
    start(): void;
    stop(): void;
};
type SpeechRecognitionEvent = {
    results: { length: number; [i: number]: { [0]: { transcript: string } } };
};

function getSpeechRecognitionCtor(): SpeechRecognitionCtor | null {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const w = window as any;
    return (w["SpeechRecognition"] ?? w["webkitSpeechRecognition"] ?? null) as SpeechRecognitionCtor | null;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type RecInstance = any;

function useVoiceInput(onTranscript: (text: string) => void) {
    const [isListening, setIsListening] = useState(false);
    const recRef = useRef<RecInstance | null>(null);

    const toggle = () => {
        const SR = getSpeechRecognitionCtor();
        if (!SR) {
            alert("Speech recognition is not supported in this environment.");
            return;
        }

        if (isListening) {
            recRef.current?.stop();
            setIsListening(false);
            return;
        }

        const r = new SR();
        r.continuous = false;
        r.interimResults = true;
        r.lang = navigator.language || "en-US";

        r.onresult = (e: SpeechRecognitionEvent) => {
            const parts: string[] = [];
            for (let i = 0; i < e.results.length; i++) parts.push(e.results[i][0].transcript);
            onTranscript(parts.join(""));
        };

        r.onend = () => setIsListening(false);
        r.onerror = () => setIsListening(false);

        r.start();
        recRef.current = r;
        setIsListening(true);
    };

    return { isListening, toggle };
}

interface Message {
    role: "user" | "assistant";
    content: string;
}

interface PendingWrite {
    path: string;
    content: string;
}

interface ChatResponse {
    message: string;
    tool_output: string;
    pending_write?: PendingWrite;
}

interface AIChatProps {
    provider: string;
    context?: string;
    fileTree?: string[];
    currentFile?: string | null;
    onFileAction?: () => void;
    onPendingWrite?: (path: string, content: string) => void;
}

/** Extract the `@query` fragment at the cursor position, or null if none. */
function getAtQuery(text: string, cursorPos: number): { query: string; start: number } | null {
    const beforeCursor = text.slice(0, cursorPos);
    // Find the last `@` that is not preceded by a non-whitespace character
    const match = beforeCursor.match(/(?:^|[\s\n])(@(\S*))$/);
    if (!match) return null;
    const fullMatch = match[1]; // the "@..." token
    const query = match[2];    // everything after @
    const start = beforeCursor.lastIndexOf(fullMatch);
    return { query, start };
}

export function AIChat({ provider, context, fileTree, currentFile, onFileAction, onPendingWrite }: AIChatProps) {
    const [messages, setMessages] = useState<Message[]>([]);
    const [input, setInput] = useState("");
    const [isLoading, setIsLoading] = useState(false);
    const [pickerQuery, setPickerQuery] = useState<string | null>(null);
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    const { isListening, toggle: toggleVoice } = useVoiceInput((transcript) =>
        setInput((prev) => prev + transcript)
    );

    const cleanMessage = (content: string): string => {
        let cleaned = content.replace(/<write_file path="([^"]+)">[\s\S]*?<\/write_file>/g, "✅ Proposed changes to $1");
        cleaned = cleaned.replace(/<read_file path="([^"]+)" \/>/g, "📖 Read file $1");
        cleaned = cleaned.replace(/<list_dir path="([^"]+)" \/>/g, "📂 Listed directory $1");
        return cleaned;
    };

    const sendMessage = async () => {
        if (!input.trim()) return;
        if (!provider) {
            setMessages(prev => [...prev, {
                role: "assistant",
                content: "⚠️ Please select an AI provider from the dropdown menu first."
            }]);
            return;
        }

        const userMessage: Message = { role: "user", content: input };
        setMessages([...messages, userMessage]);
        setInput("");
        setPickerQuery(null);
        setIsLoading(true);

        try {
            const response = await invoke<ChatResponse>("send_chat_message", {
                request: {
                    messages: [...messages, userMessage],
                    provider,
                    context,
                    file_tree: fileTree,
                    current_file: currentFile,
                },
            });

            let displayContent = cleanMessage(response.message);
            const assistantMessage: Message = { role: "assistant", content: displayContent };
            setMessages((prev) => [...prev, assistantMessage]);

            if (response.pending_write && onPendingWrite) {
                onPendingWrite(response.pending_write.path, response.pending_write.content);
            }
            if (onFileAction) {
                onFileAction();
            }
        } catch (error) {
            console.error("Failed to send message:", error);
            setMessages((prev) => [...prev, {
                role: "assistant",
                content: "Sorry, I encountered an error. Please make sure an AI provider is configured.",
            }]);
        } finally {
            setIsLoading(false);
        }
    };

    const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        const val = e.target.value;
        setInput(val);
        const cursor = e.target.selectionStart ?? val.length;
        const atInfo = getAtQuery(val, cursor);
        setPickerQuery(atInfo ? atInfo.query : null);
    };

    const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
        // Let ContextPicker handle arrow/enter/escape when visible
        if (pickerQuery !== null && ["ArrowUp", "ArrowDown", "Enter", "Escape"].includes(e.key)) {
            e.preventDefault();
            return;
        }
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
        // Hide picker on space (token ended without selection)
        if (e.key === " ") {
            setPickerQuery(null);
        }
    };

    const handlePickerSelect = (insertion: string) => {
        if (!textareaRef.current) return;
        const cursor = textareaRef.current.selectionStart ?? input.length;
        const atInfo = getAtQuery(input, cursor);
        if (atInfo === null) return;

        // Replace `@<query>` at atInfo.start with the selected insertion
        const before = input.slice(0, atInfo.start);
        const after = input.slice(atInfo.start + 1 + atInfo.query.length); // skip "@query"
        const newInput = before + insertion + " " + after;
        setInput(newInput);
        setPickerQuery(null);

        // Restore focus
        setTimeout(() => textareaRef.current?.focus(), 0);
    };

    return (
        <div className="ai-chat">
            <div className="chat-header">
                <h3>🤖 AI Assistant</h3>
                <p className="chat-subtitle">
                    Ask questions about your code. Type <kbd>@file:</kbd>, <kbd>@git</kbd>, or <kbd>@web:</kbd> to inject context. Click 🎤 for voice input.
                </p>
            </div>

            <div className="chat-messages">
                {messages.length === 0 ? (
                    <div className="chat-empty">
                        <p>👋 Hi! I'm your AI coding assistant.</p>
                        <p>Ask me anything about your code, or use <kbd>@file:path</kbd> and <kbd>@git</kbd> to inject context.</p>
                    </div>
                ) : (
                    messages.map((msg, idx) => (
                        <div key={idx} className={`message message-${msg.role}`}>
                            <div className="message-icon">
                                {msg.role === "user" ? "👤" : "🤖"}
                            </div>
                            <div className="message-content">
                                <pre>{msg.content}</pre>
                            </div>
                        </div>
                    ))
                )}
                {isLoading && (
                    <div className="message message-assistant">
                        <div className="message-icon">🤖</div>
                        <div className="message-content">
                            <div className="typing-indicator">
                                <span></span><span></span><span></span>
                            </div>
                        </div>
                    </div>
                )}
            </div>

            <div className="chat-input" style={{ position: "relative" }}>
                {pickerQuery !== null && (
                    <ContextPicker
                        query={pickerQuery}
                        onSelect={handlePickerSelect}
                        onClose={() => setPickerQuery(null)}
                    />
                )}
                <textarea
                    ref={textareaRef}
                    value={input}
                    onChange={handleInputChange}
                    onKeyDown={handleKeyDown}
                    placeholder="Ask a question… (Enter to send, Shift+Enter for newline, @ for context)"
                    rows={3}
                />
                <div style={{ display: "flex", gap: "6px", alignSelf: "flex-end" }}>
                    <button
                        onClick={toggleVoice}
                        title={isListening ? "Stop recording" : "Voice input"}
                        className={`mic-btn${isListening ? " listening" : ""}`}
                    >
                        🎤
                    </button>
                    <button onClick={sendMessage} disabled={!input.trim() || isLoading}>
                        Send
                    </button>
                </div>
            </div>
        </div>
    );
}
