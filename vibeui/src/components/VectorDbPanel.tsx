/**
 * VectorDbPanel — vector database management for collections, search, and schema generation.
 *
 * Tabs: Collections (create/list), Search (vector similarity), Schema (provider-specific DDL)
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "collections" | "search" | "schema";
type Metric = "cosine" | "euclidean" | "dot_product" | "manhattan";
type Provider = "qdrant" | "pinecone" | "pgvector" | "milvus" | "weaviate" | "chroma";

interface HnswConfig {
  m: number;
  efConstruction: number;
  efSearch: number;
}

interface Collection {
  name: string;
  dimension: number;
  metric: Metric;
  vectorCount: number;
  hnsw: HnswConfig;
}

interface SearchResult {
  id: string;
  score: number;
  payload: Record<string, string>;
}

export function VectorDbPanel() {
  const [tab, setTab] = useState<Tab>("collections");
  const [loading, setLoading] = useState(true);

  // Collections state
  const [collName, setCollName] = useState("");
  const [collDimension, setCollDimension] = useState(1536);
  const [collMetric, setCollMetric] = useState<Metric>("cosine");
  const [hnsw, setHnsw] = useState<HnswConfig>({ m: 16, efConstruction: 128, efSearch: 64 });
  const [collections, setCollections] = useState<Collection[]>([]);

  // Search state
  const [searchCollection, setSearchCollection] = useState("");
  const [vectorInput, setVectorInput] = useState("");
  const [topK, setTopK] = useState(10);
  const [minScore, setMinScore] = useState(0.0);
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  // Schema state
  const [schemaProvider, setSchemaProvider] = useState<Provider>("qdrant");
  const [schemaOutput, setSchemaOutput] = useState("");

  useEffect(() => {
    let cancelled = false;
    async function loadCollections() {
      setLoading(true);
      try {
        const colls = await invoke<Collection[]>("list_vector_collections");
        if (!cancelled) setCollections(colls);
      } catch (err) {
        console.error("Failed to load vector collections:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    loadCollections();
    return () => { cancelled = true; };
  }, []);

  const handleCreateCollection = async () => {
    if (!collName.trim()) return;
    if (collections.some((c) => c.name === collName)) return;
    try {
      const collection: Collection = {
        name: collName,
        dimension: collDimension,
        metric: collMetric,
        vectorCount: 0,
        hnsw: { ...hnsw },
      };
      await invoke("create_vector_collection", { collection });
      setCollections((prev) => [...prev, collection]);
      setCollName("");
    } catch (err) {
      console.error("Failed to create collection:", err);
    }
  };

  const handleDeleteCollection = async (name: string) => {
    try {
      await invoke("delete_vector_collection", { name });
      setCollections((prev) => prev.filter((c) => c.name !== name));
    } catch (err) {
      console.error("Failed to delete collection:", err);
    }
  };

  const handleSearch = async () => {
    const parts = vectorInput.split(",").map((s) => parseFloat(s.trim())).filter((n) => !isNaN(n));
    if (parts.length === 0) return;
    setIsSearching(true);
    try {
      const results = await invoke<SearchResult[]>("vector_search", {
        collection: searchCollection,
        query: parts,
        topK,
        minScore,
      });
      setSearchResults(results);
    } catch (err) {
      console.error("Failed to search vectors:", err);
    } finally {
      setIsSearching(false);
    }
  };

  const handleGenerateSchema = () => {
    const schemas: Record<Provider, string> = {
      qdrant: `// Qdrant Collection Config (JSON)
{
  "collection_name": "my_collection",
  "vectors": {
    "size": 1536,
    "distance": "Cosine"
  },
  "hnsw_config": {
    "m": 16,
    "ef_construct": 128
  },
  "optimizers_config": {
    "default_segment_number": 2
  }
}`,
      pinecone: `// Pinecone Index Config (JSON)
{
  "name": "my-index",
  "dimension": 1536,
  "metric": "cosine",
  "spec": {
    "serverless": {
      "cloud": "aws",
      "region": "us-east-1"
    }
  }
}`,
      pgvector: `-- pgvector Schema (SQL)
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE embeddings (
  id        BIGSERIAL PRIMARY KEY,
  content   TEXT NOT NULL,
  metadata  JSONB DEFAULT '{}',
  embedding VECTOR(1536)
);

CREATE INDEX ON embeddings
  USING ivfflat (embedding vector_cosine_ops)
  WITH (lists = 100);`,
      milvus: `// Milvus Collection Schema (JSON)
{
  "collection_name": "my_collection",
  "fields": [
    { "name": "id", "data_type": "Int64", "is_primary_key": true, "auto_id": true },
    { "name": "content", "data_type": "VarChar", "max_length": 65535 },
    { "name": "embedding", "data_type": "FloatVector", "dim": 1536 }
  ],
  "index_params": {
    "metric_type": "COSINE",
    "index_type": "HNSW",
    "params": { "M": 16, "efConstruction": 128 }
  }
}`,
      weaviate: `// Weaviate Class Config (JSON)
{
  "class": "Document",
  "vectorizer": "none",
  "vectorIndexConfig": {
    "distance": "cosine",
    "efConstruction": 128,
    "ef": 64,
    "maxConnections": 16
  },
  "properties": [
    { "name": "content", "dataType": ["text"] },
    { "name": "source", "dataType": ["string"] }
  ]
}`,
      chroma: `# Chroma Collection (Python)
import chromadb

client = chromadb.PersistentClient(path="./chroma_db")

collection = client.create_collection(
    name="my_collection",
    metadata={
        "hnsw:space": "cosine",
        "hnsw:construction_ef": 128,
        "hnsw:M": 16,
    },
)`,
    };
    setSchemaOutput(schemas[schemaProvider]);
  };

  const tabs: { key: Tab; label: string }[] = [
    { key: "collections", label: "Collections" },
    { key: "search", label: "Search" },
    { key: "schema", label: "Schema" },
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

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-primary)", color: "var(--text-primary)" }}>
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
        {loading && (
          <div style={{ color: "var(--text-secondary)", fontSize: 12, textAlign: "center", marginTop: 32 }}>Loading...</div>
        )}

        {!loading && tab === "collections" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={{ fontSize: 13, fontWeight: 600 }}>Create Collection</div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
              <div>
                <label style={labelStyle}>Collection Name</label>
                <input value={collName} onChange={(e) => setCollName(e.target.value)} placeholder="my_collection" style={inputStyle} />
              </div>
              <div>
                <label style={labelStyle}>Dimension</label>
                <input type="number" min={1} max={65536} value={collDimension} onChange={(e) => setCollDimension(Number(e.target.value))} style={inputStyle} />
              </div>
            </div>

            <div>
              <label style={labelStyle}>Distance Metric</label>
              <select value={collMetric} onChange={(e) => setCollMetric(e.target.value as Metric)} style={inputStyle}>
                <option value="cosine">Cosine</option>
                <option value="euclidean">Euclidean</option>
                <option value="dot_product">Dot Product</option>
                <option value="manhattan">Manhattan</option>
              </select>
            </div>

            {/* HNSW config */}
            <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border)", borderRadius: 4, padding: 12 }}>
              <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 8, color: "var(--text-secondary)" }}>HNSW Index Config</div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12 }}>
                <div>
                  <label style={labelStyle}>M (max connections)</label>
                  <input type="number" min={4} max={128} value={hnsw.m} onChange={(e) => setHnsw((h) => ({ ...h, m: Number(e.target.value) }))} style={inputStyle} />
                </div>
                <div>
                  <label style={labelStyle}>ef_construction</label>
                  <input type="number" min={16} max={512} value={hnsw.efConstruction} onChange={(e) => setHnsw((h) => ({ ...h, efConstruction: Number(e.target.value) }))} style={inputStyle} />
                </div>
                <div>
                  <label style={labelStyle}>ef_search</label>
                  <input type="number" min={16} max={512} value={hnsw.efSearch} onChange={(e) => setHnsw((h) => ({ ...h, efSearch: Number(e.target.value) }))} style={inputStyle} />
                </div>
              </div>
            </div>

            <button onClick={handleCreateCollection} disabled={!collName.trim()} style={{ ...btnPrimary, alignSelf: "flex-start", opacity: !collName.trim() ? 0.5 : 1 }}>
              Create Collection
            </button>

            {/* Collection list table */}
            {collections.length > 0 && (
              <div style={{ marginTop: 8 }}>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>{collections.length} collection(s)</div>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                  <thead>
                    <tr style={{ background: "var(--bg-secondary)" }}>
                      <th style={{ padding: "6px 8px", textAlign: "left", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Name</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Dim</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Metric</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Vectors</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>HNSW</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600, width: 60 }}></th>
                    </tr>
                  </thead>
                  <tbody>
                    {collections.map((c, i) => (
                      <tr key={c.name} style={{ background: i % 2 === 0 ? "transparent" : "var(--bg-secondary)" }}>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", fontFamily: "var(--font-mono)" }}>{c.name}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>{c.dimension}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>{c.metric}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>{c.vectorCount.toLocaleString()}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center", fontSize: 10 }}>M={c.hnsw.m} ef={c.hnsw.efConstruction}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>
                          <button onClick={() => handleDeleteCollection(c.name)} style={{ background: "none", border: "none", color: "var(--error-color)", cursor: "pointer", fontSize: 11 }}>Delete</button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            {collections.length === 0 && (
              <div style={{ textAlign: "center", opacity: 0.4, fontSize: 12, marginTop: 16 }}>No collections yet. Create one above.</div>
            )}
          </div>
        )}

        {!loading && tab === "search" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div>
              <label style={labelStyle}>Collection</label>
              <select value={searchCollection} onChange={(e) => setSearchCollection(e.target.value)} style={inputStyle}>
                <option value="">Select a collection...</option>
                {collections.map((c) => (
                  <option key={c.name} value={c.name}>{c.name}</option>
                ))}
              </select>
            </div>
            <div>
              <label style={labelStyle}>Query Vector (comma-separated floats)</label>
              <textarea
                value={vectorInput}
                onChange={(e) => setVectorInput(e.target.value)}
                placeholder="0.1, -0.23, 0.87, 0.54, ..."
                rows={3}
                style={{ ...inputStyle, fontFamily: "var(--font-mono)", resize: "vertical" }}
              />
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
              <div>
                <label style={labelStyle}>Top K</label>
                <input type="number" min={1} max={100} value={topK} onChange={(e) => setTopK(Number(e.target.value))} style={inputStyle} />
              </div>
              <div>
                <label style={labelStyle}>Min Score</label>
                <input type="number" min={0} max={1} step={0.05} value={minScore} onChange={(e) => setMinScore(Number(e.target.value))} style={inputStyle} />
              </div>
            </div>

            <button onClick={handleSearch} disabled={isSearching || !vectorInput.trim()} style={{ ...btnPrimary, alignSelf: "flex-start", opacity: isSearching || !vectorInput.trim() ? 0.5 : 1 }}>
              {isSearching ? "Searching..." : "Search"}
            </button>

            {/* Results table */}
            {searchResults.length > 0 && (
              <div style={{ marginTop: 8 }}>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>{searchResults.length} result(s)</div>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                  <thead>
                    <tr style={{ background: "var(--bg-secondary)" }}>
                      <th style={{ padding: "6px 8px", textAlign: "left", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>ID</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600, width: 80 }}>Score</th>
                      <th style={{ padding: "6px 8px", textAlign: "left", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Payload</th>
                    </tr>
                  </thead>
                  <tbody>
                    {searchResults.map((r, i) => (
                      <tr key={r.id} style={{ background: i % 2 === 0 ? "transparent" : "var(--bg-secondary)" }}>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", fontFamily: "var(--font-mono)" }}>{r.id}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center", color: r.score > 0.8 ? "var(--success-color)" : r.score > 0.5 ? "var(--warning-color)" : "var(--text-secondary)" }}>{r.score}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", fontSize: 11 }}>
                          {Object.entries(r.payload).map(([k, v]) => (
                            <span key={k} style={{ marginRight: 12 }}><strong>{k}:</strong> {v}</span>
                          ))}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            {searchResults.length === 0 && !isSearching && (
              <div style={{ textAlign: "center", opacity: 0.4, fontSize: 12, marginTop: 16 }}>Enter a vector and click Search to find similar items.</div>
            )}
          </div>
        )}

        {!loading && tab === "schema" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div>
              <label style={labelStyle}>Vector Database Provider</label>
              <select value={schemaProvider} onChange={(e) => setSchemaProvider(e.target.value as Provider)} style={inputStyle}>
                <option value="qdrant">Qdrant</option>
                <option value="pinecone">Pinecone</option>
                <option value="pgvector">pgvector (PostgreSQL)</option>
                <option value="milvus">Milvus</option>
                <option value="weaviate">Weaviate</option>
                <option value="chroma">Chroma</option>
              </select>
            </div>

            <button onClick={handleGenerateSchema} style={{ ...btnPrimary, alignSelf: "flex-start" }}>
              Generate Schema
            </button>

            {schemaOutput && (
              <pre style={{
                background: "var(--bg-secondary)",
                border: "1px solid var(--border)",
                borderRadius: 4,
                padding: 16,
                fontSize: 12,
                fontFamily: "var(--font-mono)",
                margin: 0,
                whiteSpace: "pre-wrap",
                color: "var(--text-primary)",
                overflow: "auto",
                maxHeight: 400,
              }}>
                {schemaOutput}
              </pre>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
