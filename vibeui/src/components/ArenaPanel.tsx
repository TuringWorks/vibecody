import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModelRegistry, PROVIDER_DEFAULT_MODEL } from "../hooks/useModelRegistry";

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

// -- Sub-components -----------------------------------------------------------

function ProviderSelector({
  label, provider, model, onProvider, onModel,
}: {
  label: string; provider: string; model: string;
  onProvider: (v: string) => void; onModel: (v: string) => void;
}) {
  const { providers, modelsForProvider } = useModelRegistry();
  const listId = `arena-models-${label}`;
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "8px", flex: 1 }}>
      <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)", minWidth: "14px" }}>{label}</span>
      <select
        value={provider}
        onChange={e => { onProvider(e.target.value); onModel(PROVIDER_DEFAULT_MODEL[e.target.value] ?? ""); }}
        className="panel-select"
      >
        {providers.map(p => <option key={p} value={p}>{p}</option>)}
      </select>
      <datalist id={listId}>
        {modelsForProvider(provider).map(m => <option key={m} value={m} />)}
      </datalist>
      <input
        value={model}
        onChange={e => onModel(e.target.value)}
        list={listId}
        placeholder="model"
        className="panel-input"
        style={{ flex: 1, minWidth: 0 }}
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
      borderRadius: "var(--radius-sm)",
      overflow: "hidden",
      minWidth: 0,
    }}>
      <div style={{
        padding: "8px 12px",
        background: "var(--bg-secondary)",
        display: "flex",
        alignItems: "center",
        gap: "8px",
      }}>
        <span style={{ fontWeight: "bold", color: side === "A" ? "var(--info-color)" : "var(--accent-color)" }}>
          Model {side}
        </span>
        <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", fontStyle: "italic" }}>
          (identity hidden)
        </span>
      </div>
      <div style={{ flex: 1, overflowY: "auto", padding: "12px 12px" }}>
        {isError ? (
          <span style={{ color: "var(--error-color)" }}>{error}</span>
        ) : (
          <pre style={{ margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-word", fontFamily: "inherit", fontSize: "var(--font-size-md)" }}>
            {content || <span style={{ color: "var(--text-secondary)" }}>(empty response)</span>}
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
  const [modelA, setModelA] = useState(PROVIDER_DEFAULT_MODEL.ollama ?? "");
  const [providerB, setProviderB] = useState("claude");
  const [modelB, setModelB] = useState(PROVIDER_DEFAULT_MODEL.claude ?? "");

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
    <div className="panel-container" style={{ padding: "12px", gap: "12px" }}>
      {/* Header */}
      <div style={{ fontWeight: "bold", marginBottom: "2px", display: "flex", alignItems: "center", gap: "8px" }}>
        <span>Arena Mode</span>
        <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", fontWeight: "normal" }}>
          Blind A/B comparison with voting
        </span>
      </div>

      {/* Provider selectors */}
      <div style={{ display: "flex", gap: "12px", flexWrap: "wrap" }}>
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
        className="panel-input panel-textarea panel-input-full"
        style={{ resize: "vertical", fontFamily: "inherit" }}
      />

      <button
        onClick={handleBattle}
        disabled={loading || !prompt.trim()}
        className="panel-btn panel-btn-primary"
        style={{ alignSelf: "flex-start" }}
      >
        {loading ? "Fighting..." : "Battle"}
      </button>

      {error && (
        <div className="panel-error">{error}</div>
      )}

      {/* Side-by-side blind responses */}
      {result && (
        <div style={{ display: "flex", gap: "12px", flex: 1, overflow: "hidden", minHeight: "150px", flexGrow: 1 }}>
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
          <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)", alignSelf: "center" }}>Vote:</span>
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
          borderRadius: "var(--radius-sm)",
          padding: "12px 16px",
          background: "var(--bg-secondary)",
        }}>
          <div style={{ fontWeight: "bold", marginBottom: "8px" }}>
            Reveal
            {voteChoice === "a" && " -- Model A wins!"}
            {voteChoice === "b" && " -- Model B wins!"}
            {voteChoice === "tie" && " -- Tie"}
            {voteChoice === "both_bad" && " -- Both bad"}
          </div>
          <div style={{ display: "flex", gap: "20px", fontSize: "var(--font-size-base)", flexWrap: "wrap" }}>
            <div>
              <span style={{ color: "var(--info-color)", fontWeight: "bold" }}>Model A: </span>
              <span>{result.a.provider}/{result.a.model}</span>
              {result.a.duration_ms > 0 && (
                <span style={{ color: "var(--text-secondary)", marginLeft: "8px" }}>
                  {result.a.duration_ms}ms{result.a.tokens != null && ` / ${result.a.tokens} tok`}
                </span>
              )}
            </div>
            <div>
              <span style={{ color: "var(--accent-color)", fontWeight: "bold" }}>Model B: </span>
              <span>{result.b.provider}/{result.b.model}</span>
              {result.b.duration_ms > 0 && (
                <span style={{ color: "var(--text-secondary)", marginLeft: "8px" }}>
                  {result.b.duration_ms}ms{result.b.tokens != null && ` / ${result.b.tokens} tok`}
                </span>
              )}
            </div>
          </div>
          <button
            onClick={handleSendWinner}
            className="panel-btn panel-btn-primary panel-btn-sm"
            style={{ marginTop: "8px" }}
          >
            Send winner to Chat
          </button>
        </div>
      )}

      {/* Placeholder when idle */}
      {!result && !loading && (
        <div className="panel-empty" style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
          Enter a prompt and click Battle to start a blind comparison.<br />
          Identities are hidden until you vote.
        </div>
      )}

      {/* Leaderboard */}
      {sortedStats.length > 0 && (
        <div style={{ borderTop: "1px solid var(--border-color)", paddingTop: "8px" }}>
          <div style={{ fontWeight: "bold", marginBottom: "4px", fontSize: "var(--font-size-base)" }}>Leaderboard</div>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)" }}>
                <th style={{ textAlign: "left", padding: "3px 8px" }}>Provider</th>
                <th style={{ textAlign: "right", padding: "3px 8px" }}>Wins</th>
                <th style={{ textAlign: "right", padding: "3px 8px" }}>Losses</th>
                <th style={{ textAlign: "right", padding: "3px 8px" }}>Ties</th>
                <th style={{ textAlign: "right", padding: "3px 8px" }}>Total</th>
                <th style={{ textAlign: "right", padding: "3px 8px" }}>Win Rate</th>
              </tr>
            </thead>
            <tbody>
              {sortedStats.map(s => (
                <tr key={s.provider} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "3px 8px", fontWeight: "bold" }}>{s.provider}</td>
                  <td style={{ textAlign: "right", padding: "3px 8px", color: "var(--success-color)" }}>{s.wins}</td>
                  <td style={{ textAlign: "right", padding: "3px 8px", color: "var(--error-color)" }}>{s.losses}</td>
                  <td style={{ textAlign: "right", padding: "3px 8px", color: "var(--warning-color)" }}>{s.ties}</td>
                  <td style={{ textAlign: "right", padding: "3px 8px" }}>{s.total}</td>
                  <td style={{ textAlign: "right", padding: "3px 8px", color: "var(--info-color)" }}>
                    {(s.win_rate * 100).toFixed(1)}%
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", marginTop: "4px" }}>
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
    borderRadius: "var(--radius-xs-plus)",
    padding: "4px 14px",
    cursor: "pointer",
    fontSize: "var(--font-size-base)",
    fontWeight: "bold",
  };
}
