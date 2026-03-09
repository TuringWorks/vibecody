// Unified database client module for VibeCody CLI.
// Provides connection string building, CLI command generation, schema utilities,
// migration management, and query result parsing for multiple database engines.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Supported database engines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseEngine {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
    Redis,
    DuckDb,
}

impl std::fmt::Display for DatabaseEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseEngine::PostgreSQL => write!(f, "PostgreSQL"),
            DatabaseEngine::MySQL => write!(f, "MySQL"),
            DatabaseEngine::SQLite => write!(f, "SQLite"),
            DatabaseEngine::MongoDB => write!(f, "MongoDB"),
            DatabaseEngine::Redis => write!(f, "Redis"),
            DatabaseEngine::DuckDb => write!(f, "DuckDb"),
        }
    }
}

/// Configuration for connecting to a database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub engine: DatabaseEngine,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub ssl: bool,
    /// If set, overrides individual connection fields.
    pub connection_string: Option<String>,
    pub pool_size: u16,
    pub timeout_secs: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            engine: DatabaseEngine::PostgreSQL,
            host: "localhost".to_string(),
            port: 5432,
            database: "vibecody".to_string(),
            username: None,
            password: None,
            ssl: false,
            connection_string: None,
            pool_size: 5,
            timeout_secs: 30,
        }
    }
}

impl DatabaseConfig {
    /// Create a SQLite configuration pointing at the given file path.
    pub fn sqlite(path: &str) -> Self {
        Self {
            engine: DatabaseEngine::SQLite,
            host: String::new(),
            port: 0,
            database: path.to_string(),
            username: None,
            password: None,
            ssl: false,
            connection_string: None,
            pool_size: 1,
            timeout_secs: 30,
        }
    }

    /// Create a Redis configuration.
    pub fn redis(host: &str, port: u16) -> Self {
        Self {
            engine: DatabaseEngine::Redis,
            host: host.to_string(),
            port,
            database: "0".to_string(),
            username: None,
            password: None,
            ssl: false,
            connection_string: None,
            pool_size: 1,
            timeout_secs: 30,
        }
    }
}

/// Result of executing a database query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub rows_affected: u64,
    pub execution_time_ms: u64,
}

/// Metadata about a database table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub schema: Option<String>,
    pub columns: Vec<ColumnInfo>,
    pub row_count: Option<u64>,
    pub size_bytes: Option<u64>,
}

/// Metadata about a single column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub default_value: Option<String>,
    pub foreign_key: Option<String>,
}

/// Metadata about a database index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub index_type: String,
}

/// Status of a migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    Pending,
    Applied,
    Failed,
}

/// A single database migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    pub version: String,
    pub name: String,
    pub sql_up: String,
    pub sql_down: String,
    pub status: MigrationStatus,
    pub applied_at: Option<String>,
}

/// Scans, generates, and validates migration files on disk.
#[derive(Debug)]
pub struct MigrationRunner {
    pub migrations_dir: PathBuf,
    pub engine: DatabaseEngine,
}

impl MigrationRunner {
    pub fn new(dir: &Path, engine: DatabaseEngine) -> Self {
        Self {
            migrations_dir: dir.to_path_buf(),
            engine,
        }
    }

