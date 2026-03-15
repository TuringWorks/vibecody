import { useRef, useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useToast } from "../hooks/useToast";
import { ContextPicker } from "./ContextPicker";
import { flowContext } from "../utils/FlowContext";
import { Mic, User } from "lucide-react";
import "./AIChat.css";

// ── Voice input hook (MediaRecorder + Groq Whisper via Tauri backend) ────────

function useVoiceInput(onTranscript: (text: string) => void) {
 const [isListening, setIsListening] = useState(false);
 const [isTranscribing, setIsTranscribing] = useState(false);
 const recorderRef = useRef<MediaRecorder | null>(null);
 const chunksRef = useRef<Blob[]>([]);
 const { toast } = useToast();

 const toggle = useCallback(async () => {
   // Stop recording
   if (isListening && recorderRef.current) {
     recorderRef.current.stop();
     return;
   }

   // Start recording
   try {
     const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
     const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
       ? "audio/webm;codecs=opus"
       : "audio/webm";
     const recorder = new MediaRecorder(stream, { mimeType });
     chunksRef.current = [];

     recorder.ondataavailable = (e) => {
       if (e.data.size > 0) chunksRef.current.push(e.data);
     };

     recorder.onstop = async () => {
       // Stop all tracks to release mic
       stream.getTracks().forEach((t) => t.stop());
       setIsListening(false);

       const blob = new Blob(chunksRef.current, { type: mimeType });
       if (blob.size < 100) return; // too short

       setIsTranscribing(true);
       try {
         // Convert blob to base64
         const arrayBuf = await blob.arrayBuffer();
         const bytes = new Uint8Array(arrayBuf);
         let binary = "";
         for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
         const base64 = btoa(binary);

         const text = await invoke<string>("transcribe_audio_bytes", {
           audioBase64: base64,
           mimeType: mimeType.split(";")[0], // "audio/webm"
         });
         if (text.trim()) onTranscript(text);
       } catch (e) {
         const msg = String(e);
         if (msg.includes("GROQ_API_KEY")) {
           toast.warn("Set GROQ_API_KEY env var for voice input (Groq Whisper).");
         } else {
           toast.error(`Transcription failed: ${msg}`);
         }
       }
       setIsTranscribing(false);
     };

     recorder.onerror = () => {
       stream.getTracks().forEach((t) => t.stop());
       setIsListening(false);
       toast.error("Microphone recording failed.");
     };

     recorder.start();
     recorderRef.current = recorder;
     setIsListening(true);
   } catch (e) {
     toast.warn(`Microphone access denied: ${e}`);
   }
 }, [isListening, onTranscript, toast]);

 return { isListening, isTranscribing, toggle };
}

interface Message {
 role: "user" | "assistant";
 content: string;
 timestamp?: number;
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
 /** When set, appends this text to the current input (Cascade flow inject). */
 pendingInput?: string;
 /** Called once after pendingInput is consumed. */
 onPendingInputConsumed?: () => void;
}

/** Extract the `@query` fragment at the cursor position, or null if none. */
function getAtQuery(text: string, cursorPos: number): { query: string; start: number } | null {
 const beforeCursor = text.slice(0, cursorPos);
 // Find the last `@` that is not preceded by a non-whitespace character
 const match = beforeCursor.match(/(?:^|[\s\n])(@(\S*))$/);
 if (!match) return null;
 const fullMatch = match[1]; // the "@..." token
 const query = match[2]; // everything after @
 const start = beforeCursor.lastIndexOf(fullMatch);
 return { query, start };
}

