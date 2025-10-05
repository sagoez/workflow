//! Application state management with phase-based workflow progression
//!
//! This module defines the different phases of workflow execution and their corresponding
//! state types, ensuring type safety and proper state transitions.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::domain::workflow::Workflow;

/// Trait for displaying workflow state in a table format
/// This trait enforces that each state provides display information without polluting the state
/// structs
pub trait StateDisplay {
    /// Get the phase name for display
    fn phase_name(&self) -> String;

    /// Get the table rows for display (key-value pairs)
    fn table_rows(&self) -> Vec<(String, String)>;
}

/// Wrapper type for displaying states in a table
#[derive(Tabled)]
pub struct StateTableRow {
    pub key:   String,
    pub value: String
}

/// Unified state enum representing different phases of workflow execution
///
/// Each phase contains only the data that is guaranteed to exist at that point,
/// providing compile-time guarantees about data availability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowState {
    /// Initial state - no workflows discovered yet
    Initial(InitialState),
    /// Workflows have been discovered from filesystem
    WorkflowsDiscovered(WorkflowsDiscoveredState),
    /// Workflows have been listed to the user
    WorkflowsListed(WorkflowsListedState),
    /// A specific workflow has been selected
    WorkflowSelected(WorkflowSelectedState),
    /// The selected workflow has been started
    WorkflowStarted(WorkflowStartedState),
    /// Workflow arguments have been resolved
    WorkflowArgumentsResolved(WorkflowArgumentsResolvedState),
    /// Workflow execution has completed
    WorkflowCompleted(WorkflowCompletedState),
    /// Workflows have been synced from git repository
    SyncRequested(SyncRequestedState),
    /// Workflows have been synced from git repository
    WorkflowsSynced(WorkflowsSyncedState),
    /// Language has been set
    LanguageSet(LanguageSetState)
}

impl Default for WorkflowState {
    fn default() -> Self {
        WorkflowState::Initial(InitialState::default())
    }
}

// **********************
// Sync Management States
// **********************

/// State after sync has been requested
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequestedState {
    /// Remote repository URL that was synced
    pub remote_url: String,
    /// Branch that was synced
    pub branch:     String,
    /// SSH key that was used
    pub ssh_key:    Option<String>
}

impl SyncRequestedState {
    pub fn new(remote_url: String, branch: String, ssh_key: Option<String>) -> Self {
        Self { remote_url, branch, ssh_key }
    }
}

// **********************
// Phase-Specific State Types
// **********************

/// Initial state - no workflows have been discovered yet
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitialState;

/// State after workflows have been discovered from the filesystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowsDiscoveredState {
    /// All workflows discovered from the filesystem
    pub discovered_workflows: Vec<Workflow>
}

impl WorkflowsDiscoveredState {
    pub fn new(discovered_workflows: Vec<Workflow>) -> Self {
        Self { discovered_workflows }
    }
}

/// State after workflows have been listed/displayed to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowsListedState {
    /// All workflows discovered from the filesystem
    pub discovered_workflows: Vec<Workflow>
}

impl WorkflowsListedState {
    pub fn new(discovered_workflows: Vec<Workflow>) -> Self {
        Self { discovered_workflows }
    }
}

/// State after a specific workflow has been selected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSelectedState {
    /// All workflows discovered from the filesystem
    pub discovered_workflows: Vec<Workflow>,
    /// The workflow that has been selected (guaranteed non-null)
    pub selected_workflow:    Workflow
}

impl WorkflowSelectedState {
    pub fn new(discovered_workflows: Vec<Workflow>, selected_workflow: Workflow) -> Self {
        Self { discovered_workflows, selected_workflow }
    }
}

/// State after the selected workflow has been started
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStartedState {
    /// All workflows discovered from the filesystem
    pub discovered_workflows: Vec<Workflow>,
    /// The workflow that has been selected (guaranteed non-null)
    pub selected_workflow:    Workflow,
    /// Unique execution ID for this workflow run (guaranteed non-null)
    pub execution_id:         String
}

impl WorkflowStartedState {
    pub fn new(discovered_workflows: Vec<Workflow>, selected_workflow: Workflow, execution_id: String) -> Self {
        Self { discovered_workflows, selected_workflow, execution_id }
    }
}