    /// Scan the migrations directory for `.sql` files matching the pattern `V###__name.sql`.
    /// Returns them sorted by version.
    pub fn scan_migrations(&self) -> Result<Vec<Migration>> {
        let mut migrations = Vec::new();

        if !self.migrations_dir.exists() {
            return Ok(migrations);
        }

        let mut entries: Vec<_> = std::fs::read_dir(&self.migrations_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext == "sql")
            })
            .collect();

        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let filename = entry.file_name().to_string_lossy().to_string();
            // Expected format: V001__create_users.sql
            if let Some(rest) = filename.strip_prefix('V') {
                if let Some(sep_pos) = rest.find("__") {
                    let version = rest[..sep_pos].to_string();
                    let name = rest[sep_pos + 2..]
                        .strip_suffix(".sql")
                        .unwrap_or(&rest[sep_pos + 2..])
                        .to_string();
                    let content = std::fs::read_to_string(entry.path()).unwrap_or_default();

                    // Split on "-- DOWN" marker if present
                    let (sql_up, sql_down) = if let Some(pos) = content.find("-- DOWN") {
                        (content[..pos].trim().to_string(), content[pos + 7..].trim().to_string())
                    } else {
                        (content.trim().to_string(), String::new())
                    };

                    migrations.push(Migration {
                        version,
                        name,
                        sql_up,
                        sql_down,
                        status: MigrationStatus::Pending,
                        applied_at: None,
                    });
                }
            }
        }

        Ok(migrations)
    }

    /// Generate a new migration file with a timestamp-based version.
    /// Returns the path to the created file.
    pub fn generate_migration(&self, name: &str) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.migrations_dir)?;

        // Count existing migrations to determine the next version number.
        let existing = self.scan_migrations().unwrap_or_default();
        let next_version = existing.len() + 1;
        let version_str = format!("{:03}", next_version);

        let sanitized_name = name.replace(' ', "_").to_lowercase();
        let filename = format!("V{}__{}.sql", version_str, sanitized_name);
        let path = self.migrations_dir.join(&filename);

        let template = format!(
            "-- Migration: {}\n-- Engine: {}\n\n-- UP\n\n-- DOWN\n",
            sanitized_name, self.engine
        );
        std::fs::write(&path, template)?;

        Ok(path)
    }

    /// Validate a list of migrations for gaps and duplicate versions.
    /// Returns a list of warning/error messages.
    pub fn validate_migrations(&self, migrations: &[Migration]) -> Vec<String> {
        let mut errors = Vec::new();

        // Check for duplicate versions.
        let mut seen_versions = std::collections::HashSet::new();
        for m in migrations {
            if !seen_versions.insert(&m.version) {
                errors.push(format!("Duplicate migration version: {}", m.version));
            }
        }

        // Check for gaps in numeric versions.
        let mut numeric_versions: Vec<u64> = migrations
            .iter()
            .filter_map(|m| m.version.parse::<u64>().ok())
            .collect();
        numeric_versions.sort();
        numeric_versions.dedup();

        for window in numeric_versions.windows(2) {
            if window[1] - window[0] > 1 {
                errors.push(format!(
                    "Gap in migration versions between {} and {}",
                    window[0], window[1]
                ));
            }
        }

        errors
    }
}

/// Client that builds connection strings and CLI commands for database operations.
#[derive(Debug)]
pub struct DatabaseClient {
    pub config: DatabaseConfig,
}

impl DatabaseClient {
    pub fn new(config: DatabaseConfig) -> Self {
        Self { config }
    }

