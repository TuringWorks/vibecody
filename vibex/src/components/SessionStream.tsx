import { useState } from "react";
import { TaskPrompt } from "./TaskPrompt";
import { ToolUseBlock } from "./ToolUseBlock";

interface SessionStreamProps {
  daemonUrl: string;
  daemonOnline: boolean;
}

type StreamItem =
  | { kind: "user"; text: string }
  | { kind: "agent"; text: string }
  | { kind: "tool"; tool: string; summary: string; detail?: string; durationMs?: number };

/**
 * VX-103 — center linear conversation (Codex screenshots 1, 8).
 * User messages are right-aligned chips; agent output is left-aligned prose;
 * agent actions render as structured ToolUseBlocks (VX-104). The composer
 * (TaskPrompt) is pinned at the bottom and carries all run controls.
 */
export function SessionStream({ daemonUrl, daemonOnline }: SessionStreamProps) {
  // Scripted demo transcript until VX-105/VX-112 wire live streaming.
  const [items] = useState<StreamItem[]>([
    { kind: "user", text: "fix the auth timeout bug in the login flow" },
    { kind: "tool", tool: "Read", summary: "src/auth/mod.rs (2.3 KB)", detail: "14 lines of context", durationMs: 3000 },
    { kind: "tool", tool: "Edit", summary: "src/auth/mod.rs (+18/-4)", detail: "exponential backoff with jitter", durationMs: 7000 },
    { kind: "tool", tool: "Run", summary: "cargo test -- auth", detail: "✓ 3 tests passing", durationMs: 12000 },
    { kind: "agent", text: "Done. Added exponential backoff with jitter to the auth timeout handler; all auth tests pass." },
  ]);

  return (
    <div className="vx-stream">
      <header className="vx-stream__header">
        <span className="vx-stream__title">fix the auth timeout bug</span>
      </header>

      <div className="vx-stream__body">
        {items.map((item, i) => {
          if (item.kind === "user") {
            return (
              <div key={i} className="vx-msg vx-msg--user">
                <div className="vx-msg__chip">{item.text}</div>
              </div>
            );
          }
          if (item.kind === "agent") {
            return (
              <div key={i} className="vx-msg vx-msg--agent">
                <div className="vx-msg__prose">{item.text}</div>
              </div>
            );
          }
          return (
            <ToolUseBlock
              key={i}
              tool={item.tool}
              summary={item.summary}
              detail={item.detail}
              durationMs={item.durationMs}
            />
          );
        })}
      </div>

      <TaskPrompt daemonUrl={daemonUrl} daemonOnline={daemonOnline} />
    </div>
  );
}
