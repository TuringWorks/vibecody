import { useState, useMemo, useCallback } from "react";

type CharInfo = { char: string; cp: number; name: string };

// Unicode block definitions [start, end, label]
const BLOCKS: [number, number, string][] = [
  [0x0020, 0x007E, "Basic Latin (printable)"],
  [0x00A0, 0x00FF, "Latin-1 Supplement"],
  [0x0100, 0x017F, "Latin Extended-A"],
  [0x0180, 0x024F, "Latin Extended-B"],
  [0x0370, 0x03FF, "Greek & Coptic"],
  [0x0400, 0x04FF, "Cyrillic"],
  [0x0600, 0x06FF, "Arabic"],
  [0x0900, 0x097F, "Devanagari"],
  [0x3040, 0x309F, "Hiragana"],
  [0x30A0, 0x30FF, "Katakana"],
  [0x4E00, 0x4EFF, "CJK Unified (first 256)"],
  [0x2190, 0x21FF, "Arrows"],
  [0x2200, 0x22FF, "Mathematical Operators"],
  [0x2300, 0x23FF, "Miscellaneous Technical"],
  [0x2600, 0x26FF, "Miscellaneous Symbols"],
  [0x2700, 0x27BF, "Dingbats"],
  [0x1F300, 0x1F3FF, "Misc Symbols & Pictographs"],
  [0x1F400, 0x1F4FF, "Emoticons & Transport"],
  [0x1F600, 0x1F64F, "Emoticons (faces)"],
  [0x1F680, 0x1F6FF, "Transport & Map"],
];

// Named chars for search — a curated subset
const NAMED: Record<number, string> = {
  0x0020: "SPACE", 0x0021: "EXCLAMATION MARK", 0x0022: "QUOTATION MARK",
  0x0023: "NUMBER SIGN", 0x0024: "DOLLAR SIGN", 0x0025: "PERCENT SIGN",
  0x0026: "AMPERSAND", 0x0027: "APOSTROPHE", 0x0028: "LEFT PARENTHESIS",
  0x0029: "RIGHT PARENTHESIS", 0x002A: "ASTERISK", 0x002B: "PLUS SIGN",
  0x002C: "COMMA", 0x002D: "HYPHEN-MINUS", 0x002E: "FULL STOP",
  0x002F: "SOLIDUS", 0x003A: "COLON", 0x003B: "SEMICOLON",
  0x003C: "LESS-THAN SIGN", 0x003D: "EQUALS SIGN", 0x003E: "GREATER-THAN SIGN",
  0x003F: "QUESTION MARK", 0x0040: "COMMERCIAL AT", 0x005B: "LEFT SQUARE BRACKET",
  0x005C: "REVERSE SOLIDUS", 0x005D: "RIGHT SQUARE BRACKET", 0x005E: "CIRCUMFLEX ACCENT",
  0x005F: "LOW LINE", 0x0060: "GRAVE ACCENT", 0x007B: "LEFT CURLY BRACKET",
  0x007C: "VERTICAL LINE", 0x007D: "RIGHT CURLY BRACKET", 0x007E: "TILDE",
  0x00A9: "COPYRIGHT SIGN", 0x00AE: "REGISTERED SIGN", 0x00B0: "DEGREE SIGN",
  0x00B1: "PLUS-MINUS SIGN", 0x00B5: "MICRO SIGN", 0x00D7: "MULTIPLICATION SIGN",
  0x00F7: "DIVISION SIGN", 0x2014: "EM DASH", 0x2013: "EN DASH",
  0x2018: "LEFT SINGLE QUOTATION MARK", 0x2019: "RIGHT SINGLE QUOTATION MARK",
  0x201C: "LEFT DOUBLE QUOTATION MARK", 0x201D: "RIGHT DOUBLE QUOTATION MARK",
  0x2026: "HORIZONTAL ELLIPSIS", 0x2122: "TRADE MARK SIGN",
  0x2190: "LEFTWARDS ARROW", 0x2191: "UPWARDS ARROW",
  0x2192: "RIGHTWARDS ARROW", 0x2193: "DOWNWARDS ARROW",
  0x21D0: "LEFTWARDS DOUBLE ARROW", 0x21D2: "RIGHTWARDS DOUBLE ARROW",
  0x2200: "FOR ALL", 0x2203: "THERE EXISTS", 0x2205: "EMPTY SET",
  0x2207: "NABLA", 0x2208: "ELEMENT OF", 0x220F: "N-ARY PRODUCT",
  0x2211: "N-ARY SUMMATION", 0x221A: "SQUARE ROOT", 0x221E: "INFINITY",
  0x2227: "LOGICAL AND", 0x2228: "LOGICAL OR", 0x2229: "INTERSECTION",
  0x222A: "UNION", 0x222B: "INTEGRAL", 0x2260: "NOT EQUAL TO",
  0x2264: "LESS-THAN OR EQUAL TO", 0x2265: "GREATER-THAN OR EQUAL TO",
  0x2713: "CHECK MARK", 0x2714: "HEAVY CHECK MARK", 0x2717: "BALLOT X",
  0x2718: "HEAVY BALLOT X", 0x25A0: "BLACK SQUARE", 0x25B6: "BLACK RIGHT-POINTING TRIANGLE",
  0x2665: "BLACK HEART SUIT", 0x2764: "HEAVY BLACK HEART",
  0x1F600: "GRINNING FACE", 0x1F601: "GRINNING FACE WITH SMILING EYES",
  0x1F602: "FACE WITH TEARS OF JOY", 0x1F4A9: "PILE OF POO",
  0x1F525: "FIRE", 0x1F680: "ROCKET", 0x1F4BB: "PERSONAL COMPUTER",
  0x1F4A1: "ELECTRIC LIGHT BULB", 0x2B50: "WHITE MEDIUM STAR",
};

