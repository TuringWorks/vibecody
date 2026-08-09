import { useState } from "react";
import { ShieldAlert, Plus, X } from "lucide-react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { SandboxPolicy } from "../lib/sandbox";
import { describe, isLocked } from "../lib/sandbox";

interface SandboxSettingsProps {
  value: SandboxPolicy;
  onChange: (next: SandboxPolicy) => void;
  onClose: () => void;
}

const AXES: { key: keyof SandboxPolicy; label: string; hint: string }[] = [
  { key: "readOutside", label: "Read files", hint: "Open files outside the workspace" },
  { key: "writeOutside", label: "Write files", hint: "Create and modify them" },
  {
    key: "execOutside",
    label: "Run commands unconfined",
    hint: "Off runs shell commands under OS-level sandboxing",
  },
  { key: "network", label: "Network", hint: "Web search, URL fetch, outbound requests" },
];

/**
 * Per-axis grants for Sandbox mode.
 *
 * Everything defaults to off, and each axis is separate because "look at my
 * other repo" and "edit my other repo" are different decisions. Roots narrow it
 * further: an allow-list confines outside access to those directories, and a
 * deny-list wins over it.
 *
 * The note about credentials is not decoration — it is the one guarantee the
 * daemon makes that no setting here can undo, and a permissions screen that
 * doesn't say what it *cannot* do invites the user to assume the worst.
 */
export function SandboxSettings({ value, onChange, onClose }: SandboxSettingsProps) {
  const [draft, setDraft] = useState<SandboxPolicy>(value);

  function setAxis(key: keyof SandboxPolicy, on: boolean) {
    setDraft((p) => ({ ...p, [key]: on }));
  }

  async function pickRoot(kind: "allowRoots" | "denyRoots") {
    const picked = await openDialog({ directory: true, multiple: false });
    if (typeof picked === "string" && picked) {
      setDraft((p) => (p[kind].includes(picked) ? p : { ...p, [kind]: [...p[kind], picked] }));
    }
  }

  function removeRoot(kind: "allowRoots" | "denyRoots", path: string) {
    setDraft((p) => ({ ...p, [kind]: p[kind].filter((x) => x !== path) }));
  }

  return (
    <div className="vx-sandbox">
      <div className="vx-sandbox__head">
        <span className="vx-sandbox__title">Sandbox access</span>
        <button className="vx-icon-btn" aria-label="Close sandbox settings" onClick={onClose}>
          <X size={15} />
        </button>
      </div>

      <p className="vx-sandbox__lede">
        Sandbox mode runs the agent with the access you grant here. Inside the workspace nothing
        changes — these settings only widen what it can reach outside.
      </p>

      <ul className="vx-sandbox__axes">
        {AXES.map((a) => (
          <li key={a.key} className="vx-sandbox__axis">
            <label className="vx-sandbox__toggle">
              <input
                type="checkbox"
                checked={Boolean(draft[a.key])}
                onChange={(e) => setAxis(a.key, e.target.checked)}
              />
              <span className="vx-sandbox__axis-label">{a.label}</span>
            </label>
            <span className="vx-sandbox__axis-hint">{a.hint}</span>
          </li>
        ))}
      </ul>

      {(["allowRoots", "denyRoots"] as const).map((kind) => (
        <section key={kind} className="vx-sandbox__roots">
          <div className="vx-sandbox__roots-head">
            <span className="vx-sandbox__roots-title">
              {kind === "allowRoots" ? "Allowed folders" : "Denied folders"}
            </span>
            <button className="vx-sandbox__add" onClick={() => pickRoot(kind)}>
              <Plus size={12} /> Add
            </button>
          </div>
          <p className="vx-sandbox__roots-hint">
            {kind === "allowRoots"
              ? "When set, outside access is confined to these. Empty means anywhere the toggles allow."
              : "Never reachable, even if an allowed folder covers them."}
          </p>
          {draft[kind].length > 0 && (
            <ul className="vx-sandbox__root-list">
              {draft[kind].map((path) => (
                <li key={path} className="vx-sandbox__root" title={path}>
                  <span className="vx-sandbox__root-path">{path}</span>
                  <button
                    className="vx-icon-btn"
                    aria-label={`Remove ${path}`}
                    onClick={() => removeRoot(kind, path)}
                  >
                    <X size={12} />
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      ))}

      <p className="vx-sandbox__guard">
        <ShieldAlert size={13} />
        <span>
          Credential paths — <code>~/.ssh</code>, <code>~/.aws</code>, <code>~/.vibecli</code>,
          private keys, tokens — stay blocked no matter what is set here, including via an allowed
          folder.
        </span>
      </p>

      <div className="vx-sandbox__foot">
        <span className="vx-sandbox__summary">
          {isLocked(draft) ? "Grants nothing — same as Agent mode." : describe(draft)}
        </span>
        <button
          className="vx-sandbox__apply"
          onClick={() => {
            onChange(draft);
            onClose();
          }}
        >
          Apply
        </button>
      </div>
    </div>
  );
}
