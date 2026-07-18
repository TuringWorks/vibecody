/**
 * StreamingPanel — Kafka / event streaming management panel.
 *
 * Tabs: Topics, Producer/Consumer, Infrastructure
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "topics" | "prodcon" | "infra";
type CleanupPolicy = "delete" | "compact" | "compact,delete";
type Compression = "none" | "gzip" | "snappy" | "lz4" | "zstd";
type Acks = "0" | "1" | "all";

interface TopicEntry {
  name: string;
  partitions: number;
  replicationFactor: number;
  cleanupPolicy: CleanupPolicy;
}

interface ConnectorConfig {
  name: string;
  connectorClass: string;
  tasksMax: number;
  topics: string;
  connectionUrl: string;
}

export function StreamingPanel() {
  const [tab, setTab] = useState<Tab>("topics");
  const [loading, setLoading] = useState(true);

  // Topics state
  const [topicName, setTopicName] = useState("");
  const [partitions, setPartitions] = useState(3);
  const [replicationFactor, setReplicationFactor] = useState(1);
  const [cleanupPolicy, setCleanupPolicy] = useState<CleanupPolicy>("delete");
  const [topics, setTopics] = useState<TopicEntry[]>([]);

  // Producer/Consumer state
  const [brokers, setBrokers] = useState("localhost:9092");
  const [pcTopic, setPcTopic] = useState("");
  const [groupId, setGroupId] = useState("my-group");
  const [compression, setCompression] = useState<Compression>("none");
  const [acks, setAcks] = useState<Acks>("all");
  const [generatedCommand, setGeneratedCommand] = useState("");

  // Infrastructure state
  const [numBrokers, setNumBrokers] = useState(3);
  const [useZookeeper, setUseZookeeper] = useState(false);
  const [composeYaml, setComposeYaml] = useState("");
  const [connector, setConnector] = useState<ConnectorConfig>({
    name: "my-connector",
    connectorClass: "io.confluent.connect.jdbc.JdbcSinkConnector",
    tasksMax: 1,
    topics: "",
    connectionUrl: "jdbc:postgresql://localhost:5432/mydb",
  });
  const [connectorJson, setConnectorJson] = useState("");

  // Load topics from backend on mount
  useEffect(() => {
    const loadTopics = async () => {
      setLoading(true);
      try {
        const data = await invoke<TopicEntry[]>("get_streaming_topics");
        setTopics(data);
      } catch (err) {
        console.error("Failed to load streaming topics:", err);
      } finally {
        setLoading(false);
      }
    };
    loadTopics();
  }, []);

  const codeBlock: React.CSSProperties = {
    background: "var(--bg-secondary)",
    border: "1px solid var(--border)",
    borderRadius: "var(--radius-xs-plus)",
    padding: 12,
    fontFamily: "var(--font-mono)",
    fontSize: "var(--font-size-sm)",
    whiteSpace: "pre-wrap",
    overflowX: "auto",
    color: "var(--text-primary)",
    lineHeight: 1.5,
  };

  // --- Topics ---

  const handleAddTopic = async () => {
    const name = topicName.trim();
    if (!name) return;
    if (topics.some((t) => t.name === name)) return;
    const newTopic: TopicEntry = { name, partitions, replicationFactor, cleanupPolicy };
    setTopics([...topics, newTopic]);
    setTopicName("");
    try {
      await invoke("save_streaming_topic", { topic: newTopic });
    } catch (err) {
      console.error("Failed to save topic:", err);
    }
  };

  const handleDeleteTopic = async (name: string) => {
    setTopics(topics.filter((t) => t.name !== name));
    try {
      await invoke("delete_streaming_topic", { name });
    } catch (err) {
      console.error("Failed to delete topic:", err);
    }
  };

  // --- Producer / Consumer ---

  const generateProducerCommand = () => {
    const parts = [
      "kafka-console-producer.sh",
      `--bootstrap-server ${brokers}`,
      `--topic ${pcTopic || "<topic>"}`,
    ];
    if (compression !== "none") {
      parts.push(`--compression-codec ${compression}`);
    }
    if (acks !== "all") {
      parts.push(`--request-required-acks ${acks}`);
    }
    parts.push("--property parse.key=true --property key.separator=:");
    setGeneratedCommand(parts.join(" \\\n  "));
  };

  const generateConsumerCommand = () => {
    const parts = [
      "kafka-console-consumer.sh",
      `--bootstrap-server ${brokers}`,
      `--topic ${pcTopic || "<topic>"}`,
      `--group ${groupId}`,
      "--from-beginning",
      "--property print.key=true",
      "--property print.timestamp=true",
    ];
    setGeneratedCommand(parts.join(" \\\n  "));
  };

  // --- Infrastructure ---

  const generateDockerCompose = () => {
    const lines: string[] = [];
    lines.push("version: '3.8'");
    lines.push("services:");

    if (useZookeeper) {
      lines.push("  zookeeper:");
      lines.push("    image: confluentinc/cp-zookeeper:7.6.0");
      lines.push("    environment:");
      lines.push("      ZOOKEEPER_CLIENT_PORT: 2181");
      lines.push("      ZOOKEEPER_TICK_TIME: 2000");
      lines.push("    ports:");
      lines.push('      - "2181:2181"');
      lines.push("");
    }

    for (let i = 1; i <= numBrokers; i++) {
      const port = 9091 + i;
      lines.push(`  kafka-${i}:`);
      lines.push("    image: confluentinc/cp-kafka:7.6.0");
      if (useZookeeper) {
        lines.push("    depends_on:");
        lines.push("      - zookeeper");
      }
      lines.push("    ports:");
      lines.push(`      - "${port}:${port}"`);
      lines.push("    environment:");
      lines.push(`      KAFKA_BROKER_ID: ${i}`);

      if (useZookeeper) {
        lines.push("      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181");
      } else {
        lines.push(`      KAFKA_NODE_ID: ${i}`);
        lines.push("      KAFKA_PROCESS_ROLES: broker,controller");
        const voters = Array.from({ length: numBrokers }, (_, j) => `${j + 1}@kafka-${j + 1}:29093`).join(",");
        lines.push(`      KAFKA_CONTROLLER_QUORUM_VOTERS: ${voters}`);
        lines.push("      KAFKA_CONTROLLER_LISTENER_NAMES: CONTROLLER");
      }

      lines.push(`      KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: PLAINTEXT:PLAINTEXT,CONTROLLER:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT`);
      lines.push(`      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://kafka-${i}:29092,PLAINTEXT_HOST://localhost:${port}`);
      lines.push(`      KAFKA_INTER_BROKER_LISTENER_NAME: PLAINTEXT`);
      lines.push(`      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: ${Math.min(numBrokers, 3)}`);
      lines.push(`      KAFKA_TRANSACTION_STATE_LOG_REPLICATION_FACTOR: ${Math.min(numBrokers, 3)}`);
      lines.push(`      KAFKA_TRANSACTION_STATE_LOG_MIN_ISR: ${Math.min(numBrokers, 2)}`);
      if (i < numBrokers) lines.push("");
    }

    setComposeYaml(lines.join("\n"));
  };

  const generateConnectorConfig = () => {
    const config = {
      name: connector.name,
      config: {
        "connector.class": connector.connectorClass,
        "tasks.max": String(connector.tasksMax),
        topics: connector.topics,
        "connection.url": connector.connectionUrl,
        "auto.create": "true",
        "insert.mode": "upsert",
        "pk.mode": "record_key",
      },
    };
    setConnectorJson(JSON.stringify(config, null, 2));
  };

  // --- Render ---

  const tabs: { key: Tab; label: string }[] = [
    { key: "topics", label: "Topics" },
    { key: "prodcon", label: "Producer / Consumer" },
    { key: "infra", label: "Infrastructure" },
  ];

  return (
    <div className="panel-container">
      {/* Tab bar */}
      <div className="panel-tab-bar">
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`panel-tab ${tab === t.key ? "active" : ""}`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="panel-body">
        {/* ===== Topics ===== */}
        {tab === "topics" && (
          <div>
            <h3 style={{ margin: "0 0 12px", fontSize: "var(--font-size-lg)" }}>Create Topic</h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr auto", gap: 8, alignItems: "end", marginBottom: 16 }}>
              <div>
                <label className="panel-label">Name</label>
                <input
                  value={topicName}
                  onChange={(e) => setTopicName(e.target.value)}
                  placeholder="my-events"
                  className="panel-input"
                  onKeyDown={(e) => e.key === "Enter" && handleAddTopic()}
                />
              </div>
              <div>
                <label className="panel-label">Partitions</label>
                <input
                  type="number"
                  min={1}
                  value={partitions}
                  onChange={(e) => setPartitions(Number(e.target.value))}
                  className="panel-input"
                />
              </div>
              <div>
                <label className="panel-label">Replication Factor</label>
                <input
                  type="number"
                  min={1}
                  value={replicationFactor}
                  onChange={(e) => setReplicationFactor(Number(e.target.value))}
                  className="panel-input"
                />
              </div>
              <div>
                <label className="panel-label">Cleanup Policy</label>
                <select
                  value={cleanupPolicy}
                  onChange={(e) => setCleanupPolicy(e.target.value as CleanupPolicy)}
                  className="panel-select"
                >
                  <option value="delete">delete</option>
                  <option value="compact">compact</option>
                  <option value="compact,delete">compact,delete</option>
                </select>
              </div>
              <button onClick={handleAddTopic} className="panel-btn panel-btn-primary">
                Add
              </button>
            </div>

            {/* Topics table */}
            {loading ? (
              <div className="panel-loading">Loading topics...</div>
            ) : topics.length === 0 ? (
              <div className="panel-empty">No topics created yet. Use the form above to add topics.</div>
            ) : (
              <div style={{ overflowX: "auto" }}>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)" }}>
                  <thead>
                    <tr style={{ background: "var(--bg-secondary)" }}>
                      <th style={{ padding: "8px 8px", textAlign: "left", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Name</th>
                      <th style={{ padding: "8px 8px", textAlign: "right", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Partitions</th>
                      <th style={{ padding: "8px 8px", textAlign: "right", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Replication</th>
                      <th style={{ padding: "8px 8px", textAlign: "left", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Cleanup</th>
                      <th style={{ padding: "8px 8px", textAlign: "left", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>CLI Command</th>
                      <th style={{ padding: "8px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600 }}></th>
                    </tr>
                  </thead>
                  <tbody>
                    {topics.map((t, i) => (
                      <tr key={t.name} style={{ background: i % 2 === 0 ? "transparent" : "var(--bg-secondary)" }}>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)" }}>{t.name}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "right" }}>{t.partitions}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "right" }}>{t.replicationFactor}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)" }}>{t.cleanupPolicy}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", fontSize: "var(--font-size-xs)", opacity: 0.7 }}>
                          kafka-topics.sh --create --topic {t.name} --partitions {t.partitions} --replication-factor {t.replicationFactor} --config cleanup.policy={t.cleanupPolicy}
                        </td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>
                          <button onClick={() => handleDeleteTopic(t.name)} style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: "var(--font-size-lg)" }} title="Remove topic">
                            x
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}

        {/* ===== Producer / Consumer ===== */}
        {tab === "prodcon" && (
          <div>
            <h3 style={{ margin: "0 0 12px", fontSize: "var(--font-size-lg)" }}>Client Configuration</h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 16 }}>
              <div>
                <label className="panel-label">Bootstrap Servers</label>
                <input value={brokers} onChange={(e) => setBrokers(e.target.value)} placeholder="localhost:9092" className="panel-input" />
              </div>
              <div>
                <label className="panel-label">Topic</label>
                <input value={pcTopic} onChange={(e) => setPcTopic(e.target.value)} placeholder="my-topic" className="panel-input" />
              </div>
              <div>
                <label className="panel-label">Group ID</label>
                <input value={groupId} onChange={(e) => setGroupId(e.target.value)} placeholder="my-group" className="panel-input" />
              </div>
              <div>
                <label className="panel-label">Compression</label>
                <select value={compression} onChange={(e) => setCompression(e.target.value as Compression)} className="panel-select">
                  <option value="none">none</option>
                  <option value="gzip">gzip</option>
                  <option value="snappy">snappy</option>
                  <option value="lz4">lz4</option>
                  <option value="zstd">zstd</option>
                </select>
              </div>
              <div>
                <label className="panel-label">Acks</label>
                <select value={acks} onChange={(e) => setAcks(e.target.value as Acks)} className="panel-select">
                  <option value="all">all</option>
                  <option value="1">1</option>
                  <option value="0">0</option>
                </select>
              </div>
            </div>

            <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
              <button onClick={generateProducerCommand} className="panel-btn panel-btn-primary">Generate Producer Command</button>
              <button onClick={generateConsumerCommand} className="panel-btn panel-btn-secondary">Generate Consumer Command</button>
            </div>

            {generatedCommand && (
              <div>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Generated Command</span>
                  <button
                    onClick={() => navigator.clipboard.writeText(generatedCommand)}
                    className="panel-btn panel-btn-secondary panel-btn-xs"
                  >
                    Copy
                  </button>
                </div>
                <pre style={codeBlock}>{generatedCommand}</pre>
              </div>
            )}
          </div>
        )}

        {/* ===== Infrastructure ===== */}
        {tab === "infra" && (
          <div>
            {/* Docker Compose Generator */}
            <h3 style={{ margin: "0 0 12px", fontSize: "var(--font-size-lg)" }}>Docker Compose Generator</h3>
            <div style={{ display: "flex", gap: 12, alignItems: "end", marginBottom: 12 }}>
              <div>
                <label className="panel-label">Number of Brokers</label>
                <input
                  type="number"
                  min={1}
                  max={9}
                  value={numBrokers}
                  onChange={(e) => setNumBrokers(Math.max(1, Math.min(9, Number(e.target.value))))}
                  className="panel-input"
                  style={{ width: 80 }}
                />
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 6, paddingBottom: 2 }}>
                <input
                  type="checkbox"
                  id="zk-toggle"
                  checked={useZookeeper}
                  onChange={(e) => setUseZookeeper(e.target.checked)}
                  style={{ accentColor: "var(--accent)" }}
                />
                <label htmlFor="zk-toggle" style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", cursor: "pointer" }}>
                  Use Zookeeper (legacy)
                </label>
              </div>
              <button onClick={generateDockerCompose} className="panel-btn panel-btn-primary">Generate Compose</button>
            </div>

            {composeYaml && (
              <div style={{ marginBottom: 24 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>docker-compose.yml</span>
                  <button
                    onClick={() => navigator.clipboard.writeText(composeYaml)}
                    className="panel-btn panel-btn-secondary panel-btn-xs"
                  >
                    Copy
                  </button>
                </div>
                <pre style={{ ...codeBlock, maxHeight: 400, overflowY: "auto" }}>{composeYaml}</pre>
              </div>
            )}

            {/* Kafka Connect Connector */}
            <h3 style={{ margin: "0 0 12px", fontSize: "var(--font-size-lg)" }}>Kafka Connect — Connector Config</h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 12 }}>
              <div>
                <label className="panel-label">Connector Name</label>
                <input value={connector.name} onChange={(e) => setConnector({ ...connector, name: e.target.value })} className="panel-input" />
              </div>
              <div>
                <label className="panel-label">Connector Class</label>
                <input value={connector.connectorClass} onChange={(e) => setConnector({ ...connector, connectorClass: e.target.value })} className="panel-input" />
              </div>
              <div>
                <label className="panel-label">Tasks Max</label>
                <input type="number" min={1} value={connector.tasksMax} onChange={(e) => setConnector({ ...connector, tasksMax: Number(e.target.value) })} className="panel-input" />
              </div>
              <div>
                <label className="panel-label">Topics</label>
                <input value={connector.topics} onChange={(e) => setConnector({ ...connector, topics: e.target.value })} placeholder="topic-a,topic-b" className="panel-input" />
              </div>
              <div style={{ gridColumn: "1 / -1" }}>
                <label className="panel-label">Connection URL</label>
                <input value={connector.connectionUrl} onChange={(e) => setConnector({ ...connector, connectionUrl: e.target.value })} className="panel-input" />
              </div>
            </div>

            <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
              <button onClick={generateConnectorConfig} className="panel-btn panel-btn-primary">Generate Connector JSON</button>
            </div>

            {connectorJson && (
              <div>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Connector Configuration</span>
                  <button
                    onClick={() => navigator.clipboard.writeText(connectorJson)}
                    className="panel-btn panel-btn-secondary panel-btn-xs"
                  >
                    Copy
                  </button>
                </div>
                <pre style={codeBlock}>{connectorJson}</pre>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
