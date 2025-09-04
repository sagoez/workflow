//! RocksDB implementation of storage ports - optimized for event-driven applications

use std::{collections::HashMap, path::Path, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rocksdb::{ColumnFamily, DB, IteratorMode, Options};
use serde_json::Value;

use crate::{
    ports::storage::{
        Config, ConfigStore, EventData, EventStore, ExecutionStats, HistoryFilter, HistoryStore, WorkflowExecution
    },
    shared::WorkflowError
};

/// Column family names for different data types
const CF_EVENTS: &str = "events";
const CF_CONFIG: &str = "config";
const CF_HISTORY: &str = "history";
const CF_SNAPSHOTS: &str = "snapshots";
const CF_INDEXES: &str = "indexes";

/// RocksDB implementation optimized for event sourcing
pub struct RocksDbEventStore {
    db: Arc<DB>
}

impl RocksDbEventStore {
    /// Create a new RocksDB event store
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, WorkflowError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Optimize for write-heavy workloads (typical for event sourcing)
        opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB
        opts.set_max_write_buffer_number(3);
        opts.set_target_file_size_base(64 * 1024 * 1024); // 64MB
        opts.set_level_zero_file_num_compaction_trigger(4);
        opts.set_level_zero_slowdown_writes_trigger(20);
        opts.set_level_zero_stop_writes_trigger(36);
        opts.set_max_background_jobs(4);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        // Define column families
        let cf_names = vec![CF_EVENTS, CF_CONFIG, CF_HISTORY, CF_SNAPSHOTS, CF_INDEXES];

        let db = DB::open_cf(&opts, path, &cf_names)
            .map_err(|e| WorkflowError::Configuration(format!("Failed to open RocksDB: {}", e)))?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Get column family handle
    fn get_cf(&self, name: &str) -> Result<&ColumnFamily, WorkflowError> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| WorkflowError::Configuration(format!("Column family '{}' not found", name)))
    }

    /// Generate event key for storage
    fn event_key(aggregate_id: &str, sequence: u64) -> String {
        format!("{}:{:020}", aggregate_id, sequence)
    }

    /// Generate time-based index key
    fn time_index_key(timestamp: DateTime<Utc>, event_id: &str) -> String {
        format!("{}:{}", timestamp.timestamp_nanos_opt().unwrap_or(0), event_id)
    }

    /// Get the underlying database for sharing with other stores
    pub fn get_db(&self) -> Arc<DB> {
        self.db.clone()
    }
}

#[async_trait]
impl EventStore for RocksDbEventStore {
    async fn save_event(&self, event: &EventData) -> Result<(), WorkflowError> {
        let cf_events = self.get_cf(CF_EVENTS)?;
        let cf_indexes = self.get_cf(CF_INDEXES)?;

        // Serialize event
        let event_data = serde_json::to_vec(event).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

        // Generate keys
        let aggregate_id = event.aggregate_id.as_deref().unwrap_or("global");
        let sequence = self.get_next_sequence(aggregate_id)?;
        let event_key = Self::event_key(aggregate_id, sequence);
        let time_index_key = Self::time_index_key(event.timestamp, &event.event_id);

        // Create batch write for atomicity
        let mut batch = rocksdb::WriteBatch::default();

        // Store event
        batch.put_cf(cf_events, &event_key, &event_data);

        // Store time-based index
        batch.put_cf(cf_indexes, format!("time:{}", time_index_key), &event.event_id);

        // Store type-based index
        batch.put_cf(cf_indexes, format!("type:{}:{}", event.event_type, event.event_id), &event_key);

        // Update sequence counter
        batch.put_cf(cf_indexes, format!("seq:{}", aggregate_id), &sequence.to_be_bytes());

        // Write batch atomically
        self.db.write(batch).map_err(|e| WorkflowError::Event(format!("Failed to save event: {}", e)))?;

        Ok(())
    }

    async fn load_events(&self, aggregate_id: &str) -> Result<Vec<EventData>, WorkflowError> {
        let cf_events = self.get_cf(CF_EVENTS)?;

        let prefix = format!("{}:", aggregate_id);
        let iter = self.db.prefix_iterator_cf(cf_events, &prefix);

        let mut events = Vec::new();

        for item in iter {
            let (_, value) = item.map_err(|e| WorkflowError::Event(format!("Failed to read event: {}", e)))?;

            let event: EventData = serde_json::from_slice(&value)
                .map_err(|e| WorkflowError::Serialization(format!("Failed to deserialize event: {}", e)))?;

            events.push(event);
        }

        Ok(events)
    }

