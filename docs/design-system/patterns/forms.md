---
render_with_liquid: false
layout: page
title: Forms — Design System
permalink: /design-system/patterns/forms/
---

# Forms

Form patterns for data entry in panels. Panels use compact, single-card forms — not full-page form layouts.

---

## Core Form Card

The standard container for a group of inputs + submit action:

```tsx
<div className="panel-card" style={{ marginBottom: 10 }}>
  {/* Field 1 */}
  <div className="panel-label">Intent</div>
  <input
    className="panel-input panel-input-full"
    value={intent}
    onChange={e => setIntent(e.target.value)}
    placeholder="make this module testable"
    style={{ marginBottom: 8 }}
  />

  {/* Field 2 */}
  <div className="panel-label">Target Files</div>
  <input
    className="panel-input panel-input-full"
    value={files}
    onChange={e => setFiles(e.target.value)}
    placeholder="src/main.rs, src/lib.rs"
    style={{ marginBottom: 8 }}
  />

  {/* Action */}
  <button
    className="panel-btn panel-btn-primary"
    onClick={handleSubmit}
    disabled={loading || !intent}
  >
    {loading ? "Generating…" : "Generate Plan"}
  </button>
</div>
```

### Spacing rule inside form cards

| Gap between | Spacing |
|-------------|---------|
| Label → Input | `margin-bottom: 4px` on `.panel-label` (automatic) |
| Input → next label | `margin-bottom: 8px` on input |
| Last input → button | `margin-bottom: 8px` on input |
| After button (bottom of card) | Card padding handles it (`padding: 12px`) |

---

## Field Types

### Single-line text
```tsx
<div className="panel-label">Workspace Path</div>
<input
  className="panel-input panel-input-full"
  value={path}
  onChange={e => setPath(e.target.value)}
  placeholder="/Users/me/project"
  style={{ marginBottom: 8 }}
/>
```

### Multi-line text (prose)
```tsx
<div className="panel-label">Description</div>
<textarea
  className="panel-input panel-input-full panel-textarea"
  value={description}
  onChange={e => setDescription(e.target.value)}
  rows={4}
  placeholder="Describe the change..."
  style={{ marginBottom: 8 }}
/>
```

### Code input (mono)
```tsx
<div className="panel-label">Paste code to analyze</div>
<textarea
  className="panel-input panel-input-full panel-textarea"
  value={code}
  onChange={e => setCode(e.target.value)}
  rows={8}
  style={{ fontFamily: "var(--font-mono)", marginBottom: 8 }}
  placeholder="fn main() { ... }"
/>
```

### Select / Dropdown
```tsx
<div className="panel-label">Provider</div>
<select
  className="panel-select"
  value={provider}
  onChange={e => setProvider(e.target.value)}
  style={{ width: "100%", marginBottom: 8 }}
>
  <option value="anthropic">Anthropic</option>
  <option value="openai">OpenAI</option>
  <option value="ollama">Ollama</option>
</select>
```

### Path + browse (inline)
```tsx
<div className="panel-label">Workspace Path</div>
<div className="panel-row" style={{ marginBottom: 8 }}>
  <input
    className="panel-input panel-input-full"
    value={path}
    onChange={e => setPath(e.target.value)}
    placeholder="."
  />
  <button className="panel-btn panel-btn-secondary" style={{ flexShrink: 0 }}>Browse</button>
</div>
```

---

## Validation

### Required field indicator
```tsx
<div className="panel-label">
  Review Title <span style={{ color: "var(--text-danger)" }}>*</span>
</div>
```

### Field error
```tsx
<input
  className="panel-input panel-input-full"
  style={{ borderColor: hasError ? "var(--error-color)" : undefined, marginBottom: hasError ? 4 : 8 }}
  value={value}
  onChange={e => setValue(e.target.value)}
/>
{hasError && (
  <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-danger)", marginBottom: 8 }}>
    This field is required.
  </div>
)}
```

### Disable submit until valid
```tsx
<button
  className="panel-btn panel-btn-primary"
  onClick={handleSubmit}
  disabled={loading || !title.trim()}
>
  {loading ? "Submitting…" : "Submit"}
</button>
```

---

## Stepper / Numeric Control

For model parameters (exploration rate, decay rate):