function cpToStr(cp: number): string {
  return String.fromCodePoint(cp);
}

function cpToHex(cp: number): string {
  return cp.toString(16).toUpperCase().padStart(4, "0");
}

function htmlEntity(cp: number): string {
  return `&#x${cpToHex(cp)};`;
}

function cssEscape(cp: number): string {
  return `\\${cpToHex(cp)}`;
}

function jsEscape(cp: number): string {
  if (cp > 0xFFFF) {
    // surrogate pair
    const hi = Math.floor((cp - 0x10000) / 0x400) + 0xD800;
    const lo = ((cp - 0x10000) % 0x400) + 0xDC00;
    return `\\u${hi.toString(16).toUpperCase()}\\u${lo.toString(16).toUpperCase()}`;
  }
  return `\\u${cpToHex(cp)}`;
}

function charName(cp: number): string {
  if (NAMED[cp]) return NAMED[cp];
  if (cp >= 0x41 && cp <= 0x5A) return `LATIN CAPITAL LETTER ${cpToStr(cp)}`;
  if (cp >= 0x61 && cp <= 0x7A) return `LATIN SMALL LETTER ${cpToStr(cp).toUpperCase()}`;
  if (cp >= 0x30 && cp <= 0x39) return `DIGIT ${cpToStr(cp)}`;
  return `U+${cpToHex(cp)}`;
}

interface CharGridProps {
  chars: CharInfo[];
  selected: CharInfo | null;
  onSelect: (info: CharInfo) => void;
}

function CharGrid({ chars, selected, onSelect }: CharGridProps) {
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 3, padding: 8 }}>
      {chars.map((info) => (
        <button key={info.cp} onClick={() => onSelect(info)} title={`${info.name} U+${cpToHex(info.cp)}`}
          style={{
            width: 38, height: 38, display: "flex", alignItems: "center", justifyContent: "center",
            fontSize: 18, background: selected?.cp === info.cp ? "var(--accent-color)" : "var(--bg-secondary)",
            border: `1px solid ${selected?.cp === info.cp ? "var(--accent-color)" : "var(--border-color)"}`,
            borderRadius: 4, cursor: "pointer", color: "var(--text-primary)",
            transition: "background 0.1s",
          }}>
          {info.char}
        </button>
      ))}
    </div>
  );
}

interface InfoPanelProps {
  info: CharInfo;
  toggleFavorite: (info: CharInfo) => void;
  isFav: (cp: number) => boolean;
}

function InfoPanel({ info, toggleFavorite, isFav }: InfoPanelProps) {
  return (
    <div style={{ padding: "12px 16px", borderTop: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 10 }}>
        <div style={{ fontSize: 40, lineHeight: 1, width: 52, height: 52, display: "flex", alignItems: "center", justifyContent: "center", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 6 }}>
          {info.char}
        </div>
        <div>
          <div style={{ fontWeight: 600, fontSize: 14 }}>{info.name}</div>
          <div style={{ color: "var(--text-secondary)", fontSize: 12, fontFamily: "var(--font-mono)" }}>U+{cpToHex(info.cp)} · dec {info.cp}</div>
        </div>
        <button onClick={() => toggleFavorite(info)}
          style={{ marginLeft: "auto", background: "none", border: "1px solid var(--border-color)", borderRadius: 4, padding: "4px 8px", cursor: "pointer", color: isFav(info.cp) ? "#f5a623" : "var(--text-secondary)", fontSize: 16 }}>
          {isFav(info.cp) ? "★" : "☆"}
        </button>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))", gap: 6 }}>
        {[
          { label: "Character", value: info.char },
          { label: "HTML entity", value: htmlEntity(info.cp) },
          { label: "CSS escape", value: cssEscape(info.cp) },
          { label: "JS escape", value: jsEscape(info.cp) },
          { label: "UTF-8 hex", value: Array.from(new TextEncoder().encode(info.char)).map(b => b.toString(16).toUpperCase().padStart(2, "0")).join(" ") },
          { label: "Percent-encoded", value: encodeURIComponent(info.char) },
        ].map(({ label, value }) => (
          <div key={label} style={{ background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, padding: "5px 8px", display: "flex", justifyContent: "space-between", alignItems: "center", gap: 6 }}>
            <div>
              <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>{label}</div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>{value}</div>
            </div>
            <button onClick={() => navigator.clipboard.writeText(value)}
              style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", fontSize: 11, flexShrink: 0 }}>⎘</button>
          </div>
        ))}
      </div>
    </div>
  );
}

