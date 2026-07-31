import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Globe, ExternalLink } from "lucide-react";

interface BrowserViewProps {
  onClose: () => void;
}

/**
 * The Browser quick-action: open a website in its own webview window.
 *
 * A separate window rather than a pane inside the shell, for a concrete reason:
 * the app's CSP is `default-src 'self'`, so a remote page cannot be framed in
 * here at all — and loosening that to embed arbitrary web content next to the
 * conversation would trade a real boundary for a cosmetic one. The spawned
 * window is not covered by the app's Tauri capability, so the page has no
 * access to any command; it is a browser tab, not part of VibeDesk.
 */
export function BrowserView({ onClose }: BrowserViewProps) {
  const [url, setUrl] = useState("");
  const [opened, setOpened] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function open() {
    const target = url.trim();
    if (!target) return;
    setError(null);
    try {
      // The daemon-side normaliser decides what's openable; showing its message
      // verbatim means the rules can't drift from what actually gets enforced.
      const resolved = await invoke<string>("open_browser", { url: target });
      setOpened((prev) => [resolved, ...prev.filter((u) => u !== resolved)].slice(0, 8));
      setUrl("");
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="vx-skills">
      <div className="vx-skills__head">
        <Globe size={14} />
        <span>Browser</span>
        <div className="vx-skills__spacer" />
        <button className="vx-icon-btn" aria-label="Close browser" onClick={onClose}>
          <X size={14} />
        </button>
      </div>

      <div className="vx-auto__new">
        <input
          className="vx-auto__input"
          placeholder="example.com"
          value={url}
          autoFocus
          spellCheck={false}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              open();
            }
          }}
        />
        <button className="vx-trash__btn" onClick={open} disabled={!url.trim()}>
          Open
        </button>
      </div>
      <div className="vx-auto__hint">
        Opens in a separate window. http and https only — the page runs with no
        access to VibeDesk.
      </div>
      {error && <div className="vx-auto__error">{error}</div>}

      <div className="vx-plugins__body">
        {opened.length === 0 ? (
          <div className="vx-files__empty">Nothing opened yet this session.</div>
        ) : (
          <>
            <div className="vx-pill-menu__group">Opened this session</div>
            <ul className="vx-auto__list">
              {opened.map((u) => (
                <li key={u} className="vx-auto__row">
                  <span className="vx-auto__prompt" title={u}>
                    {u}
                  </span>
                  <button
                    className="vx-auto__stop"
                    onClick={() => invoke("open_browser", { url: u }).catch((e) => setError(String(e)))}
                    title="Open again"
                  >
                    <ExternalLink size={11} /> Reopen
                  </button>
                </li>
              ))}
            </ul>
          </>
        )}
      </div>
    </div>
  );
}