    /// Build a connection string from the config fields.
    /// If `connection_string` is set, returns that directly.
    pub fn build_connection_string(&self) -> String {
        if let Some(ref cs) = self.config.connection_string {
            return cs.clone();
        }

        match self.config.engine {
            DatabaseEngine::PostgreSQL => {
                let mut s = format!("postgresql://");
                if let Some(ref user) = self.config.username {
                    s.push_str(user);
                    if let Some(ref pass) = self.config.password {
                        s.push(':');
                        s.push_str(pass);
                    }
                    s.push('@');
                }
                s.push_str(&format!(
                    "{}:{}/{}",
                    self.config.host, self.config.port, self.config.database
                ));
                if self.config.ssl {
                    s.push_str("?sslmode=require");
                }
                s
            }
            DatabaseEngine::MySQL => {
                let mut s = "mysql://".to_string();
                if let Some(ref user) = self.config.username {
                    s.push_str(user);
                    if let Some(ref pass) = self.config.password {
                        s.push(':');
                        s.push_str(pass);
                    }
                    s.push('@');
                }
                s.push_str(&format!(
                    "{}:{}/{}",
                    self.config.host, self.config.port, self.config.database
                ));
                if self.config.ssl {
                    s.push_str("?ssl=true");
                }
                s
            }
            DatabaseEngine::SQLite => {
                format!("sqlite://{}", self.config.database)
            }
            DatabaseEngine::MongoDB => {
                let mut s = "mongodb://".to_string();
                if let Some(ref user) = self.config.username {
                    s.push_str(user);
                    if let Some(ref pass) = self.config.password {
                        s.push(':');
                        s.push_str(pass);
                    }
                    s.push('@');
                }
                s.push_str(&format!(
                    "{}:{}/{}",
                    self.config.host, self.config.port, self.config.database
                ));
                if self.config.ssl {
                    s.push_str("?tls=true");
                }
                s
            }
            DatabaseEngine::Redis => {
                let mut s = "redis://".to_string();
                if let Some(ref pass) = self.config.password {
                    s.push(':');
                    s.push_str(pass);
                    s.push('@');
                }
                s.push_str(&format!(
                    "{}:{}/{}",
                    self.config.host, self.config.port, self.config.database
                ));
                s
            }
            DatabaseEngine::DuckDb => {
                format!("duckdb://{}", self.config.database)
            }
        }
    }

    /// Build a `psql` CLI argument list for the given query.
    pub fn build_psql_command(&self, query: &str) -> Vec<String> {
        let mut args = vec!["psql".to_string()];
        args.push("-h".to_string());
        args.push(self.config.host.clone());
        args.push("-p".to_string());
        args.push(self.config.port.to_string());
        if let Some(ref user) = self.config.username {
            args.push("-U".to_string());
            args.push(user.clone());
        }
        args.push("-d".to_string());
        args.push(self.config.database.clone());
        args.push("-c".to_string());
        args.push(query.to_string());
        args
    }

    /// Build a `mysql` CLI argument list for the given query.
    pub fn build_mysql_command(&self, query: &str) -> Vec<String> {
        let mut args = vec!["mysql".to_string()];
        args.push("-h".to_string());
        args.push(self.config.host.clone());
        args.push("-P".to_string());
        args.push(self.config.port.to_string());
        if let Some(ref user) = self.config.username {
            args.push("-u".to_string());
            args.push(user.clone());
        }
        if let Some(ref pass) = self.config.password {
            args.push(format!("-p{}", pass));
        }
        args.push(self.config.database.clone());
        args.push("-e".to_string());
        args.push(query.to_string());
        args
    }

    /// Build a `sqlite3` CLI argument list for the given query.
    pub fn build_sqlite_command(&self, query: &str) -> Vec<String> {
        vec![
            "sqlite3".to_string(),
            self.config.database.clone(),
            query.to_string(),
        ]
    }

    /// Build a `mongosh` CLI argument list for the given query.
    pub fn build_mongosh_command(&self, query: &str) -> Vec<String> {
        let mut args = vec!["mongosh".to_string()];
        let conn = self.build_connection_string();
        args.push(conn);
        args.push("--eval".to_string());
        args.push(query.to_string());
        args
    }

    /// Build a `redis-cli` argument list for the given command.
    pub fn build_redis_command(&self, command: &str) -> Vec<String> {
        let mut args = vec!["redis-cli".to_string()];
        args.push("-h".to_string());
        args.push(self.config.host.clone());
        args.push("-p".to_string());
        args.push(self.config.port.to_string());
        if let Some(ref pass) = self.config.password {
            args.push("-a".to_string());
            args.push(pass.clone());
        }
        // Split command into individual tokens.
        for token in command.split_whitespace() {
            args.push(token.to_string());
        }
        args
    }
}