export function UnicodePanel() {
  const [blockIdx, setBlockIdx] = useState(0);
  const [selected, setSelected] = useState<CharInfo | null>(null);
  const [search, setSearch] = useState("");
  const [favorites, setFavorites] = useState<CharInfo[]>([]);
  const [tab, setTab] = useState<"browse" | "search" | "favorites" | "input">("browse");
  const [inputText, setInputText] = useState("");

  const blockChars = useMemo((): CharInfo[] => {
    const [start, end] = BLOCKS[blockIdx];
    const out: CharInfo[] = [];
    for (let cp = start; cp <= end; cp++) {
      try {
        const char = cpToStr(cp);
        if (char) out.push({ char, cp, name: charName(cp) });
      } catch { /* skip invalid */ }
    }
    return out;
  }, [blockIdx]);

  const searchResults = useMemo((): CharInfo[] => {
    if (!search.trim()) return [];
    const q = search.trim().toUpperCase();
    // Try as code point first (U+XXXX or hex)
    const hexMatch = q.replace(/^U\+/, "");
    if (/^[0-9A-F]{2,6}$/.test(hexMatch)) {
      const cp = parseInt(hexMatch, 16);
      if (cp >= 0x20 && cp <= 0x10FFFF) {
        try {
          return [{ char: cpToStr(cp), cp, name: charName(cp) }];
        } catch { return []; }
      }
    }
    // Search by name across all blocks
    const results: CharInfo[] = [];
    for (const [start, end] of BLOCKS) {
      for (let cp = start; cp <= end && results.length < 120; cp++) {
        const name = charName(cp);
        if (name.toUpperCase().includes(q)) {
          try { results.push({ char: cpToStr(cp), cp, name }); } catch { /* skip */ }
        }
      }
    }
    return results;
  }, [search]);

  const toggleFavorite = useCallback((info: CharInfo) => {
    setFavorites(prev =>
      prev.some(f => f.cp === info.cp) ? prev.filter(f => f.cp !== info.cp) : [...prev, info]
    );
  }, []);

  const isFav = (cp: number) => favorites.some(f => f.cp === cp);

  const inputAnalysis = useMemo((): CharInfo[] => {
    if (!inputText) return [];
    const out: CharInfo[] = [];
    for (const ch of inputText) {
      const cp = ch.codePointAt(0)!;
      out.push({ char: ch, cp, name: charName(cp) });
    }
    return out;
  }, [inputText]);

  const TABS = [
    { id: "browse" as const, label: "Browse" },
    { id: "search" as const, label: "Search" },
    { id: "favorites" as const, label: `Favorites (${favorites.length})` },
    { id: "input" as const, label: "Analyze" },
  ];

  return (
    <div className="panel-container" style={{ fontSize: 13, color: "var(--text-primary)" }}>
      {/* Sub-tab bar */}
      <div className="panel-tab-bar">
        {TABS.map(t => (
          <button key={t.id} onClick={() => setTab(t.id)} className={`panel-tab ${tab === t.id ? "active" : ""}`}>
            {t.label}
          </button>
        ))}
      </div>

      {/* Browse */}
      {tab === "browse" && (
        <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
          {/* Block list */}
          <div style={{ width: 190, borderRight: "1px solid var(--border-color)", overflowY: "auto", flexShrink: 0 }}>
            {BLOCKS.map(([, , label], i) => (
              <button key={i} onClick={() => { setBlockIdx(i); setSelected(null); }}
                style={{ display: "block", width: "100%", padding: "7px 10px", border: "none", textAlign: "left", background: blockIdx === i ? "rgba(var(--accent-rgb,99,102,241),0.15)" : "none", color: blockIdx === i ? "var(--accent-color)" : "var(--text-secondary)", cursor: "pointer", fontSize: 11, borderLeft: blockIdx === i ? "3px solid var(--accent-color)" : "3px solid transparent" }}>
                {label}
              </button>
            ))}
          </div>
          {/* Grid + info */}
          <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
            <div style={{ flex: 1, overflowY: "auto" }}>
              <CharGrid chars={blockChars} selected={selected} onSelect={setSelected} />
            </div>
            {selected && <InfoPanel info={selected} toggleFavorite={toggleFavorite} isFav={isFav} />}
          </div>
        </div>
      )}

      {/* Search */}
      {tab === "search" && (
        <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
          <div style={{ padding: 12, borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}>
            <input value={search} onChange={e => setSearch(e.target.value)} placeholder="Name (e.g. ARROW), U+2192, or hex 2192"
              autoFocus
              className="panel-input panel-input-full" />
            {search && <div style={{ marginTop: 6, fontSize: 11, color: "var(--text-secondary)" }}>{searchResults.length} results</div>}
          </div>
          <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
            <div style={{ flex: 1, overflowY: "auto" }}>
              {searchResults.length > 0 ? (
                <CharGrid chars={searchResults} selected={selected} onSelect={setSelected} />
              ) : search ? (
                <div style={{ padding: 24, color: "var(--text-secondary)", textAlign: "center" }}>No characters found</div>
              ) : (
                <div style={{ padding: 24, color: "var(--text-secondary)", textAlign: "center" }}>Type a character name or code point above</div>
              )}
            </div>
            {selected && <InfoPanel info={selected} toggleFavorite={toggleFavorite} isFav={isFav} />}
          </div>
        </div>
      )}

      {/* Favorites */}
      {tab === "favorites" && (
        <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
          <div style={{ flex: 1, overflowY: "auto" }}>
            {favorites.length === 0 ? (
              <div style={{ padding: 32, textAlign: "center", color: "var(--text-secondary)" }}>
                <div style={{ fontSize: 32, marginBottom: 8 }}>☆</div>
                No favorites yet — click ☆ in the info panel while browsing
              </div>
            ) : (
              <>
                <div style={{ padding: "8px 12px", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{favorites.length} saved</span>
                  <button onClick={() => navigator.clipboard.writeText(favorites.map(f => f.char).join(""))}
                    style={{ padding: "3px 8px", background: "none", border: "1px solid var(--border-color)", borderRadius: 4, cursor: "pointer", fontSize: 11, color: "var(--text-primary)" }}>
                    Copy all
                  </button>
                </div>
                <CharGrid chars={favorites} selected={selected} onSelect={setSelected} />
              </>
            )}
          </div>
          {selected && <InfoPanel info={selected} toggleFavorite={toggleFavorite} isFav={isFav} />}
        </div>
      )}

      {/* Input Analyzer */}
      {tab === "input" && (
        <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
          <div style={{ padding: 12, borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}>
            <textarea value={inputText} onChange={e => setInputText(e.target.value)} placeholder="Paste or type text to analyze each character…"
              className="panel-input panel-textarea panel-input-full" style={{ height: 72, resize: "none" }} />
            {inputText && <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>{[...inputText].length} code points · {new TextEncoder().encode(inputText).length} UTF-8 bytes</div>}
          </div>
          <div style={{ flex: 1, overflowY: "auto" }}>
            {inputAnalysis.length > 0 && (
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                <thead>
                  <tr>
                    {["Char", "Code Point", "Name", "HTML Entity", "UTF-8"].map(h => (
                      <th key={h} style={{ padding: "5px 10px", textAlign: "left", borderBottom: "2px solid var(--accent-blue)", color: "var(--text-secondary)", fontSize: 11 }}>{h}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {inputAnalysis.map((info, i) => (
                    <tr key={i} style={{ background: i % 2 === 0 ? "transparent" : "rgba(255,255,255,0.02)", cursor: "pointer" }} onClick={() => setSelected(info)}>
                      <td style={{ padding: "4px 10px", fontSize: 20, fontFamily: "var(--font-mono)" }}>{info.char}</td>
                      <td style={{ padding: "4px 10px", fontFamily: "var(--font-mono)", color: "var(--accent-color)" }}>U+{cpToHex(info.cp)}</td>
                      <td style={{ padding: "4px 10px", color: "var(--text-secondary)" }}>{info.name}</td>
                      <td style={{ padding: "4px 10px", fontFamily: "var(--font-mono)" }}>{htmlEntity(info.cp)}</td>
                      <td style={{ padding: "4px 10px", fontFamily: "var(--font-mono)", color: "var(--text-secondary)" }}>{Array.from(new TextEncoder().encode(info.char)).map(b => b.toString(16).padStart(2, "0").toUpperCase()).join(" ")}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
          {selected && <InfoPanel info={selected} toggleFavorite={toggleFavorite} isFav={isFav} />}
        </div>
      )}
    </div>
  );
}