/// State after workflow arguments have been resolved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowArgumentsResolvedState {
    /// All workflows discovered from the filesystem
    pub discovered_workflows: Vec<Workflow>,
    /// The workflow that has been selected (guaranteed non-null)
    pub selected_workflow:    Workflow,
    /// Unique execution ID for this workflow run (guaranteed non-null)
    pub execution_id:         String,
    /// Resolved arguments for the workflow (guaranteed non-null)
    pub resolved_arguments:   HashMap<String, String>
}

impl WorkflowArgumentsResolvedState {
    pub fn new(
        discovered_workflows: Vec<Workflow>,
        selected_workflow: Workflow,
        execution_id: String,
        resolved_arguments: HashMap<String, String>
    ) -> Self {
        Self { discovered_workflows, selected_workflow, execution_id, resolved_arguments }
    }
}

/// State after workflow execution has completed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCompletedState {
    /// All workflows discovered from the filesystem
    pub discovered_workflows: Vec<Workflow>,
    /// The workflow that was executed (guaranteed non-null)
    pub completed_workflow:   Workflow,
    /// The execution ID that completed (guaranteed non-null)
    pub execution_id:         String,
    /// The resolved arguments that were used (guaranteed non-null)
    pub resolved_arguments:   HashMap<String, String>
}

impl WorkflowCompletedState {
    pub fn new(
        discovered_workflows: Vec<Workflow>,
        completed_workflow: Workflow,
        execution_id: String,
        resolved_arguments: HashMap<String, String>
    ) -> Self {
        Self { discovered_workflows, completed_workflow, execution_id, resolved_arguments }
    }
}

/// State after workflows have been synced from git repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowsSyncedState {
    /// Remote repository URL that was synced
    pub remote_url:   String,
    /// Branch that was synced
    pub branch:       String,
    /// Commit ID that was synced
    pub commit_id:    String,
    /// Number of workflows synced
    pub synced_count: u32,
    /// Timestamp of sync
    pub synced_at:    DateTime<Utc>
}

impl WorkflowsSyncedState {
    pub fn new(
        remote_url: String,
        branch: String,
        commit_id: String,
        synced_count: u32,
        synced_at: DateTime<Utc>
    ) -> Self {
        Self { remote_url, branch, commit_id, synced_count, synced_at }
    }
}

/// State after language has been set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageSetState {
    /// The language that was set
    pub language: String,
    /// Timestamp when language was set
    pub set_at:   DateTime<Utc>
}

impl LanguageSetState {
    pub fn new(language: String, set_at: DateTime<Utc>) -> Self {
        Self { language, set_at }
    }
}

impl StateDisplay for InitialState {
    fn phase_name(&self) -> String {
        crate::t!("state_phase_initial").to_string()
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        vec![(crate::t!("state_field_status").to_string(), crate::t!("state_status_no_workflows").to_string())]
    }
}

impl StateDisplay for WorkflowsDiscoveredState {
    fn phase_name(&self) -> String {
        crate::t!("state_phase_workflows_discovered").to_string()
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        vec![(crate::t!("state_field_workflows_count").to_string(), self.discovered_workflows.len().to_string())]
    }
}

impl StateDisplay for WorkflowsListedState {
    fn phase_name(&self) -> String {
        crate::t!("state_phase_workflows_listed").to_string()
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        vec![(crate::t!("state_field_workflows_count").to_string(), self.discovered_workflows.len().to_string())]
    }
}

impl StateDisplay for WorkflowSelectedState {
    fn phase_name(&self) -> String {
        crate::t!("state_phase_workflow_selected").to_string()
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        vec![
            (crate::t!("state_field_selected").to_string(), self.selected_workflow.name.clone()),
            (crate::t!("state_field_description").to_string(), self.selected_workflow.description.clone()),
        ]
    }
}

impl StateDisplay for WorkflowStartedState {
    fn phase_name(&self) -> String {
        crate::t!("state_phase_workflow_started").to_string()
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        vec![
            (crate::t!("state_field_workflow").to_string(), self.selected_workflow.name.clone()),
            (crate::t!("state_field_execution_id").to_string(), self.execution_id.clone()),
        ]
    }
}