function formatTime(ts?: number): string {
 if (!ts) return "";
 const d = new Date(ts);
 return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function AIChat({ provider, context, fileTree, currentFile, onFileAction, onPendingWrite, pendingInput, onPendingInputConsumed }: AIChatProps) {
 const [messages, setMessages] = useState<Message[]>([]);
 const [input, setInput] = useState("");
 const [isLoading, setIsLoading] = useState(false);
 const [streamingText, setStreamingText] = useState(""); // live assistant text while streaming
 const [pickerQuery, setPickerQuery] = useState<string | null>(null);
 const [copiedIdx, setCopiedIdx] = useState<number | null>(null);
 const textareaRef = useRef<HTMLTextAreaElement>(null);
 const messagesEndRef = useRef<HTMLDivElement>(null);
 const cancelledRef = useRef(false);
 // Streaming speed metrics
 const streamStartMsRef = useRef<number | null>(null);
 const streamCharsRef = useRef<number>(0);
 const [tokensPerSec, setTokensPerSec] = useState<number | null>(null);
 const { isListening, isTranscribing, toggle: toggleVoice } = useVoiceInput((transcript) =>
 setInput((prev) => prev + transcript)
 );

 // Auto-scroll to latest message or new streaming chunk
 useEffect(() => {
 messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
 }, [messages, streamingText, isLoading]);

 // Register Tauri chat stream event listeners once
 useEffect(() => {
 let cancelled = false;
 const unlisteners: Array<() => void> = [];
 (async () => {
 const u1 = await listen<string>("chat:chunk", (e) => {
 const now = Date.now();
 const chunk = e.payload;
 if (streamStartMsRef.current === null) streamStartMsRef.current = now;
 streamCharsRef.current += chunk.length;
 const elapsedSec = (now - streamStartMsRef.current) / 1000;
 if (elapsedSec > 0) {
 setTokensPerSec(Math.round((streamCharsRef.current / 4) / elapsedSec));
 }
 setStreamingText((prev) => prev + chunk);
 });
 if (cancelled) { u1(); return; }
 unlisteners.push(u1);

 const u2 = await listen<ChatResponse>("chat:complete", (e) => {
 const response = e.payload;
 const displayContent = cleanMessage(response.message);
 setMessages((prev) => {
 // Replace the in-progress streaming entry with final message
 const updated = [...prev, { role: "assistant" as const, content: displayContent, timestamp: Date.now() }];
 return updated;
 });
 setStreamingText("");
 setTokensPerSec(null);
 setIsLoading(false);
 if (response.pending_write && onPendingWrite) {
 onPendingWrite(response.pending_write.path, response.pending_write.content);
 }
 if (onFileAction) onFileAction();
 });
 if (cancelled) { u2(); return; }
 unlisteners.push(u2);

 const u3 = await listen<string>("chat:error", (e) => {
 setMessages((prev) => [...prev, {
 role: "assistant",
 content: ` ${e.payload}`,
 timestamp: Date.now(),
 }]);
 setStreamingText("");
 setTokensPerSec(null);
 setIsLoading(false);
 });
 if (cancelled) { u3(); return; }
 unlisteners.push(u3);
 })();
 return () => {
 cancelled = true;
 unlisteners.forEach((fn) => fn());
 };
 // eslint-disable-next-line react-hooks/exhaustive-deps
 }, [onFileAction, onPendingWrite]);

 // Consume pendingInput from Cascade "Inject into chat"
 useEffect(() => {
 if (pendingInput) {
 setInput((prev) => prev ? `${prev}\n${pendingInput}` : pendingInput);
 onPendingInputConsumed?.();
 textareaRef.current?.focus();
 }
 // eslint-disable-next-line react-hooks/exhaustive-deps
 }, [pendingInput]);

 const cleanMessage = (content: string): string => {
 let cleaned = content.replace(/<write_file path="([^"]+)">[\s\S]*?<\/write_file>/g, "Proposed changes to $1");
 cleaned = cleaned.replace(/<read_file path="([^"]+)" \/>/g, "Read file $1");
 cleaned = cleaned.replace(/<list_dir path="([^"]+)" \/>/g, "Listed directory $1");
 return cleaned;
 };

 const sendMessage = useCallback(async () => {
 if (!input.trim()) return;
 if (!provider) {
 setMessages(prev => [...prev, {
 role: "assistant",
 content: "Please select an AI provider from the dropdown menu first."
 }]);
 return;
 }

 const userMessage: Message = { role: "user", content: input, timestamp: Date.now() };
 setMessages((prev) => [...prev, userMessage]);
 setInput("");
 setPickerQuery(null);
 cancelledRef.current = false;
 setIsLoading(true);
 setStreamingText("");
 setTokensPerSec(null);
 streamStartMsRef.current = null;
 streamCharsRef.current = 0;

 // Record user message in Cascade flow immediately
 flowContext.add({
 kind: "chat",
 summary: userMessage.content.slice(0, 100),
 detail: `Q: ${userMessage.content}`,
 });

 try {
 // Kick off streaming — response arrives via chat:chunk / chat:complete / chat:error events.
 await invoke("stream_chat_message", {
 request: {
 messages: [...messages, userMessage],
 provider,
 context,
 file_tree: fileTree,
 current_file: currentFile,
 },
 });
 } catch (error) {
 console.error("Failed to start chat stream:", error);
 setMessages((prev) => [...prev, {
 role: "assistant",
 content: "Sorry, I encountered an error. Please make sure an AI provider is configured.",
 }]);
 setStreamingText("");
 setIsLoading(false);
 }
 // eslint-disable-next-line react-hooks/exhaustive-deps
 }, [input, provider, context, fileTree, currentFile, messages]);

 const stopMessage = useCallback(async () => {
 cancelledRef.current = true;
 await invoke("stop_chat_stream").catch(() => {});
 // Commit whatever was streamed so far as the final message
 setMessages((prev) => {
 if (streamingText) {
 return [...prev, { role: "assistant" as const, content: cleanMessage(streamingText) }];
 }
 return prev;
 });
 setStreamingText("");
 setTokensPerSec(null);
 setIsLoading(false);
 // eslint-disable-next-line react-hooks/exhaustive-deps
 }, [streamingText]);

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
 <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
 <h3 style={{ margin: 0 }}>AI Assistant</h3>
 <div style={{ display: "flex", gap: "6px" }}>
 {isLoading && (
 <button
 onClick={stopMessage}
 style={{ fontSize: "11px", padding: "2px 8px", background: "var(--text-danger, #ff4d4f)", color: "#fff", border: "none", borderRadius: "4px", cursor: "pointer" }}
 title="Stop generation"
 >
 Stop
 </button>
 )}
 {messages.length > 0 && !isLoading && (
 <button
 onClick={() => setMessages([])}
 style={{ fontSize: "11px", padding: "2px 8px", background: "var(--bg-tertiary)", color: "var(--text-secondary)", border: "1px solid var(--border-color)", borderRadius: "4px", cursor: "pointer" }}
 title="Clear chat history"
 >
 Clear
 </button>
 )}
 </div>
 </div>
 <p className="chat-subtitle">
 Ask questions about your code. Type <kbd>@file:</kbd>, <kbd>@git</kbd>, or <kbd>@web:</kbd> to inject context. Click the mic icon for voice input.
 </p>
 </div>

 <div className="chat-messages">
 {messages.length === 0 ? (
 <div className="chat-empty">
 <p>Hi! I'm your AI coding assistant.</p>
 <p>Ask me anything about your code, or use <kbd>@file:path</kbd> and <kbd>@git</kbd> to inject context.</p>
 </div>
 ) : (
 messages.map((msg, idx) => (
 <div key={idx} className={`message message-${msg.role}`}>
 <div className="message-icon">
 {msg.role === "user" ? <User size={14} strokeWidth={1.5} /> : ""}
 </div>
 {msg.timestamp && (
 <time className="message-time" dateTime={new Date(msg.timestamp).toISOString()}>
 {formatTime(msg.timestamp)}
 </time>
 )}
 <div className="message-content" style={{ position: "relative" }}>
 <pre>{msg.content}</pre>
 {msg.role === "assistant" && (
 <button
 onClick={() => {
 navigator.clipboard.writeText(msg.content).then(() => {
 setCopiedIdx(idx);
 setTimeout(() => setCopiedIdx(null), 1500);
 }).catch(() => {});
 }}
 title="Copy response"
 style={{
 position: "absolute", top: "4px", right: "4px",
 background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
 borderRadius: "4px", cursor: "pointer", fontSize: "11px",
 padding: "2px 6px", color: copiedIdx === idx ? "var(--text-success, #3fb950)" : "var(--text-secondary)",
 opacity: 0.8,
 }}
 >
 {copiedIdx === idx ? "✓ Copied" : "Copy"}
 </button>
 )}
 </div>
 </div>
 ))
 )}
 {isLoading && (
 <div className="message message-assistant">
 <div className="message-icon"></div>
 <div className="message-content">
 {streamingText ? (
 <>
 <div style={{ whiteSpace: "pre-wrap" }}>
 {streamingText}
 {/* blinking cursor */}
 <span style={{
 display: "inline-block",
 width: "2px",
 height: "1em",
 background: "currentColor",
 verticalAlign: "text-bottom",
 animation: "blink 1s step-end infinite",
 marginLeft: 1,
 }} />
 </div>
 {tokensPerSec !== null && (
 <div
 aria-live="polite"
 style={{
 marginTop: 4,
 fontSize: 11,
 color: "var(--text-muted)",
 fontVariantNumeric: "tabular-nums",
 }}
 >
 {tokensPerSec} tok/s · ~{Math.round(streamCharsRef.current / 4)} tokens
 </div>
 )}
 </>
 ) : (
 <div className="typing-indicator">
 <span></span><span></span><span></span>
 </div>
 )}
 </div>
 </div>
 )}
 <div ref={messagesEndRef} />
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
 title={isTranscribing ? "Transcribing..." : isListening ? "Stop recording" : "Voice input"}
 className={`mic-btn${isListening ? " listening" : ""}${isTranscribing ? " transcribing" : ""}`}
 disabled={isTranscribing}
 >
 <Mic size={14} strokeWidth={1.5} />
 </button>
 <button onClick={sendMessage} disabled={!input.trim() || isLoading}>
 Send
 </button>
 </div>
 </div>
 </div>
 );
}
