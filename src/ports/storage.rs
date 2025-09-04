//! Storage ports - interfaces for data persistence

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::shared::WorkflowError;

/// Serializable event data for storage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventData {
    pub event_id:     String,
    pub event_type:   String,
    pub aggregate_id: Option<String>,
    pub timestamp:    DateTime<Utc>,
    pub data:         Value,
    pub metadata:     Option<Value>
}

/// Port for storing and retrieving events
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Save an event to the store
    async fn save_event(&self, event: &EventData) -> Result<(), WorkflowError>;

    /// Load events for a specific aggregate
    async fn load_events(&self, aggregate_id: &str) -> Result<Vec<EventData>, WorkflowError>;

    /// Load events within a time range
    async fn load_events_by_time_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>
    ) -> Result<Vec<EventData>, WorkflowError>;

    /// Get the latest snapshot for an aggregate
    async fn get_latest_snapshot(&self, aggregate_id: &str) -> Result<Option<Value>, WorkflowError>;

    /// Save a snapshot of aggregate state
    async fn save_snapshot(&self, aggregate_id: &str, state: &Value) -> Result<(), WorkflowError>;
}

/// Configuration structure for the workflow CLI
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Currently selected language
    pub language:     String,
    /// Git URL for workflows repository
    pub resource_url: Option<String>,
    /// SSH private key path for Git authentication
    pub ssh_key_path: Option<String>
}

impl Default for Config {
    fn default() -> Self {
        Self { language: "en".to_string(), resource_url: None, ssh_key_path: None }
    }
}

/// Port for storing configuration data
#[async_trait]
pub trait ConfigStore: Send + Sync {
    /// Load the complete configuration
    async fn load_config(&self) -> Result<Config, WorkflowError>;

    /// Save the complete configuration
    async fn save_config(&self, config: &Config) -> Result<(), WorkflowError>;

    /// Check if configuration file exists
    async fn config_exists(&self) -> Result<bool, WorkflowError>;

    /// Initialize configuration with default values if it doesn't exist
    async fn init_config(&self) -> Result<(), WorkflowError>;
}

/// Port for storing workflow execution history
#[async_trait]
pub trait HistoryStore: Send + Sync {
    /// Save a workflow execution record
    async fn save_execution(&self, execution: &WorkflowExecution) -> Result<(), WorkflowError>;

    /// Get execution history with optional filters
    async fn get_history(&self, filter: &HistoryFilter) -> Result<Vec<WorkflowExecution>, WorkflowError>;

    /// Get execution statistics
    async fn get_stats(&self) -> Result<ExecutionStats, WorkflowError>;

    /// Search execution history
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<WorkflowExecution>, WorkflowError>;
}

/// Workflow execution record
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowExecution {
    pub id:            String,
    pub workflow_name: String,
    pub workflow_file: String,
    pub command:       String,
    pub arguments:     HashMap<String, String>,
    pub started_at:    DateTime<Utc>,
    pub completed_at:  Option<DateTime<Utc>>,
    pub duration_ms:   Option<u64>,
    pub exit_code:     Option<i32>,
    pub hostname:      String,
    pub user:          String,
    pub session_id:    String
}

/// History filter for querying executions
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HistoryFilter {
    pub workflow_name: Option<String>,
    pub user:          Option<String>,
    pub hostname:      Option<String>,
    pub from_date:     Option<DateTime<Utc>>,
    pub to_date:       Option<DateTime<Utc>>,
    pub limit:         Option<usize>,
    pub offset:        Option<usize>
}

/// Execution statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionStats {
    pub total_executions:      u64,
    pub successful_executions: u64,
    pub failed_executions:     u64,
    pub most_used_workflows:   Vec<(String, u64)>,
    pub average_duration_ms:   Option<f64>
}
