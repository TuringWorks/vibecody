/**
 * WebCrawlerPanel — web crawling and sitemap parsing utilities.
 *
 * Tabs: Crawl (URL crawling with config), Sitemap (sitemap/robots.txt parsing)
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "crawl" | "sitemap";

interface CrawlConfig {
  url: string;
  maxPages: number;
  maxDepth: number;
  delayMs: number;
  respectRobots: boolean;
  followExternal: boolean;
}

interface CrawlResult {
  url: string;
  status: number;
  contentType: string;
}

export function WebCrawlerPanel() {
  const [tab, setTab] = useState<Tab>("crawl");

  // Crawl state
  const [crawlConfig, setCrawlConfig] = useState<CrawlConfig>({
    url: "",
    maxPages: 50,
    maxDepth: 3,
    delayMs: 500,
    respectRobots: true,
    followExternal: false,
  });
  const [crawlResults, setCrawlResults] = useState<CrawlResult[]>([]);
  const [isCrawling, setIsCrawling] = useState(false);
  const [loadingResults, setLoadingResults] = useState(true);

  // Sitemap state
  const [sitemapUrl, setSitemapUrl] = useState("");
  const [sitemapUrls, setSitemapUrls] = useState<string[]>([]);
  const [robotsUrl, setRobotsUrl] = useState("");
  const [robotsResult, setRobotsResult] = useState("");
  const [isLoadingSitemap, setIsLoadingSitemap] = useState(false);

  // Load previous crawl results on mount
  useEffect(() => {
    const loadResults = async () => {
      setLoadingResults(true);
      try {
        const data = await invoke<CrawlResult[]>("get_crawl_results");
        setCrawlResults(data);
      } catch (err) {
        console.error("Failed to load crawl results:", err);
      } finally {
        setLoadingResults(false);
      }
    };
    loadResults();
  }, []);

  const handleStartCrawl = async () => {
    if (!crawlConfig.url.trim()) return;
    setIsCrawling(true);
    setCrawlResults([]);
    try {
      const results = await invoke<CrawlResult[]>("run_web_crawl", { config: crawlConfig });
      setCrawlResults(results);
    } catch (err) {
      console.error("Failed to run crawl:", err);
    } finally {
      setIsCrawling(false);
    }
  };

  const handleParseSitemap = async () => {
    if (!sitemapUrl.trim()) return;
    setIsLoadingSitemap(true);
    try {
      const urls = await invoke<string[]>("parse_sitemap", { url: sitemapUrl });
      setSitemapUrls(urls);
    } catch (err) {
      console.error("Failed to parse sitemap:", err);
    } finally {
      setIsLoadingSitemap(false);
    }
  };

  const handleCheckRobots = async () => {
    if (!robotsUrl.trim()) return;
    try {
      const result = await invoke<string>("check_robots_txt", { url: robotsUrl });
      setRobotsResult(result);
    } catch (err) {
      console.error("Failed to check robots.txt:", err);
    }
  };

  const tabs: { key: Tab; label: string }[] = [
    { key: "crawl", label: "Crawl" },
    { key: "sitemap", label: "Sitemap" },
  ];

  const inputStyle: React.CSSProperties = {
    width: "100%",
    background: "var(--bg-secondary)",
    border: "1px solid var(--border)",
    borderRadius: 4,
    color: "var(--text-primary)",
    padding: "6px 8px",
    fontSize: 12,
    boxSizing: "border-box",
  };

  const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", display: "block", marginBottom: 4 };

  const btnPrimary: React.CSSProperties = {
    background: "var(--accent)",
    color: "var(--btn-primary-fg)",
    border: "none",
    borderRadius: 4,
    padding: "8px 16px",
    cursor: "pointer",
    fontSize: 12,
    fontWeight: 600,
  };

  const btnSecondary: React.CSSProperties = {
    background: "var(--bg-secondary)",
    color: "var(--text-primary)",
    border: "1px solid var(--border)",
    borderRadius: 4,
    padding: "8px 16px",
    cursor: "pointer",
    fontSize: 12,
    fontWeight: 600,
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, background: "var(--bg-primary)", color: "var(--text-primary)" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", borderBottom: "1px solid var(--border)", background: "var(--bg-secondary)" }}>
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            style={{
              padding: "8px 16px",
              background: tab === t.key ? "var(--bg-primary)" : "transparent",
              border: "none",
              borderBottom: tab === t.key ? "2px solid var(--accent)" : "2px solid transparent",
              color: tab === t.key ? "var(--text-primary)" : "var(--text-secondary)",
              cursor: "pointer",
              fontSize: 12,
              fontWeight: tab === t.key ? 600 : 400,
            }}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
        {tab === "crawl" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {/* URL input */}
            <div>
              <label style={labelStyle}>Start URL</label>
              <input
                value={crawlConfig.url}
                onChange={(e) => setCrawlConfig((c) => ({ ...c, url: e.target.value }))}
                placeholder="https://example.com"
                style={inputStyle}
              />
            </div>

            {/* Numeric configs */}
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12 }}>
              <div>
                <label style={labelStyle}>Max Pages</label>
                <input
                  type="number"
                  min={1}
                  max={1000}
                  value={crawlConfig.maxPages}
                  onChange={(e) => setCrawlConfig((c) => ({ ...c, maxPages: Number(e.target.value) }))}
                  style={inputStyle}
                />
              </div>
              <div>
                <label style={labelStyle}>Max Depth</label>
                <input
                  type="number"
                  min={1}
                  max={20}
                  value={crawlConfig.maxDepth}
                  onChange={(e) => setCrawlConfig((c) => ({ ...c, maxDepth: Number(e.target.value) }))}
                  style={inputStyle}
                />
              </div>
              <div>
                <label style={labelStyle}>Delay (ms)</label>
                <input
                  type="number"
                  min={0}
                  max={10000}
                  step={100}
                  value={crawlConfig.delayMs}
                  onChange={(e) => setCrawlConfig((c) => ({ ...c, delayMs: Number(e.target.value) }))}
                  style={inputStyle}
                />
              </div>
            </div>

            {/* Toggles */}
            <div style={{ display: "flex", gap: 24 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <button
                  onClick={() => setCrawlConfig((c) => ({ ...c, respectRobots: !c.respectRobots }))}
                  style={{
                    width: 40,
                    height: 22,
                    borderRadius: 11,
                    border: "none",
                    background: crawlConfig.respectRobots ? "var(--accent)" : "var(--bg-secondary)",
                    cursor: "pointer",
                    position: "relative",
                  }}
                >
                  <div style={{
                    width: 16, height: 16, borderRadius: "50%", background: "white",
                    position: "absolute", top: 3,
                    left: crawlConfig.respectRobots ? 21 : 3,
                    transition: "left 0.15s ease",
                  }} />
                </button>
                <span style={{ fontSize: 12 }}>Respect robots.txt</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <button
                  onClick={() => setCrawlConfig((c) => ({ ...c, followExternal: !c.followExternal }))}
                  style={{
                    width: 40,
                    height: 22,
                    borderRadius: 11,
                    border: "none",
                    background: crawlConfig.followExternal ? "var(--accent)" : "var(--bg-secondary)",
                    cursor: "pointer",
                    position: "relative",
                  }}
                >
                  <div style={{
                    width: 16, height: 16, borderRadius: "50%", background: "white",
                    position: "absolute", top: 3,
                    left: crawlConfig.followExternal ? 21 : 3,
                    transition: "left 0.15s ease",
                  }} />
                </button>
                <span style={{ fontSize: 12 }}>Follow external links</span>
              </div>
            </div>

            {/* Start button */}
            <button
              onClick={handleStartCrawl}
              disabled={isCrawling || !crawlConfig.url.trim()}
              style={{ ...btnPrimary, opacity: isCrawling || !crawlConfig.url.trim() ? 0.5 : 1 }}
            >
              {isCrawling ? "Crawling..." : "Start Crawl"}
            </button>

            {/* Results table */}
            {loadingResults ? (
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 8 }}>Loading previous results...</div>
            ) : crawlResults.length > 0 ? (
              <div style={{ marginTop: 8 }}>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>{crawlResults.length} page(s) crawled</div>
                <div style={{ overflowX: "auto" }}>
                  <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12, fontFamily: "var(--font-mono)" }}>
                    <thead>
                      <tr style={{ background: "var(--bg-secondary)" }}>
                        <th style={{ padding: "6px 8px", textAlign: "left", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>URL</th>
                        <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600, width: 60 }}>Status</th>
                        <th style={{ padding: "6px 8px", textAlign: "left", borderBottom: "1px solid var(--border)", fontWeight: 600, width: 120 }}>Content-Type</th>
                      </tr>
                    </thead>
                    <tbody>
                      {crawlResults.map((r, i) => (
                        <tr key={i} style={{ background: i % 2 === 0 ? "transparent" : "var(--bg-secondary)" }}>
                          <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", maxWidth: 400, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.url}</td>
                          <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center", color: r.status === 200 ? "var(--success-color)" : r.status === 301 ? "var(--warning-color)" : "var(--error-color)" }}>{r.status}</td>
                          <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)" }}>{r.contentType}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            ) : !isCrawling ? (
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 8 }}>No crawl results yet. Enter a URL and start a crawl.</div>
            ) : null}
          </div>
        )}

        {tab === "sitemap" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {/* Sitemap section */}
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              <div style={{ fontSize: 13, fontWeight: 600 }}>Parse Sitemap</div>
              <div>
                <label style={labelStyle}>Sitemap URL</label>
                <input
                  value={sitemapUrl}
                  onChange={(e) => setSitemapUrl(e.target.value)}
                  placeholder="https://example.com/sitemap.xml"
                  style={inputStyle}
                />
              </div>
              <button
                onClick={handleParseSitemap}
                disabled={isLoadingSitemap || !sitemapUrl.trim()}
                style={{ ...btnPrimary, alignSelf: "flex-start", opacity: isLoadingSitemap || !sitemapUrl.trim() ? 0.5 : 1 }}
              >
                {isLoadingSitemap ? "Parsing..." : "Parse Sitemap"}
              </button>
              {sitemapUrls.length > 0 && (
                <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border)", borderRadius: 4, padding: 12, maxHeight: 200, overflow: "auto" }}>
                  <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6 }}>{sitemapUrls.length} URLs found</div>
                  {sitemapUrls.map((u, i) => (
                    <div key={i} style={{ fontSize: 12, fontFamily: "var(--font-mono)", padding: "2px 0", borderBottom: i < sitemapUrls.length - 1 ? "1px solid var(--border)" : "none" }}>
                      {u}
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div style={{ borderTop: "1px solid var(--border)", paddingTop: 16 }} />

            {/* Robots.txt section */}
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              <div style={{ fontSize: 13, fontWeight: 600 }}>Check robots.txt</div>
              <div>
                <label style={labelStyle}>robots.txt URL</label>
                <input
                  value={robotsUrl}
                  onChange={(e) => setRobotsUrl(e.target.value)}
                  placeholder="https://example.com/robots.txt"
                  style={inputStyle}
                />
              </div>
              <button
                onClick={handleCheckRobots}
                disabled={!robotsUrl.trim()}
                style={{ ...btnSecondary, alignSelf: "flex-start", opacity: !robotsUrl.trim() ? 0.5 : 1 }}
              >
                Check robots.txt
              </button>
              {robotsResult && (
                <pre style={{ background: "var(--bg-secondary)", border: "1px solid var(--border)", borderRadius: 4, padding: 12, fontSize: 12, fontFamily: "var(--font-mono)", margin: 0, whiteSpace: "pre-wrap", color: "var(--text-primary)" }}>
                  {robotsResult}
                </pre>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