    async fn load_events_by_time_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>
    ) -> Result<Vec<EventData>, WorkflowError> {
        let cf_indexes = self.get_cf(CF_INDEXES)?;
        let cf_events = self.get_cf(CF_EVENTS)?;

        let from_key = format!("time:{}", from.timestamp_nanos_opt().unwrap_or(0));
        let to_key = format!("time:{}", to.timestamp_nanos_opt().unwrap_or(i64::MAX));

        let iter =
            self.db.iterator_cf(cf_indexes, IteratorMode::From(&from_key.as_bytes(), rocksdb::Direction::Forward));

        let mut events = Vec::new();

        for item in iter {
            let (key, event_id) = item.map_err(|e| WorkflowError::Event(format!("Failed to read index: {}", e)))?;

            let key_str = String::from_utf8_lossy(&key);
            if key_str.as_ref() > to_key.as_str() {
                break;
            }

            if !key_str.starts_with("time:") {
                continue;
            }

            // Get event by ID
            let event_id_str = String::from_utf8_lossy(&event_id);
            if let Some(event_data) = self
                .db
                .get_cf(cf_events, event_id_str.as_ref())
                .map_err(|e| WorkflowError::Event(format!("Failed to get event: {}", e)))?
            {
                let event: EventData = serde_json::from_slice(&event_data)
                    .map_err(|e| WorkflowError::Serialization(format!("Failed to deserialize event: {}", e)))?;

                events.push(event);
            }
        }

        Ok(events)
    }

    async fn get_latest_snapshot(&self, aggregate_id: &str) -> Result<Option<Value>, WorkflowError> {
        let cf_snapshots = self.get_cf(CF_SNAPSHOTS)?;

        if let Some(data) = self
            .db
            .get_cf(cf_snapshots, aggregate_id)
            .map_err(|e| WorkflowError::Event(format!("Failed to get snapshot: {}", e)))?
        {
            let value: Value =
                serde_json::from_slice(&data).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    async fn save_snapshot(&self, aggregate_id: &str, state: &Value) -> Result<(), WorkflowError> {
        let cf_snapshots = self.get_cf(CF_SNAPSHOTS)?;

        let data = serde_json::to_vec(state).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

        self.db
            .put_cf(cf_snapshots, aggregate_id, &data)
            .map_err(|e| WorkflowError::Event(format!("Failed to save snapshot: {}", e)))?;

        Ok(())
    }
}

impl RocksDbEventStore {
    /// Get the next sequence number for an aggregate
    fn get_next_sequence(&self, aggregate_id: &str) -> Result<u64, WorkflowError> {
        let cf_indexes = self.get_cf(CF_INDEXES)?;
        let key = format!("seq:{}", aggregate_id);

        if let Some(data) = self
            .db
            .get_cf(cf_indexes, &key)
            .map_err(|e| WorkflowError::Event(format!("Failed to get sequence: {}", e)))?
        {
            let bytes: [u8; 8] =
                data.try_into().map_err(|_| WorkflowError::Event("Invalid sequence data".to_string()))?;

            Ok(u64::from_be_bytes(bytes) + 1)
        } else {
            Ok(1)
        }
    }
}

/// RocksDB implementation of ConfigStore
pub struct RocksDbConfigStore {
    db: Arc<DB>
}

impl RocksDbConfigStore {
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    fn get_cf(&self) -> Result<&ColumnFamily, WorkflowError> {
        self.db
            .cf_handle(CF_CONFIG)
            .ok_or_else(|| WorkflowError::Configuration("Config column family not found".to_string()))
    }
}

#[async_trait]
impl ConfigStore for RocksDbConfigStore {
    async fn load_config(&self) -> Result<Config, WorkflowError> {
        let cf = self.get_cf()?;
        const CONFIG_KEY: &str = "app_config";

        if let Some(data) = self
            .db
            .get_cf(cf, CONFIG_KEY)
            .map_err(|e| WorkflowError::Configuration(format!("Failed to load config: {}", e)))?
        {
            let json_str = String::from_utf8(data)
                .map_err(|e| WorkflowError::Configuration(format!("Invalid UTF-8 in config: {}", e)))?;

            serde_json::from_str(&json_str)
                .map_err(|e| WorkflowError::Configuration(format!("Failed to parse config JSON: {}", e)))
        } else {
            // Return default config if not found
            Ok(Config::default())
        }
    }

    async fn save_config(&self, config: &Config) -> Result<(), WorkflowError> {
        let cf = self.get_cf()?;
        const CONFIG_KEY: &str = "app_config";

        let json_str = serde_json::to_string(config)
            .map_err(|e| WorkflowError::Configuration(format!("Failed to serialize config: {}", e)))?;

        self.db
            .put_cf(cf, CONFIG_KEY, json_str.as_bytes())
            .map_err(|e| WorkflowError::Configuration(format!("Failed to save config: {}", e)))?;

        Ok(())
    }

    async fn config_exists(&self) -> Result<bool, WorkflowError> {
        let cf = self.get_cf()?;
        const CONFIG_KEY: &str = "app_config";

        let exists = self
            .db
            .get_cf(cf, CONFIG_KEY)
            .map_err(|e| WorkflowError::Configuration(format!("Failed to check config existence: {}", e)))?
            .is_some();

        Ok(exists)
    }

    async fn init_config(&self) -> Result<(), WorkflowError> {
        // Create default config if it doesn't exist
        if !self.config_exists().await? {
            let default_config = Config::default();
            self.save_config(&default_config).await?;
        }

        Ok(())
    }
}

/// RocksDB implementation of HistoryStore
pub struct RocksDbHistoryStore {
    db: Arc<DB>
}

impl RocksDbHistoryStore {
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    fn get_cf(&self) -> Result<&ColumnFamily, WorkflowError> {
        self.db
            .cf_handle(CF_HISTORY)
            .ok_or_else(|| WorkflowError::Configuration("History column family not found".to_string()))
    }

    fn execution_key(execution: &WorkflowExecution) -> String {
        format!("{}:{}", execution.started_at.timestamp_nanos_opt().unwrap_or(0), execution.id)
    }
}

#[async_trait]
impl HistoryStore for RocksDbHistoryStore {
    async fn save_execution(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError> {
        let cf = self.get_cf()?;

        let data = serde_json::to_vec(execution).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

        let key = Self::execution_key(execution);

        self.db
            .put_cf(cf, &key, &data)
            .map_err(|e| WorkflowError::Event(format!("Failed to save execution: {}", e)))?;

        Ok(())
    }

    async fn get_history(&self, filter: &HistoryFilter) -> Result<Vec<WorkflowExecution>, WorkflowError> {
        let cf = self.get_cf()?;
        let mut executions = Vec::new();

        let iter = self.db.iterator_cf(cf, IteratorMode::End);
        let mut count = 0;
        let limit = filter.limit.unwrap_or(100);
        let offset = filter.offset.unwrap_or(0);

        for item in iter {
            if count < offset {
                count += 1;
                continue;
            }

            if executions.len() >= limit {
                break;
            }

            let (_, value) = item.map_err(|e| WorkflowError::Event(format!("Failed to read history: {}", e)))?;

            let execution: WorkflowExecution =
                serde_json::from_slice(&value).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

            // Apply filters
            if let Some(workflow_name) = &filter.workflow_name {
                if execution.workflow_name != *workflow_name {
                    continue;
                }
            }

            if let Some(user) = &filter.user {
                if execution.user != *user {
                    continue;
                }
            }

            if let Some(hostname) = &filter.hostname {
                if execution.hostname != *hostname {
                    continue;
                }
            }

            executions.push(execution);
            count += 1;
        }

        Ok(executions)
    }

    async fn get_stats(&self) -> Result<ExecutionStats, WorkflowError> {
        let cf = self.get_cf()?;

        let mut total_executions = 0u64;
        let mut successful_executions = 0u64;
        let mut workflow_counts: HashMap<String, u64> = HashMap::new();
        let mut total_duration = 0u64;
        let mut duration_count = 0u64;

        let iter = self.db.iterator_cf(cf, IteratorMode::Start);

        for item in iter {
            let (_, value) = item.map_err(|e| WorkflowError::Event(format!("Failed to read history: {}", e)))?;

            let execution: WorkflowExecution =
                serde_json::from_slice(&value).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

            total_executions += 1;

            if execution.exit_code == Some(0) {
                successful_executions += 1;
            }

            *workflow_counts.entry(execution.workflow_name.clone()).or_insert(0) += 1;

            if let Some(duration) = execution.duration_ms {
                total_duration += duration;
                duration_count += 1;
            }
        }

        let failed_executions = total_executions - successful_executions;

        let mut most_used_workflows: Vec<(String, u64)> = workflow_counts.into_iter().collect();
        most_used_workflows.sort_by(|a, b| b.1.cmp(&a.1));
        most_used_workflows.truncate(10);

        let average_duration_ms =
            if duration_count > 0 { Some(total_duration as f64 / duration_count as f64) } else { None };

        Ok(ExecutionStats {
            total_executions,
            successful_executions,
            failed_executions,
            most_used_workflows,
            average_duration_ms
        })
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<WorkflowExecution>, WorkflowError> {
        let cf = self.get_cf()?;
        let mut executions = Vec::new();
        let query_lower = query.to_lowercase();

        let iter = self.db.iterator_cf(cf, IteratorMode::End);

        for item in iter {
            if executions.len() >= limit {
                break;
            }

            let (_, value) = item.map_err(|e| WorkflowError::Event(format!("Failed to read history: {}", e)))?;

            let execution: WorkflowExecution =
                serde_json::from_slice(&value).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

            // Simple text search
            if execution.workflow_name.to_lowercase().contains(&query_lower)
                || execution.command.to_lowercase().contains(&query_lower)
                || execution.workflow_file.to_lowercase().contains(&query_lower)
            {
                executions.push(execution);
            }
        }

        Ok(executions)
    }
}
