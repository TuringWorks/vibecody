import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

// -- Types --------------------------------------------------------------------

interface ModelResponse {
  provider: string;
  model: string;
  content: string;
  duration_ms: number;
  tokens: number | null;
  error: string | null;
}

interface CompareResult {
  a: ModelResponse;
  b: ModelResponse;
}

interface ArenaVote {
  timestamp: string;
  prompt: string;
  provider_a: string;
  model_a: string;
  provider_b: string;
  model_b: string;
  winner: string;
}

interface ArenaStats {
  provider: string;
  wins: number;
  losses: number;
  ties: number;
  total: number;
  win_rate: number;
}

// -- Constants ----------------------------------------------------------------

const PROVIDERS = ["ollama", "claude", "openai", "gemini", "grok", "groq"];

const DEFAULT_MODELS: Record<string, string> = {
  ollama: "codellama",
  claude: "claude-sonnet-4-6",
  openai: "gpt-4o",
  gemini: "gemini-2.0-flash",
  grok: "grok-2",
  groq: "llama-3.3-70b-versatile",
};

// -- Sub-components -----------------------------------------------------------

function ProviderSelector({
  label, provider, model, onProvider, onModel,
}: {
  label: string; provider: string; model: string;
  onProvider: (v: string) => void; onModel: (v: string) => void;
}) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "8px", flex: 1 }}>
      <span style={{ color: "var(--text-muted)", fontSize: "12px", minWidth: "14px" }}>{label}</span>
      <select
        value={provider}
        onChange={e => { onProvider(e.target.value); onModel(DEFAULT_MODELS[e.target.value] ?? ""); }}
        style={{ background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "3px 6px", fontSize: "12px" }}
      >
        {PROVIDERS.map(p => <option key={p} value={p}>{p}</option>)}
      </select>
      <input
        value={model}
        onChange={e => onModel(e.target.value)}
        placeholder="model"
        style={{ flex: 1, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "3px 6px", fontSize: "12px", minWidth: 0 }}
      />
    </div>
  );
}

function BlindResponseCard({ content, side, error }: { content: string; side: "A" | "B"; error: string | null }) {
  const isError = !!error;
  return (
    <div style={{
      flex: 1,
      display: "flex",
      flexDirection: "column",
      border: `1px solid ${isError ? "var(--error-color)" : "var(--border-color)"}`,
      borderRadius: "6px",
      overflow: "hidden",
      minWidth: 0,
    }}>
      <div style={{
        padding: "6px 12px",
        background: "var(--bg-secondary)",
        display: "flex",
        alignItems: "center",
        gap: "8px",
      }}>
        <span style={{ fontWeight: "bold", color: side === "A" ? "var(--info-color)" : "var(--accent-color)" }}>
          Model {side}
        </span>
        <span style={{ color: "var(--text-muted)", fontSize: "11px", fontStyle: "italic" }}>
          (identity hidden)
        </span>
      </div>
      <div style={{ flex: 1, overflowY: "auto", padding: "10px 12px" }}>
        {isError ? (
          <span style={{ color: "var(--error-color)" }}>{error}</span>
        ) : (
          <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontFamily: "inherit", fontSize: "13px" }}>
            {content || <span style={{ color: "var(--text-muted)" }}>(empty response)</span>}
          </pre>
        )}
      </div>
    </div>
  );
}

// -- Main Panel ---------------------------------------------------------------

