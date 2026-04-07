---
layout: page
title: Tabs — Design System
permalink: /design-system/components/tabs/
---

# Tabs

Tab bars for switching between panel views.

---

## Panel Tabs

```tsx
const [tab, setTab] = useState("overview");

<div className="panel-tab-bar">
  {["overview", "details", "history"].map(t => (
    <button
      key={t}
      className={`panel-tab ${tab === t ? "active" : ""}`}
      onClick={() => setTab(t)}
    >
      {t.charAt(0).toUpperCase() + t.slice(1)}
    </button>
  ))}
</div>

{tab === "overview" && <div>...</div>}
{tab === "details"  && <div>...</div>}
{tab === "history"  && <div>...</div>}
```

---

## Tab Content

Place tab content directly after `.panel-tab-bar`, inside `.panel-body`:

```tsx
<div className="panel-body">
  <div className="panel-tab-bar">
    <button className={`panel-tab ${tab === "a" ? "active" : ""}`} onClick={() => setTab("a")}>A</button>
    <button className={`panel-tab ${tab === "b" ? "active" : ""}`} onClick={() => setTab("b")}>B</button>
  </div>

  {tab === "a" && (
    <div style={{ padding: "var(--space-3) 0" }}>
      {/* Tab A content */}
    </div>
  )}
  {tab === "b" && (
    <div style={{ padding: "var(--space-3) 0" }}>
      {/* Tab B content */}
    </div>
  )}
</div>
```

---

## Rules

- Use `useState` for tab state — no router needed
- Always use `panel-tab` + `active` class pattern — never inline active styles
- At most 6 tabs per bar; beyond that, use a select or sidebar navigation