/// Validate a database configuration. Returns a list of error messages (empty if valid).
pub fn validate_config(config: &DatabaseConfig) -> Vec<String> {
    let mut errors = Vec::new();

    // SQLite and DuckDb don't need host/port validation.
    match config.engine {
        DatabaseEngine::SQLite | DatabaseEngine::DuckDb => {
            if config.database.is_empty() {
                errors.push("Database path must not be empty".to_string());
            }
        }
        _ => {
            if config.host.is_empty() {
                errors.push("Host must not be empty".to_string());
            }
            if config.port == 0 {
                errors.push("Port must be greater than 0".to_string());
            }
            if config.database.is_empty() {
                errors.push("Database name must not be empty".to_string());
            }
        }
    }

    errors
}

/// Generate CREATE TABLE DDL statements from table metadata.
pub fn generate_schema_sql(tables: &[TableInfo], engine: &DatabaseEngine) -> String {
    let mut sql = String::new();

    for table in tables {
        let qualified_name = if let Some(ref schema) = table.schema {
            format!("{}.{}", schema, table.name)
        } else {
            table.name.clone()
        };

        sql.push_str(&format!("CREATE TABLE {} (\n", qualified_name));

        let mut column_defs = Vec::new();
        let mut primary_keys = Vec::new();

        for col in &table.columns {
            let mut def = format!("  {} {}", col.name, col.data_type);
            if !col.nullable {
                def.push_str(" NOT NULL");
            }
            if let Some(ref default) = col.default_value {
                def.push_str(&format!(" DEFAULT {}", default));
            }
            if col.primary_key {
                primary_keys.push(col.name.clone());
            }
            column_defs.push(def);
        }

        if !primary_keys.is_empty() {
            column_defs.push(format!("  PRIMARY KEY ({})", primary_keys.join(", ")));
        }

        // Add foreign key constraints.
        for col in &table.columns {
            if let Some(ref fk) = col.foreign_key {
                column_defs.push(format!(
                    "  FOREIGN KEY ({}) REFERENCES {}",
                    col.name, fk
                ));
            }
        }

        sql.push_str(&column_defs.join(",\n"));
        sql.push_str("\n);\n\n");
    }

    let _ = engine; // Engine could be used for dialect-specific syntax in the future.
    sql
}

/// Parse CSV-formatted output (first row = headers) into a `QueryResult`.
pub fn parse_csv_result(csv: &str) -> QueryResult {
    let mut lines = csv.lines();

    let columns: Vec<String> = match lines.next() {
        Some(header) => header.split(',').map(|s| s.trim().to_string()).collect(),
        None => {
            return QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: 0,
                execution_time_ms: 0,
            }
        }
    };

    let mut rows = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<serde_json::Value> = line
            .split(',')
            .map(|s| {
                let trimmed = s.trim();
                // Try to parse as number first, then fall back to string.
                if let Ok(n) = trimmed.parse::<i64>() {
                    serde_json::Value::Number(n.into())
                } else if let Ok(f) = trimmed.parse::<f64>() {
                    serde_json::json!(f)
                } else {
                    serde_json::Value::String(trimmed.to_string())
                }
            })
            .collect();
        rows.push(values);
    }

    let row_count = rows.len() as u64;

    QueryResult {
        columns,
        rows,
        rows_affected: row_count,
        execution_time_ms: 0,
    }
}

/// Suggest a CREATE INDEX statement if any of the given query columns are non-primary-key
/// columns in the table.
pub fn suggest_index(table: &TableInfo, query_columns: &[&str]) -> Option<String> {
    let indexable: Vec<&str> = query_columns
        .iter()
        .filter(|qc| {
            table.columns.iter().any(|col| col.name == **qc && !col.primary_key)
        })
        .copied()
        .collect();

    if indexable.is_empty() {
        return None;
    }

    let index_name = format!("idx_{}_{}", table.name, indexable.join("_"));
    let cols = indexable.join(", ");
    Some(format!(
        "CREATE INDEX {} ON {} ({});",
        index_name, table.name, cols
    ))
}

