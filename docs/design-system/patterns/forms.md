---
layout: page
title: Forms — Design System
permalink: /design-system/patterns/forms/
---

# Forms

Form layout, validation, and submit feedback patterns for panels.

---

## Basic Form

```tsx
const [value, setValue] = useState("");
const [saving, setSaving] = useState(false);
const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

const handleSubmit = async () => {
  setSaving(true);
  setMessage(null);
  try {
    await invoke("save_item", { value });
    setMessage({ type: "success", text: "Saved successfully." });
  } catch (e) {
    setMessage({ type: "error", text: String(e) });
  } finally {
    setSaving(false);
  }
};

return (
  <div className="panel-body">
    {/* Field */}
    <div style={{ marginBottom: "var(--space-3)" }}>
      <label style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: "var(--space-1)" }}>
        Name
      </label>
      <input
        className="panel-input"
        value={value}
        onChange={e => setValue(e.target.value)}
        placeholder="Enter name…"
      />
    </div>

    {/* Submit feedback */}
    {message && (
      <div style={{ marginBottom: "var(--space-3)", fontSize: "var(--font-size-sm)", color: message.type === "success" ? "var(--success-color)" : "var(--error-color)" }}>
        {message.text}
      </div>
    )}

    {/* Submit */}
    <button
      className="panel-btn panel-btn-primary"
      onClick={handleSubmit}
      disabled={saving || !value.trim()}
    >
      {saving ? "Saving…" : "Save"}
    </button>
  </div>
);
```

---

## Field Patterns

### Text input with label

```tsx
<div style={{ marginBottom: "var(--space-3)" }}>
  <label style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: "var(--space-1)" }}>
    Label
  </label>
  <input className="panel-input" value={val} onChange={e => setVal(e.target.value)} />
</div>
```

### Select

```tsx
<div style={{ marginBottom: "var(--space-3)" }}>
  <label style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: "var(--space-1)" }}>
    Provider
  </label>
  <select className="panel-input" value={provider} onChange={e => setProvider(e.target.value)}>
    <option value="ollama">Ollama</option>
    <option value="claude">Claude</option>
    <option value="openai">OpenAI</option>
  </select>
</div>
```

---

## Validation

```tsx
const isValid = value.trim().length > 0 && value.length <= 256;

<button
  className="panel-btn panel-btn-primary"
  disabled={!isValid || saving}
>
  Submit
</button>

{!isValid && value.length > 0 && (
  <span className="text-error" style={{ fontSize: "var(--font-size-sm)", display: "block", marginTop: "var(--space-1)" }}>
    Must be 1–256 characters
  </span>
)}
```

---

## Rules

1. **Label every field** — no placeholder-only fields
2. **Disable submit while loading** — always add `disabled={saving}` to submit button
3. **Clear message on retry** — `setMessage(null)` before each submit attempt
4. **Optimistic updates** — for fast operations, update local state before the await resolves