export function ArenaPanel() {
  // Provider / model selection
  const [providerA, setProviderA] = useState("ollama");
  const [modelA, setModelA] = useState(DEFAULT_MODELS.ollama);
  const [providerB, setProviderB] = useState("claude");
  const [modelB, setModelB] = useState(DEFAULT_MODELS.claude);

  // Prompt
  const [prompt, setPrompt] = useState("");

  // Battle state
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<CompareResult | null>(null);

  // Voting state
  const [voted, setVoted] = useState(false);
  const [voteChoice, setVoteChoice] = useState<string | null>(null);
  const [revealed, setRevealed] = useState(false);

  // Leaderboard
  const [history, setHistory] = useState<ArenaVote[]>([]);
  const [stats, setStats] = useState<ArenaStats[]>([]);

  useEffect(() => {
    loadHistory();
  }, []);

  const loadHistory = async () => {
    try {
      const [votes, s] = await invoke<[ArenaVote[], ArenaStats[]]>("get_arena_history");
      setHistory(votes);
      setStats(s);
    } catch {
      // fresh install, no history
    }
  };

  const handleBattle = async () => {
    if (!prompt.trim()) return;
    setLoading(true);
    setError(null);
    setResult(null);
    setVoted(false);
    setVoteChoice(null);
    setRevealed(false);
    // Randomize which side is which
    const flip = Math.random() < 0.5;
    try {
      const r = await invoke<CompareResult>("compare_models", {
        prompt: prompt.trim(),
        providerA: flip ? providerB : providerA,
        modelA: flip ? modelB : modelA,
        providerB: flip ? providerA : providerB,
        modelB: flip ? modelA : modelB,
      });
      setResult(r);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleVote = async (choice: "a" | "b" | "tie" | "both_bad") => {
    if (!result) return;
    setVoted(true);
    setVoteChoice(choice);
    setRevealed(true);

    const vote: ArenaVote = {
      timestamp: new Date().toISOString(),
      prompt: prompt.trim(),
      provider_a: result.a.provider,
      model_a: result.a.model,
      provider_b: result.b.provider,
      model_b: result.b.model,
      winner: choice,
    };
    try {
      await invoke("save_arena_vote", { vote });
      await loadHistory();
    } catch (e: unknown) {
      console.error("Failed to save arena vote:", e);
    }
  };

  const handleSendWinner = () => {
    if (!result || !voteChoice) return;
    let winnerContent = "";
    if (voteChoice === "a") {
      winnerContent = result.a.content;
    } else if (voteChoice === "b") {
      winnerContent = result.b.content;
    } else {
      // tie or both_bad -- send the first
      winnerContent = result.a.content || result.b.content;
    }
    if (winnerContent) {
      window.dispatchEvent(new CustomEvent("vibeui:inject-context", { detail: winnerContent }));
    }
  };

  const handleKey = (e: React.KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") handleBattle();
  };

  // -- Render -----------------------------------------------------------------

  const sortedStats = [...stats].sort((a, b) => b.win_rate - a.win_rate);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "10px", fontFamily: "var(--font-mono)", fontSize: "13px" }}>
      {/* Header */}
      <div style={{ fontWeight: "bold", marginBottom: "2px", display: "flex", alignItems: "center", gap: "8px" }}>
        <span>Arena Mode</span>
        <span style={{ color: "var(--text-muted)", fontSize: "11px", fontWeight: "normal" }}>
          Blind A/B comparison with voting
        </span>
      </div>

      {/* Provider selectors */}
      <div style={{ display: "flex", gap: "10px", flexWrap: "wrap" }}>
        <ProviderSelector label="A" provider={providerA} model={modelA} onProvider={setProviderA} onModel={setModelA} />
        <ProviderSelector label="B" provider={providerB} model={modelB} onProvider={setProviderB} onModel={setModelB} />
      </div>

      {/* Prompt input */}
      <textarea
        value={prompt}
        onChange={e => setPrompt(e.target.value)}
        onKeyDown={handleKey}
        placeholder="Enter a prompt... (Ctrl+Enter to battle)"
        rows={3}
        style={{
          resize: "vertical",
          background: "var(--bg-secondary)",
          color: "var(--text-primary)",
          border: "1px solid var(--border-color)",
          borderRadius: "4px",
          padding: "8px",
          fontFamily: "inherit",
          fontSize: "13px",
        }}
      />

      <button
        onClick={handleBattle}
        disabled={loading || !prompt.trim()}
        style={{
          alignSelf: "flex-start",
          background: loading ? "var(--bg-secondary)" : "var(--accent-color)",
          color: loading ? "var(--text-primary)" : "white", border: "none", borderRadius: "4px",
          padding: "6px 18px", cursor: loading ? "default" : "pointer",
          fontWeight: "bold",
        }}
      >
        {loading ? "Fighting..." : "Battle"}
      </button>

      {error && (
        <div style={{ color: "var(--error-color)", fontSize: "12px" }}>{error}</div>
      )}

      {/* Side-by-side blind responses */}
      {result && (
        <div style={{ display: "flex", gap: "10px", flex: 1, overflow: "hidden", minHeight: "150px" }}>
          <BlindResponseCard
            content={result.a.content}
            side="A"
            error={result.a.error}
          />
          <BlindResponseCard
            content={result.b.content}
            side="B"
            error={result.b.error}
          />
        </div>
      )}

      {/* Vote buttons -- shown after responses arrive, before reveal */}
      {result && !voted && (
        <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
          <span style={{ color: "var(--text-muted)", fontSize: "12px", alignSelf: "center" }}>Vote:</span>
          <button onClick={() => handleVote("a")} style={voteBtnStyle("var(--info-color)")}>A is better</button>
          <button onClick={() => handleVote("b")} style={voteBtnStyle("var(--accent-color)")}>B is better</button>
          <button onClick={() => handleVote("tie")} style={voteBtnStyle("var(--warning-color)")}>Tie</button>
          <button onClick={() => handleVote("both_bad")} style={voteBtnStyle("var(--error-color)")}>Both bad</button>
        </div>
      )}

      {/* Reveal panel -- shown after voting */}
      {revealed && result && (
        <div style={{
          border: "1px solid var(--border-color)",
          borderRadius: "6px",
          padding: "10px 14px",
          background: "var(--bg-secondary)",
        }}>
          <div style={{ fontWeight: "bold", marginBottom: "6px" }}>
            Reveal
            {voteChoice === "a" && " -- Model A wins!"}
            {voteChoice === "b" && " -- Model B wins!"}
            {voteChoice === "tie" && " -- Tie"}
            {voteChoice === "both_bad" && " -- Both bad"}
          </div>
          <div style={{ display: "flex", gap: "20px", fontSize: "12px", flexWrap: "wrap" }}>
            <div>
              <span style={{ color: "var(--info-color)", fontWeight: "bold" }}>Model A: </span>
              <span>{result.a.provider}/{result.a.model}</span>
              {result.a.duration_ms > 0 && (
                <span style={{ color: "var(--text-muted)", marginLeft: "6px" }}>
                  {result.a.duration_ms}ms{result.a.tokens != null && ` / ${result.a.tokens} tok`}
                </span>
              )}
            </div>
            <div>
              <span style={{ color: "var(--accent-color)", fontWeight: "bold" }}>Model B: </span>
              <span>{result.b.provider}/{result.b.model}</span>
              {result.b.duration_ms > 0 && (
                <span style={{ color: "var(--text-muted)", marginLeft: "6px" }}>
                  {result.b.duration_ms}ms{result.b.tokens != null && ` / ${result.b.tokens} tok`}
                </span>
              )}
            </div>
          </div>
          <button
            onClick={handleSendWinner}
            style={{
              marginTop: "8px",
              background: "var(--accent-color)",
              color: "var(--btn-primary-fg)", border: "none", borderRadius: "4px",
              padding: "4px 12px", cursor: "pointer", fontSize: "12px",
            }}
          >
            Send winner to Chat
          </button>
        </div>
      )}

      {/* Placeholder when idle */}
      {!result && !loading && (
        <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-muted)", textAlign: "center" }}>
          Enter a prompt and click Battle to start a blind comparison.<br />
          Identities are hidden until you vote.
        </div>
      )}

      {/* Leaderboard */}
      {sortedStats.length > 0 && (
        <div style={{ borderTop: "1px solid var(--border-color)", paddingTop: "8px" }}>
          <div style={{ fontWeight: "bold", marginBottom: "4px", fontSize: "12px" }}>Leaderboard</div>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "12px" }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)" }}>
                <th style={{ textAlign: "left", padding: "3px 6px" }}>Provider</th>
                <th style={{ textAlign: "right", padding: "3px 6px" }}>Wins</th>
                <th style={{ textAlign: "right", padding: "3px 6px" }}>Losses</th>
                <th style={{ textAlign: "right", padding: "3px 6px" }}>Ties</th>
                <th style={{ textAlign: "right", padding: "3px 6px" }}>Total</th>
                <th style={{ textAlign: "right", padding: "3px 6px" }}>Win Rate</th>
              </tr>
            </thead>
            <tbody>
              {sortedStats.map(s => (
                <tr key={s.provider} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "3px 6px", fontWeight: "bold" }}>{s.provider}</td>
                  <td style={{ textAlign: "right", padding: "3px 6px", color: "var(--success-color)" }}>{s.wins}</td>
                  <td style={{ textAlign: "right", padding: "3px 6px", color: "var(--error-color)" }}>{s.losses}</td>
                  <td style={{ textAlign: "right", padding: "3px 6px", color: "var(--warning-color)" }}>{s.ties}</td>
                  <td style={{ textAlign: "right", padding: "3px 6px" }}>{s.total}</td>
                  <td style={{ textAlign: "right", padding: "3px 6px", color: "var(--info-color)" }}>
                    {(s.win_rate * 100).toFixed(1)}%
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <div style={{ color: "var(--text-muted)", fontSize: "11px", marginTop: "4px" }}>
            {history.length} total vote{history.length !== 1 ? "s" : ""}
          </div>
        </div>
      )}
    </div>
  );
}

// -- Helpers ------------------------------------------------------------------

function voteBtnStyle(color: string): React.CSSProperties {
  return {
    background: "transparent",
    color,
    border: `1px solid ${color}`,
    borderRadius: "4px",
    padding: "4px 14px",
    cursor: "pointer",
    fontSize: "12px",
    fontWeight: "bold",
  };
}
