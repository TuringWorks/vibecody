/**
 * ModelSelector — Shared provider + model dropdown pair.
 *
 * Used across CounselPanel, ArenaPanel, MultiModelPanel, etc.
 * Reads from the cached model registry so provider/model lists
 * load instantly after the first fetch.
 */
import { useModelRegistry } from "../../hooks/useModelRegistry";

const selectStyle: React.CSSProperties = {
  padding: "5px 8px",
  borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-tertiary, #1a1a2e)",
  color: "var(--text-primary)",
  fontSize: 12,
  fontFamily: "inherit",
  cursor: "pointer",
  minWidth: 0,
};

interface ModelSelectorProps {
  /** Currently selected provider */
  provider: string;
  /** Currently selected model */
  model: string;
  /** Called when provider changes (model auto-resets to first available) */
  onProviderChange: (provider: string) => void;
  /** Called when model changes */
  onModelChange: (model: string) => void;
  /** Additional style for the container */
  style?: React.CSSProperties;
  /** Show compact (single row) layout */
  compact?: boolean;
}

export function ModelSelector({
  provider,
  model,
  onProviderChange,
  onModelChange,
  style,
  compact = true,
}: ModelSelectorProps) {
  const { providers, modelsForProvider, loading } = useModelRegistry();
  const models = modelsForProvider(provider);

  const handleProviderChange = (newProvider: string) => {
    onProviderChange(newProvider);
    // Auto-select first model for the new provider
    const newModels = modelsForProvider(newProvider);
    if (newModels.length > 0 && !newModels.includes(model)) {
      onModelChange(newModels[0]);
    }
  };

  return (
    <div style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap", ...style }}>
      <select
        style={{ ...selectStyle, flex: compact ? "1 1 90px" : "1 1 120px" }}
        value={provider}
        onChange={(e) => handleProviderChange(e.target.value)}
      >
        {providers.map((p) => (
          <option key={p} value={p}>{p}</option>
        ))}
      </select>
      <select
        style={{ ...selectStyle, flex: compact ? "2 1 120px" : "2 1 160px" }}
        value={model}
        onChange={(e) => onModelChange(e.target.value)}
      >
        {models.length === 0 && (
          <option value={model}>{loading ? "Loading..." : model || "No models"}</option>
        )}
        {models.map((m) => (
          <option key={m} value={m}>{m}</option>
        ))}
        {models.length > 0 && !models.includes(model) && model && (
          <option value={model}>{model} (custom)</option>
        )}
      </select>
    </div>
  );
}
