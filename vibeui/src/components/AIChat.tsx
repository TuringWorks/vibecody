import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./AIChat.css";

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

export function AIChat({ provider, context, fileTree, currentFile, onFileAction, onPendingWrite }: AIChatProps) {
    const [messages, setMessages] = useState<Message[]>([]);
    const [input, setInput] = useState("");
    const [isLoading, setIsLoading] = useState(false);

    const cleanMessage = (content: string): string => {
        // Replace <write_file> blocks with a summary
        let cleaned = content.replace(/<write_file path="([^"]+)">[\s\S]*?<\/write_file>/g, "✅ Proposed changes to $1");

        // Hide <read_file> tags
        cleaned = cleaned.replace(/<read_file path="([^"]+)" \/>/g, "📖 Read file $1");

        // Hide <list_dir> tags
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
        setIsLoading(true);

        try {
            const response = await invoke<ChatResponse>("send_chat_message", {
                request: {
                    messages: [...messages, userMessage],
                    provider: provider,
                    context: context,
                    file_tree: fileTree,
                    current_file: currentFile
                },
            });

            console.log("DEBUG: Received response from backend:", response);

            let displayContent = cleanMessage(response.message);
            if (response.tool_output) {
                // Also clean tool output if needed, or just show it
                // displayContent += `\n\nTool Output:\n${response.tool_output}`;
            }

            const assistantMessage: Message = { role: "assistant", content: displayContent };
            setMessages((prev) => [...prev, assistantMessage]);

            // Handle pending write if present
            if (response.pending_write) {
                console.log("DEBUG: Pending write found in response:", response.pending_write);
                if (onPendingWrite) {
                    console.log("DEBUG: Calling onPendingWrite callback");
                    onPendingWrite(response.pending_write.path, response.pending_write.content);
                } else {
                    console.warn("DEBUG: onPendingWrite callback is missing!");
                }
            } else {
                console.log("DEBUG: No pending write in response");
            }

            // Trigger file action callback if provided (e.g. to reload editor)
            if (onFileAction) {
                onFileAction();
            }
        } catch (error) {
            console.error("Failed to send message:", error);
            const errorMessage: Message = {
                role: "assistant",
                content: "Sorry, I encountered an error. Please make sure an AI provider is configured.",
            };
            setMessages((prev) => [...prev, errorMessage]);
        } finally {
            setIsLoading(false);
        }
    };

    const handleKeyPress = (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
    };

    return (
        <div className="ai-chat">
            <div className="chat-header">
                <h3>🤖 AI Assistant</h3>
                <p className="chat-subtitle">Ask questions about your code</p>
            </div>

            <div className="chat-messages">
                {messages.length === 0 ? (
                    <div className="chat-empty">
                        <p>👋 Hi! I'm your AI coding assistant.</p>
                        <p>Ask me anything about your code!</p>
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
                                <span></span>
                                <span></span>
                                <span></span>
                            </div>
                        </div>
                    </div>
                )}
            </div>

            <div className="chat-input">
                <textarea
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyPress={handleKeyPress}
                    placeholder="Ask a question... (Enter to send, Shift+Enter for new line)"
                    rows={3}
                />
                <button onClick={sendMessage} disabled={!input.trim() || isLoading}>
                    Send
                </button>
            </div>
        </div>
    );
}