impl StateDisplay for WorkflowArgumentsResolvedState {
    fn phase_name(&self) -> String {
        crate::t!("state_phase_arguments_resolved").to_string()
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        let mut rows = vec![
            (crate::t!("state_field_workflow").to_string(), self.selected_workflow.name.clone()),
            (crate::t!("state_field_execution_id").to_string(), self.execution_id.clone()),
        ];
        for (key, value) in &self.resolved_arguments {
            rows.push((key.clone(), value.clone()));
        }
        rows
    }
}

impl StateDisplay for WorkflowCompletedState {
    fn phase_name(&self) -> String {
        crate::t!("state_phase_workflow_completed").to_string()
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        let mut rows = vec![
            (crate::t!("state_field_workflow").to_string(), self.completed_workflow.name.clone()),
            (crate::t!("state_field_execution_id").to_string(), self.execution_id.clone()),
        ];
        for (key, value) in &self.resolved_arguments {
            rows.push((key.clone(), value.clone()));
        }
        rows
    }
}

impl StateDisplay for SyncRequestedState {
    fn phase_name(&self) -> String {
        crate::t!("state_phase_sync_requested").to_string()
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        let mut rows = vec![
            (crate::t!("state_field_remote_url").to_string(), self.remote_url.clone()),
            (crate::t!("state_field_branch").to_string(), self.branch.clone()),
        ];
        if self.ssh_key.is_some() {
            rows.push((crate::t!("state_field_ssh_key").to_string(), "****** (hidden)".to_string()));
        }
        rows
    }
}

impl StateDisplay for WorkflowsSyncedState {
    fn phase_name(&self) -> String {
        crate::t!("state_phase_workflows_synced").to_string()
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        vec![
            (crate::t!("state_field_remote_url").to_string(), self.remote_url.clone()),
            (crate::t!("state_field_branch").to_string(), self.branch.clone()),
            (crate::t!("state_field_commit_id").to_string(), self.commit_id.clone()),
            (crate::t!("state_field_synced_count").to_string(), self.synced_count.to_string()),
            (
                crate::t!("state_field_synced_at").to_string(),
                self.synced_at.format("%Y-%m-%d %H:%M:%S UTC").to_string()
            ),
        ]
    }
}

impl StateDisplay for LanguageSetState {
    fn phase_name(&self) -> String {
        crate::t!("state_phase_language_set").to_string()
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        vec![
            (crate::t!("state_field_language").to_string(), self.language.clone()),
            (crate::t!("state_field_set_at").to_string(), self.set_at.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
        ]
    }
}

impl StateDisplay for WorkflowState {
    fn phase_name(&self) -> String {
        match self {
            WorkflowState::Initial(s) => s.phase_name(),
            WorkflowState::WorkflowsDiscovered(s) => s.phase_name(),
            WorkflowState::WorkflowsListed(s) => s.phase_name(),
            WorkflowState::WorkflowSelected(s) => s.phase_name(),
            WorkflowState::WorkflowStarted(s) => s.phase_name(),
            WorkflowState::WorkflowArgumentsResolved(s) => s.phase_name(),
            WorkflowState::WorkflowCompleted(s) => s.phase_name(),
            WorkflowState::SyncRequested(s) => s.phase_name(),
            WorkflowState::WorkflowsSynced(s) => s.phase_name(),
            WorkflowState::LanguageSet(s) => s.phase_name()
        }
    }

    fn table_rows(&self) -> Vec<(String, String)> {
        match self {
            WorkflowState::Initial(s) => s.table_rows(),
            WorkflowState::WorkflowsDiscovered(s) => s.table_rows(),
            WorkflowState::WorkflowsListed(s) => s.table_rows(),
            WorkflowState::WorkflowSelected(s) => s.table_rows(),
            WorkflowState::WorkflowStarted(s) => s.table_rows(),
            WorkflowState::WorkflowArgumentsResolved(s) => s.table_rows(),
            WorkflowState::WorkflowCompleted(s) => s.table_rows(),
            WorkflowState::SyncRequested(s) => s.table_rows(),
            WorkflowState::WorkflowsSynced(s) => s.table_rows(),
            WorkflowState::LanguageSet(s) => s.table_rows()
        }
    }
}
