// Unified streaming/event platform client module for VibeCody CLI.
// Provides abstractions for Kafka, Pulsar, Redis Streams, NATS, RabbitMQ,
// and cloud event services (Kinesis, Event Hub, Pub/Sub).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamPlatform {
    Kafka,
    Pulsar,
    RedisStreams,
    Nats,
    RabbitMq,
    AmazonKinesis,
    AzureEventHub,
    GooglePubSub,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMechanism {
    Plain,
    SaslScram256,
    SaslScram512,
    OAuth,
    ApiKey,
    Mtls,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageFormat {
    Json,
    Avro,
    Protobuf,
    PlainText,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compression {
    None,
    Gzip,
    Snappy,
    Lz4,
    Zstd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Acks {
    None,
    Leader,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OffsetReset {
    Earliest,
    Latest,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupPolicy {
    Delete,
    Compact,
    CompactDelete,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamAuth {
    pub mechanism: AuthMechanism,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub platform: StreamPlatform,
    pub brokers: Vec<String>,
    pub topic: String,
    pub group_id: Option<String>,
    pub auth: Option<StreamAuth>,
    pub tls: bool,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerConfig {
    pub config: StreamConfig,
    pub batch_size: u32,
    pub linger_ms: u32,
    pub compression: Compression,
    pub acks: Acks,
    pub idempotent: bool,
    pub format: MessageFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    pub config: StreamConfig,
    pub auto_commit: bool,
    pub commit_interval_ms: u32,
    pub offset_reset: OffsetReset,
    pub max_poll_records: u32,
    pub format: MessageFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessage {
    pub key: Option<String>,
    pub value: String,
    pub topic: String,
    pub partition: Option<u32>,
    pub offset: Option<u64>,
    pub timestamp: Option<u64>,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicConfig {
    pub name: String,
    pub partitions: u32,
    pub replication_factor: u16,
    pub retention_ms: Option<u64>,
    pub cleanup_policy: CleanupPolicy,
    pub configs: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub name: String,
    pub connector_class: String,
    pub tasks_max: u32,
    pub topics: Vec<String>,
    pub connection: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetrics {
    pub messages_per_sec: f64,
    pub bytes_per_sec: f64,
    pub consumer_lag: u64,
    pub partition_count: u32,
}

// ---------------------------------------------------------------------------
// KafkaCliBuilder
// ---------------------------------------------------------------------------

pub struct KafkaCliBuilder;

impl KafkaCliBuilder {
    pub fn build_topic_create(config: &TopicConfig, brokers: &str) -> Vec<String> {
        let mut cmd = vec![
            "kafka-topics.sh".to_string(),
            "--create".to_string(),
            "--bootstrap-server".to_string(), brokers.to_string(),
            "--topic".to_string(), config.name.clone(),
            "--partitions".to_string(), config.partitions.to_string(),
            "--replication-factor".to_string(), config.replication_factor.to_string(),
        ];
        if let Some(retention) = config.retention_ms {
            cmd.push("--config".to_string());
            cmd.push(format!("retention.ms={retention}"));
        }
        match config.cleanup_policy {
            CleanupPolicy::Compact => {
                cmd.push("--config".to_string());
                cmd.push("cleanup.policy=compact".to_string());
            }
            CleanupPolicy::CompactDelete => {
                cmd.push("--config".to_string());
                cmd.push("cleanup.policy=compact,delete".to_string());
            }
            CleanupPolicy::Delete => {}
        }
        for (k, v) in &config.configs {
            cmd.push("--config".to_string());
            cmd.push(format!("{k}={v}"));
        }
        cmd
    }

    pub fn build_topic_list(brokers: &str) -> Vec<String> {
        vec![
            "kafka-topics.sh".to_string(),
            "--list".to_string(),
            "--bootstrap-server".to_string(), brokers.to_string(),
        ]
    }

    pub fn build_topic_describe(topic: &str, brokers: &str) -> Vec<String> {
        vec![
            "kafka-topics.sh".to_string(),
            "--describe".to_string(),
            "--bootstrap-server".to_string(), brokers.to_string(),
            "--topic".to_string(), topic.to_string(),
        ]
    }

    pub fn build_console_producer(config: &ProducerConfig) -> Vec<String> {
        let brokers = config.config.brokers.join(",");
        let mut cmd = vec![
            "kafka-console-producer.sh".to_string(),
            "--bootstrap-server".to_string(), brokers,
            "--topic".to_string(), config.config.topic.clone(),
        ];
        if config.compression != Compression::None {
            cmd.push("--compression-codec".to_string());
            cmd.push(match config.compression {
                Compression::Gzip => "gzip",
                Compression::Snappy => "snappy",
                Compression::Lz4 => "lz4",
                Compression::Zstd => "zstd",
                Compression::None => "none",
            }.to_string());
        }
        cmd
    }

    pub fn build_console_consumer(config: &ConsumerConfig) -> Vec<String> {
        let brokers = config.config.brokers.join(",");
        let mut cmd = vec![
            "kafka-console-consumer.sh".to_string(),
            "--bootstrap-server".to_string(), brokers,
            "--topic".to_string(), config.config.topic.clone(),
        ];
        if let Some(ref group) = config.config.group_id {
            cmd.push("--group".to_string());
            cmd.push(group.clone());
        }
        match config.offset_reset {
            OffsetReset::Earliest => { cmd.push("--from-beginning".to_string()); }
            _ => {}
        }
        if config.max_poll_records > 0 {
            cmd.push("--max-messages".to_string());
            cmd.push(config.max_poll_records.to_string());
        }
        cmd
    }

    pub fn build_consumer_groups_list(brokers: &str) -> Vec<String> {
        vec![
            "kafka-consumer-groups.sh".to_string(),
            "--list".to_string(),
            "--bootstrap-server".to_string(), brokers.to_string(),
        ]
    }

    pub fn build_consumer_groups_describe(group: &str, brokers: &str) -> Vec<String> {
        vec![
            "kafka-consumer-groups.sh".to_string(),
            "--describe".to_string(),
            "--bootstrap-server".to_string(), brokers.to_string(),
            "--group".to_string(), group.to_string(),
        ]
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

pub fn validate_stream_config(config: &StreamConfig) -> Vec<String> {
    let mut errors = vec![];
    if config.brokers.is_empty() {
        errors.push("At least one broker is required".to_string());
    }
    if config.topic.is_empty() {
        errors.push("Topic name is required".to_string());
    }
    for b in &config.brokers {
        if !b.contains(':') && config.platform == StreamPlatform::Kafka {
            errors.push(format!("Broker '{b}' should include port (e.g., localhost:9092)"));
        }
    }
    if let Some(ref auth) = config.auth {
        if auth.mechanism == AuthMechanism::Plain {
            if auth.username.is_none() || auth.password.is_none() {
                errors.push("PLAIN auth requires username and password".to_string());
            }
        }
    }
    errors
}

pub fn generate_kafka_connect_config(connector: &ConnectorConfig) -> String {
    let mut obj = serde_json::Map::new();
    obj.insert("name".into(), serde_json::Value::String(connector.name.clone()));
    let mut cfg = serde_json::Map::new();
    cfg.insert("connector.class".into(), serde_json::Value::String(connector.connector_class.clone()));
    cfg.insert("tasks.max".into(), serde_json::Value::String(connector.tasks_max.to_string()));
    if !connector.topics.is_empty() {
        cfg.insert("topics".into(), serde_json::Value::String(connector.topics.join(",")));
    }
    for (k, v) in &connector.connection {
        cfg.insert(k.clone(), serde_json::Value::String(v.clone()));
    }
    obj.insert("config".into(), serde_json::Value::Object(cfg));
    serde_json::to_string_pretty(&serde_json::Value::Object(obj)).unwrap_or_default()
}

pub fn generate_docker_compose_kafka(brokers: u32, zookeeper: bool) -> String {
    let mut yaml = String::from("version: '3.8'\nservices:\n");
    if zookeeper {
        yaml.push_str("  zookeeper:\n");
        yaml.push_str("    image: confluentinc/cp-zookeeper:7.6.0\n");
        yaml.push_str("    environment:\n");
        yaml.push_str("      ZOOKEEPER_CLIENT_PORT: 2181\n");
        yaml.push_str("      ZOOKEEPER_TICK_TIME: 2000\n");
        yaml.push_str("    ports:\n");
        yaml.push_str("      - '2181:2181'\n\n");
    }
    for i in 0..brokers {
        let port = 9092 + i;
        let internal_port = 29092 + i;
        yaml.push_str(&format!("  kafka-{i}:\n"));
        yaml.push_str("    image: confluentinc/cp-kafka:7.6.0\n");
        if zookeeper {
            yaml.push_str("    depends_on:\n");
            yaml.push_str("      - zookeeper\n");
        }
        yaml.push_str("    ports:\n");
        yaml.push_str(&format!("      - '{port}:{port}'\n"));
        yaml.push_str("    environment:\n");
        yaml.push_str(&format!("      KAFKA_BROKER_ID: {i}\n"));
        if zookeeper {
            yaml.push_str("      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181\n");
        } else {
            yaml.push_str("      KAFKA_PROCESS_ROLES: broker,controller\n");
            yaml.push_str(&format!("      KAFKA_NODE_ID: {i}\n"));
        }
        yaml.push_str(&format!("      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://localhost:{port},INTERNAL://kafka-{i}:{internal_port}\n"));
        yaml.push_str("      KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: PLAINTEXT:PLAINTEXT,INTERNAL:PLAINTEXT\n");
        yaml.push_str("      KAFKA_INTER_BROKER_LISTENER_NAME: INTERNAL\n");
        yaml.push_str(&format!("      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: {}\n", brokers.min(3)));
        yaml.push_str("\n");
    }
    yaml
}

pub fn estimate_throughput(msg_size_bytes: u64, msgs_per_sec: u64) -> String {
    let bytes_per_sec = msg_size_bytes * msgs_per_sec;
    if bytes_per_sec >= 1_073_741_824 {
        format!("{:.2} GB/s ({} msgs/s, {} bytes/msg)", bytes_per_sec as f64 / 1_073_741_824.0, msgs_per_sec, msg_size_bytes)
    } else if bytes_per_sec >= 1_048_576 {
        format!("{:.2} MB/s ({} msgs/s, {} bytes/msg)", bytes_per_sec as f64 / 1_048_576.0, msgs_per_sec, msg_size_bytes)
    } else if bytes_per_sec >= 1024 {
        format!("{:.2} KB/s ({} msgs/s, {} bytes/msg)", bytes_per_sec as f64 / 1024.0, msgs_per_sec, msg_size_bytes)
    } else {
        format!("{} B/s ({} msgs/s, {} bytes/msg)", bytes_per_sec, msgs_per_sec, msg_size_bytes)
    }
}

pub fn suggest_partition_count(throughput_mbps: f64, consumer_count: u32) -> u32 {
    // Rule of thumb: each partition can handle ~10 MB/s consumer throughput
    let by_throughput = (throughput_mbps / 10.0).ceil() as u32;
    // At least as many partitions as consumers for parallelism
    let by_consumers = consumer_count;
    // Minimum 1, practical max 256
    by_throughput.max(by_consumers).max(1).min(256)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_stream_config() -> StreamConfig {
        StreamConfig {
            platform: StreamPlatform::Kafka,
            brokers: vec!["localhost:9092".to_string()],
            topic: "test-topic".to_string(),
            group_id: Some("test-group".to_string()),
            auth: None,
            tls: false,
            extra: HashMap::new(),
        }
    }

    #[test]
    fn test_stream_platform_serialization() {
        let p = StreamPlatform::Kafka;
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"kafka\"");
        let back: StreamPlatform = serde_json::from_str(&json).unwrap();
        assert_eq!(back, StreamPlatform::Kafka);
    }

    #[test]
    fn test_compression_serialization() {
        let c = Compression::Zstd;
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "\"zstd\"");
    }

    #[test]
    fn test_acks_serialization() {
        let a = Acks::All;
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, "\"all\"");
    }

    #[test]
    fn test_offset_reset_serialization() {
        let o = OffsetReset::Earliest;
        let json = serde_json::to_string(&o).unwrap();
        assert_eq!(json, "\"earliest\"");
    }

    #[test]
    fn test_cleanup_policy_serialization() {
        let c = CleanupPolicy::CompactDelete;
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "\"compact_delete\"");
    }

    #[test]
    fn test_stream_config_validate_valid() {
        let config = default_stream_config();
        let errors = validate_stream_config(&config);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_stream_config_validate_no_brokers() {
        let mut config = default_stream_config();
        config.brokers.clear();
        let errors = validate_stream_config(&config);
        assert!(errors.iter().any(|e| e.contains("broker")));
    }

    #[test]
    fn test_stream_config_validate_no_topic() {
        let mut config = default_stream_config();
        config.topic.clear();
        let errors = validate_stream_config(&config);
        assert!(errors.iter().any(|e| e.contains("Topic")));
    }

    #[test]
    fn test_kafka_topic_create_command() {
        let tc = TopicConfig {
            name: "orders".to_string(),
            partitions: 12,
            replication_factor: 3,
            retention_ms: Some(604800000),
            cleanup_policy: CleanupPolicy::Delete,
            configs: HashMap::new(),
        };
        let cmd = KafkaCliBuilder::build_topic_create(&tc, "localhost:9092");
        assert!(cmd.contains(&"kafka-topics.sh".to_string()));
        assert!(cmd.contains(&"--create".to_string()));
        assert!(cmd.contains(&"orders".to_string()));
        assert!(cmd.contains(&"12".to_string()));
        assert!(cmd.contains(&"retention.ms=604800000".to_string()));
    }

    #[test]
    fn test_kafka_topic_list_command() {
        let cmd = KafkaCliBuilder::build_topic_list("broker:9092");
        assert_eq!(cmd[0], "kafka-topics.sh");
        assert!(cmd.contains(&"--list".to_string()));
        assert!(cmd.contains(&"broker:9092".to_string()));
    }

    #[test]
    fn test_kafka_console_producer_command() {
        let pc = ProducerConfig {
            config: default_stream_config(),
            batch_size: 16384,
            linger_ms: 5,
            compression: Compression::Gzip,
            acks: Acks::All,
            idempotent: true,
            format: MessageFormat::Json,
        };
        let cmd = KafkaCliBuilder::build_console_producer(&pc);
        assert!(cmd.contains(&"kafka-console-producer.sh".to_string()));
        assert!(cmd.contains(&"--compression-codec".to_string()));
        assert!(cmd.contains(&"gzip".to_string()));
    }

    #[test]
    fn test_kafka_console_consumer_command() {
        let cc = ConsumerConfig {
            config: default_stream_config(),
            auto_commit: true,
            commit_interval_ms: 5000,
            offset_reset: OffsetReset::Earliest,
            max_poll_records: 100,
            format: MessageFormat::Json,
        };
        let cmd = KafkaCliBuilder::build_console_consumer(&cc);
        assert!(cmd.contains(&"kafka-console-consumer.sh".to_string()));
        assert!(cmd.contains(&"--from-beginning".to_string()));
        assert!(cmd.contains(&"--group".to_string()));
        assert!(cmd.contains(&"--max-messages".to_string()));
    }

    #[test]
    fn test_kafka_consumer_groups_list() {
        let cmd = KafkaCliBuilder::build_consumer_groups_list("broker:9092");
        assert!(cmd.contains(&"kafka-consumer-groups.sh".to_string()));
        assert!(cmd.contains(&"--list".to_string()));
    }

    #[test]
    fn test_generate_kafka_connect_config() {
        let connector = ConnectorConfig {
            name: "jdbc-source".to_string(),
            connector_class: "io.confluent.connect.jdbc.JdbcSourceConnector".to_string(),
            tasks_max: 1,
            topics: vec!["orders".to_string()],
            connection: {
                let mut m = HashMap::new();
                m.insert("connection.url".into(), "jdbc:postgresql://localhost/db".into());
                m
            },
        };
        let json = generate_kafka_connect_config(&connector);
        assert!(json.contains("jdbc-source"));
        assert!(json.contains("connector.class"));
        assert!(json.contains("orders"));
    }

    #[test]
    fn test_generate_docker_compose_kafka() {
        let yaml = generate_docker_compose_kafka(3, true);
        assert!(yaml.contains("zookeeper"));
        assert!(yaml.contains("kafka-0"));
        assert!(yaml.contains("kafka-1"));
        assert!(yaml.contains("kafka-2"));
        assert!(yaml.contains("9092"));
    }

    #[test]
    fn test_estimate_throughput() {
        let result = estimate_throughput(1024, 1000);
        assert!(result.contains("KB/s") || result.contains("MB/s"));
        let large = estimate_throughput(1024, 1_000_000);
        assert!(large.contains("MB/s") || large.contains("GB/s"));
        let small = estimate_throughput(100, 5);
        assert!(small.contains("B/s") || small.contains("KB/s"));
    }

    #[test]
    fn test_suggest_partition_count() {
        assert_eq!(suggest_partition_count(100.0, 4), 10); // 100/10 = 10 > 4
        assert_eq!(suggest_partition_count(5.0, 8), 8);    // 1 < 8, use consumers
        assert_eq!(suggest_partition_count(0.1, 1), 1);     // minimum 1
    }
}