```tsx
const [rate, setRate] = useState(0.15);

<div className="panel-row" style={{ marginBottom: 8 }}>
  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", minWidth: 130 }}>
    Exploration Rate
  </span>
  <button
    className="panel-btn panel-btn-xs panel-btn-secondary"
    onClick={() => setRate(r => Math.max(0, r - 0.01))}
  >
    −
  </button>
  <span
    className="panel-mono"
    style={{
      fontSize: "var(--font-size-base)",
      fontWeight: "var(--font-semibold)",
      color: "var(--text-info)",
      minWidth: 44,
      textAlign: "center",
    }}
  >
    {rate.toFixed(2)}
  </span>
  <button
    className="panel-btn panel-btn-xs panel-btn-secondary"
    onClick={() => setRate(r => Math.min(1, r + 0.01))}
  >
    +
  </button>
</div>
```

---

## Form Submission Feedback

### Success — inline card
```tsx
{sessionId && (
  <div className="panel-card" style={{ borderLeft: "3px solid var(--success-color)" }}>
    <div style={{ fontWeight: "var(--font-semibold)", marginBottom: 4 }}>Started Successfully</div>
    <div className="panel-label" style={{ marginBottom: 0 }}>Session: {sessionId}</div>
  </div>
)}
```

### Error — panel-error box
```tsx
{error && (
  <div className="panel-error" style={{ marginBottom: 10 }}>
    {error}
    <button onClick={() => setError("")}>✕</button>
  </div>
)}
```

### Loading — button state only (keep form visible)
```tsx
<button className="panel-btn panel-btn-primary" onClick={submit} disabled={loading}>
  {loading ? "Saving…" : "Save"}
</button>
// Don't hide the form or show panel-loading during submission
// User should see the form in case they want to cancel
```

---

## Multi-Step / Sections in One Card

For forms with logical groups:

```tsx
<div className="panel-card" style={{ marginBottom: 10 }}>
  {/* Section 1 */}
  <div style={{ marginBottom: 12 }}>
    <div style={{ fontSize: "var(--font-size-sm)", fontWeight: "var(--font-semibold)", color: "var(--text-secondary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: "0.5px" }}>
      Target
    </div>
    <div className="panel-label">Intent</div>
    <input className="panel-input panel-input-full" style={{ marginBottom: 8 }} ... />
    <div className="panel-label">Files</div>
    <input className="panel-input panel-input-full" style={{ marginBottom: 0 }} ... />
  </div>

  <div className="panel-divider" />

  {/* Section 2 */}
  <div style={{ marginBottom: 12, marginTop: 12 }}>
    <div style={{ fontSize: "var(--font-size-sm)", fontWeight: "var(--font-semibold)", color: "var(--text-secondary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: "0.5px" }}>
      Options
    </div>
    <div className="panel-label">Provider</div>
    <select className="panel-select" style={{ width: "100%", marginBottom: 0 }} ... />
  </div>

  <div className="panel-divider" />

  {/* Actions */}
  <div style={{ display: "flex", gap: 6, justifyContent: "flex-end", marginTop: 12 }}>
    <button className="panel-btn panel-btn-secondary" onClick={handleReset}>Reset</button>
    <button className="panel-btn panel-btn-primary" onClick={handleSubmit} disabled={loading}>
      {loading ? "Running…" : "Run"}
    </button>
  </div>
</div>
```

---

## Rules

### <span class="docs-do" aria-hidden="true"></span>Do
- Place all form fields in a `panel-card`
- Use `panel-label` immediately above every field
- Add `marginBottom: 8` between fields (after inputs)
- Disable submit when required fields are empty
- Keep form visible during submission (update button label only)
- Show success result below the form card (not replace it)
- Show errors via `panel-error` above the form card

### <span class="docs-dont" aria-hidden="true"></span>Don't
```tsx
// Label without class
<label style={{ fontSize: 11, color: "var(--text-secondary)" }}>Field</label>
// → use <div className="panel-label">

// Form outside a card
<div>  // not wrapped in panel-card
  <label>...</label>
  <input ... />
</div>

// Hardcoded inputStyle object
const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 8px", ... };
// → use className="panel-input panel-input-full"

// Submit without loading state
<button onClick={submit}>Submit</button>  // no disabled, no label change
```