/// Return a human-readable size estimate given a row count and average row size in bytes.
pub fn estimate_table_size(row_count: u64, avg_row_bytes: u64) -> String {
    let total = row_count * avg_row_bytes;
    if total < 1024 {
        format!("{} B", total)
    } else if total < 1024 * 1024 {
        format!("{:.1} KB", total as f64 / 1024.0)
    } else if total < 1024 * 1024 * 1024 {
        format!("{:.1} MB", total as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", total as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_database_engine_serialization() {
        let engine = DatabaseEngine::PostgreSQL;
        let json = serde_json::to_string(&engine).expect("serialize engine");
        assert_eq!(json, "\"PostgreSQL\"");
        let back: DatabaseEngine = serde_json::from_str(&json).expect("deserialize engine");
        assert_eq!(back, DatabaseEngine::PostgreSQL);

        let mysql_json = serde_json::to_string(&DatabaseEngine::MySQL).expect("serialize mysql");
        assert_eq!(mysql_json, "\"MySQL\"");
    }

    #[test]
    fn test_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.engine, DatabaseEngine::PostgreSQL);
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.database, "vibecody");
        assert_eq!(config.pool_size, 5);
        assert_eq!(config.timeout_secs, 30);
        assert!(!config.ssl);
        assert!(config.username.is_none());
        assert!(config.password.is_none());
        assert!(config.connection_string.is_none());
    }

    #[test]
    fn test_config_sqlite() {
        let config = DatabaseConfig::sqlite("/tmp/test.db");
        assert_eq!(config.engine, DatabaseEngine::SQLite);
        assert_eq!(config.database, "/tmp/test.db");
        assert_eq!(config.pool_size, 1);
        assert!(config.host.is_empty());
        assert_eq!(config.port, 0);
    }

    #[test]
    fn test_config_redis() {
        let config = DatabaseConfig::redis("10.0.0.1", 6380);
        assert_eq!(config.engine, DatabaseEngine::Redis);
        assert_eq!(config.host, "10.0.0.1");
        assert_eq!(config.port, 6380);
        assert_eq!(config.database, "0");
    }

    #[test]
    fn test_build_connection_string_postgres() {
        let mut config = DatabaseConfig::default();
        config.username = Some("admin".to_string());
        config.password = Some("secret".to_string());
        let client = DatabaseClient::new(config);
        let cs = client.build_connection_string();
        assert_eq!(cs, "postgresql://admin:secret@localhost:5432/vibecody");
    }

    #[test]
    fn test_build_connection_string_mysql() {
        let config = DatabaseConfig {
            engine: DatabaseEngine::MySQL,
            host: "db.example.com".to_string(),
            port: 3306,
            database: "myapp".to_string(),
            username: Some("root".to_string()),
            password: None,
            ssl: false,
            connection_string: None,
            pool_size: 10,
            timeout_secs: 15,
        };
        let client = DatabaseClient::new(config);
        let cs = client.build_connection_string();
        assert_eq!(cs, "mysql://root@db.example.com:3306/myapp");
    }

    #[test]
    fn test_build_connection_string_sqlite() {
        let config = DatabaseConfig::sqlite("/data/app.db");
        let client = DatabaseClient::new(config);
        assert_eq!(client.build_connection_string(), "sqlite:///data/app.db");
    }

    #[test]
    fn test_build_connection_string_mongodb() {
        let config = DatabaseConfig {
            engine: DatabaseEngine::MongoDB,
            host: "mongo.local".to_string(),
            port: 27017,
            database: "docs".to_string(),
            username: None,
            password: None,
            ssl: true,
            connection_string: None,
            pool_size: 5,
            timeout_secs: 30,
        };
        let client = DatabaseClient::new(config);
        let cs = client.build_connection_string();
        assert_eq!(cs, "mongodb://mongo.local:27017/docs?tls=true");
    }

    #[test]
    fn test_build_psql_command() {
        let mut config = DatabaseConfig::default();
        config.username = Some("pguser".to_string());
        let client = DatabaseClient::new(config);
        let cmd = client.build_psql_command("SELECT 1");
        assert_eq!(
            cmd,
            vec!["psql", "-h", "localhost", "-p", "5432", "-U", "pguser", "-d", "vibecody", "-c", "SELECT 1"]
        );
    }

    #[test]
    fn test_build_mysql_command() {
        let config = DatabaseConfig {
            engine: DatabaseEngine::MySQL,
            host: "127.0.0.1".to_string(),
            port: 3306,
            database: "testdb".to_string(),
            username: Some("root".to_string()),
            password: Some("pass".to_string()),
            ssl: false,
            connection_string: None,
            pool_size: 5,
            timeout_secs: 30,
        };
        let client = DatabaseClient::new(config);
        let cmd = client.build_mysql_command("SHOW TABLES");
        assert_eq!(
            cmd,
            vec!["mysql", "-h", "127.0.0.1", "-P", "3306", "-u", "root", "-ppass", "testdb", "-e", "SHOW TABLES"]
        );
    }

    #[test]
    fn test_build_sqlite_command() {
        let config = DatabaseConfig::sqlite("my.db");
        let client = DatabaseClient::new(config);
        let cmd = client.build_sqlite_command(".tables");
        assert_eq!(cmd, vec!["sqlite3", "my.db", ".tables"]);
    }

    #[test]
    fn test_build_redis_command() {
        let mut config = DatabaseConfig::redis("localhost", 6379);
        config.password = Some("redispass".to_string());
        let client = DatabaseClient::new(config);
        let cmd = client.build_redis_command("GET mykey");
        assert_eq!(
            cmd,
            vec!["redis-cli", "-h", "localhost", "-p", "6379", "-a", "redispass", "GET", "mykey"]
        );
    }

    #[test]
    fn test_validate_config_valid() {
        let config = DatabaseConfig::default();
        let errors = validate_config(&config);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_validate_config_empty_host() {
        let config = DatabaseConfig {
            engine: DatabaseEngine::PostgreSQL,
            host: String::new(),
            port: 5432,
            database: "test".to_string(),
            username: None,
            password: None,
            ssl: false,
            connection_string: None,
            pool_size: 5,
            timeout_secs: 30,
        };
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("Host")));
    }

    #[test]
    fn test_generate_schema_sql() {
        let tables = vec![TableInfo {
            name: "users".to_string(),
            schema: None,
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "SERIAL".to_string(),
                    nullable: false,
                    primary_key: true,
                    default_value: None,
                    foreign_key: None,
                },
                ColumnInfo {
                    name: "email".to_string(),
                    data_type: "VARCHAR(255)".to_string(),
                    nullable: false,
                    primary_key: false,
                    default_value: None,
                    foreign_key: None,
                },
            ],
            row_count: None,
            size_bytes: None,
        }];
        let sql = generate_schema_sql(&tables, &DatabaseEngine::PostgreSQL);
        assert!(sql.contains("CREATE TABLE users"));
        assert!(sql.contains("id SERIAL NOT NULL"));
        assert!(sql.contains("email VARCHAR(255) NOT NULL"));
        assert!(sql.contains("PRIMARY KEY (id)"));
    }

    #[test]
    fn test_parse_csv_result() {
        let csv = "id, name, age\n1, Alice, 30\n2, Bob, 25\n";
        let result = parse_csv_result(csv);
        assert_eq!(result.columns, vec!["id", "name", "age"]);
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[0][0], serde_json::json!(1));
        assert_eq!(result.rows[0][1], serde_json::Value::String("Alice".to_string()));
        assert_eq!(result.rows[1][2], serde_json::json!(25));
        assert_eq!(result.rows_affected, 2);
    }

    #[test]
    fn test_suggest_index() {
        let table = TableInfo {
            name: "orders".to_string(),
            schema: None,
            columns: vec![
                ColumnInfo {
                    name: "id".to_string(),
                    data_type: "INT".to_string(),
                    nullable: false,
                    primary_key: true,
                    default_value: None,
                    foreign_key: None,
                },
                ColumnInfo {
                    name: "customer_id".to_string(),
                    data_type: "INT".to_string(),
                    nullable: false,
                    primary_key: false,
                    default_value: None,
                    foreign_key: Some("customers(id)".to_string()),
                },
                ColumnInfo {
                    name: "status".to_string(),
                    data_type: "VARCHAR(20)".to_string(),
                    nullable: false,
                    primary_key: false,
                    default_value: None,
                    foreign_key: None,
                },
            ],
            row_count: Some(10000),
            size_bytes: None,
        };

        // Should suggest index on non-PK columns.
        let suggestion = suggest_index(&table, &["customer_id", "status"]);
        assert!(suggestion.is_some());
        let idx = suggestion.unwrap();
        assert!(idx.contains("idx_orders_customer_id_status"));
        assert!(idx.contains("customer_id, status"));

        // Querying only the PK should return None.
        let no_suggestion = suggest_index(&table, &["id"]);
        assert!(no_suggestion.is_none());
    }

    #[test]
    fn test_migration_scan_empty_dir() {
        let dir = tempdir().expect("create tempdir");
        let runner = MigrationRunner::new(dir.path(), DatabaseEngine::PostgreSQL);
        let migrations = runner.scan_migrations().expect("scan");
        assert!(migrations.is_empty());
    }

    #[test]
    fn test_migration_generate() {
        let dir = tempdir().expect("create tempdir");
        let runner = MigrationRunner::new(dir.path(), DatabaseEngine::PostgreSQL);

        let path = runner.generate_migration("create users").expect("generate");
        assert!(path.exists());

        let filename = path.file_name().unwrap().to_string_lossy();
        assert!(filename.starts_with("V001__create_users.sql"), "got: {}", filename);

        let content = fs::read_to_string(&path).expect("read migration");
        assert!(content.contains("-- UP"));
        assert!(content.contains("-- DOWN"));

        // Generate a second migration to verify incrementing.
        let path2 = runner.generate_migration("add orders").expect("generate second");
        let filename2 = path2.file_name().unwrap().to_string_lossy();
        assert!(filename2.starts_with("V002__add_orders.sql"), "got: {}", filename2);
    }

    #[test]
    fn test_migration_validate_duplicates() {
        let migrations = vec![
            Migration {
                version: "001".to_string(),
                name: "create_users".to_string(),
                sql_up: String::new(),
                sql_down: String::new(),
                status: MigrationStatus::Pending,
                applied_at: None,
            },
            Migration {
                version: "001".to_string(),
                name: "also_create_users".to_string(),
                sql_up: String::new(),
                sql_down: String::new(),
                status: MigrationStatus::Pending,
                applied_at: None,
            },
            Migration {
                version: "003".to_string(),
                name: "add_orders".to_string(),
                sql_up: String::new(),
                sql_down: String::new(),
                status: MigrationStatus::Pending,
                applied_at: None,
            },
        ];

        let runner = MigrationRunner::new(Path::new("/tmp"), DatabaseEngine::PostgreSQL);
        let errors = runner.validate_migrations(&migrations);
        assert!(errors.iter().any(|e| e.contains("Duplicate")));
        assert!(errors.iter().any(|e| e.contains("Gap")));
    }

    #[test]
    fn test_estimate_table_size() {
        assert_eq!(estimate_table_size(100, 5), "500 B");
        assert_eq!(estimate_table_size(1000, 512), "500.0 KB");
        assert_eq!(estimate_table_size(1_000_000, 1024), "976.6 MB");
        assert_eq!(estimate_table_size(1_000_000_000, 1024), "953.7 GB");
    }
}
