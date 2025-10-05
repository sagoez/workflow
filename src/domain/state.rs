//! Application state management with phase-based workflow progression
//!
//! This module defines the different phases of workflow execution and their corresponding
//! state types, ensuring type safety and proper state transitions.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::domain::workflow::Workflow;

/// Unified state enum representing different phases of workflow execution
///
/// Each phase contains only the data that is guaranteed to exist at that point,
/// providing compile-time guarantees about data availability.
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
pub struct SyncRequestedState {
    #[tabled(rename = "Phase")]
    pub phase:           String,
    /// Remote repository URL that was synced
    #[tabled(rename = "Remote URL")]
    pub remote_url:      String,
    /// Branch that was synced
    #[tabled(rename = "Branch")]
    pub branch:          String,
    /// SSH key that was used (raw for serialization)
    #[tabled(skip)]
    pub ssh_key:         Option<String>,
    /// SSH key that was used (formatted for display)
    #[serde(skip)]
    #[tabled(rename = "SSH Key")]
    pub ssh_key_display: String
}

impl SyncRequestedState {
    pub fn new(remote_url: String, branch: String, ssh_key: Option<String>) -> Self {
        let ssh_key_display = ssh_key.as_deref().unwrap_or("None").to_string();
        Self { phase: "Sync Requested".to_string(), remote_url, branch, ssh_key, ssh_key_display }
    }
}

// **********************
// Phase-Specific State Types
// **********************

/// Initial state - no workflows have been discovered yet
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
pub struct InitialState {
    #[tabled(rename = "Phase")]
    pub phase: String
}

impl Default for InitialState {
    fn default() -> Self {
        Self { phase: "Initial - No workflows discovered".to_string() }
    }
}

/// State after workflows have been discovered from the filesystem
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
pub struct WorkflowsDiscoveredState {
    /// All workflows discovered from the filesystem
    #[tabled(skip)]
    pub discovered_workflows: Vec<Workflow>,
    #[tabled(rename = "Phase")]
    pub phase:                String,
    #[tabled(rename = "Workflows Count")]
    pub workflows_count:      usize
}

impl WorkflowsDiscoveredState {
    pub fn new(discovered_workflows: Vec<Workflow>) -> Self {
        let count = discovered_workflows.len();
        Self { discovered_workflows, phase: "Workflows Discovered".to_string(), workflows_count: count }
    }
}

/// State after workflows have been listed/displayed to the user
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
pub struct WorkflowsListedState {
    /// All workflows discovered from the filesystem
    #[tabled(skip)]
    pub discovered_workflows: Vec<Workflow>,
    /// The phase of the workflow
    #[tabled(rename = "Phase")]
    pub phase:                String,
    /// The number of workflows discovered from the filesystem
    #[tabled(rename = "Workflows Count")]
    pub workflows_count:      usize
}

impl WorkflowsListedState {
    pub fn new(discovered_workflows: Vec<Workflow>) -> Self {
        let count = discovered_workflows.len();
        Self { discovered_workflows, phase: "Workflows Listed".to_string(), workflows_count: count }
    }
}

/// State after a specific workflow has been selected
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
pub struct WorkflowSelectedState {
    /// All workflows discovered from the filesystem
    #[tabled(skip)]
    pub discovered_workflows: Vec<Workflow>,
    /// The workflow that has been selected (guaranteed non-null)
    #[tabled(skip)]
    pub selected_workflow:    Workflow,
    /// The phase of the workflow
    #[tabled(rename = "Phase")]
    pub phase:                String,
    /// The name of the selected workflow
    #[tabled(rename = "Selected Workflow")]
    pub workflow_name:        String,
    /// The description of the selected workflow
    #[tabled(rename = "Description")]
    pub description:          String
}

impl WorkflowSelectedState {
    pub fn new(discovered_workflows: Vec<Workflow>, selected_workflow: Workflow) -> Self {
        let workflow_name = selected_workflow.name.clone();
        let description = selected_workflow.description.clone();
        Self {
            discovered_workflows,
            selected_workflow,
            phase: "Workflow Selected".to_string(),
            workflow_name,
            description
        }
    }
}

/// State after the selected workflow has been started
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
pub struct WorkflowStartedState {
    /// All workflows discovered from the filesystem
    #[tabled(skip)]
    pub discovered_workflows: Vec<Workflow>,
    /// The workflow that has been selected (guaranteed non-null)
    #[tabled(skip)]
    pub selected_workflow:    Workflow,
    /// Unique execution ID for this workflow run (guaranteed non-null)
    #[tabled(rename = "Execution ID")]
    pub execution_id:         String,
    /// The phase of the workflow
    #[tabled(rename = "Phase")]
    pub phase:                String,
    /// The name of the selected workflow
    #[tabled(rename = "Workflow")]
    pub workflow_name:        String
}

impl WorkflowStartedState {
    pub fn new(discovered_workflows: Vec<Workflow>, selected_workflow: Workflow, execution_id: String) -> Self {
        let workflow_name = selected_workflow.name.clone();
        Self {
            discovered_workflows,
            selected_workflow,
            execution_id,
            phase: "Workflow Started".to_string(),
            workflow_name
        }
    }
}

/// State after workflow arguments have been resolved
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
pub struct WorkflowArgumentsResolvedState {
    /// All workflows discovered from the filesystem
    #[tabled(skip)]
    pub discovered_workflows: Vec<Workflow>,
    /// The workflow that has been selected (guaranteed non-null)
    #[tabled(skip)]
    pub selected_workflow:    Workflow,
    /// Unique execution ID for this workflow run (guaranteed non-null)
    #[tabled(rename = "Execution ID")]
    pub execution_id:         String,
    /// Resolved arguments for the workflow (guaranteed non-null)
    #[tabled(skip)]
    pub resolved_arguments:   std::collections::HashMap<String, String>,
    /// The phase of the workflow
    #[tabled(rename = "Phase")]
    pub phase:                String,
    /// The name of the selected workflow
    #[tabled(rename = "Workflow")]
    pub workflow_name:        String
}

impl WorkflowArgumentsResolvedState {
    pub fn new(
        discovered_workflows: Vec<Workflow>,
        selected_workflow: Workflow,
        execution_id: String,
        resolved_arguments: std::collections::HashMap<String, String>
    ) -> Self {
        let workflow_name = selected_workflow.name.clone();
        Self {
            discovered_workflows,
            selected_workflow,
            execution_id,
            resolved_arguments,
            phase: "Arguments Resolved".to_string(),
            workflow_name
        }
    }
}

/// State after workflow execution has completed
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
pub struct WorkflowCompletedState {
    /// All workflows discovered from the filesystem
    #[tabled(skip)]
    pub discovered_workflows: Vec<Workflow>,
    /// The workflow that was executed (guaranteed non-null)
    #[tabled(skip)]
    pub completed_workflow:   Workflow,
    /// The execution ID that completed (guaranteed non-null)
    #[tabled(rename = "Execution ID")]
    pub execution_id:         String,
    /// The resolved arguments that were used (guaranteed non-null)
    #[tabled(skip)]
    pub resolved_arguments:   HashMap<String, String>,
    /// The phase of the workflow
    #[tabled(rename = "Phase")]
    pub phase:                String,
    /// The name of the selected workflow
    #[tabled(rename = "Workflow")]
    pub workflow_name:        String
}

impl WorkflowCompletedState {
    pub fn new(
        discovered_workflows: Vec<Workflow>,
        completed_workflow: Workflow,
        execution_id: String,
        resolved_arguments: std::collections::HashMap<String, String>
    ) -> Self {
        let workflow_name = completed_workflow.name.clone();
        Self {
            discovered_workflows,
            completed_workflow,
            execution_id,
            resolved_arguments,
            phase: "Workflow Completed".to_string(),
            workflow_name
        }
    }
}

/// State after workflows have been synced from git repository
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
pub struct WorkflowsSyncedState {
    /// The phase of the workflow
    #[tabled(rename = "Phase")]
    pub phase:             String,
    /// Remote repository URL that was synced
    #[tabled(rename = "Remote URL")]
    pub remote_url:        String,
    /// Branch that was synced
    #[tabled(rename = "Branch")]
    pub branch:            String,
    /// Commit ID that was synced
    #[tabled(rename = "Commit ID")]
    pub commit_id:         String,
    /// Number of workflows synced
    #[tabled(rename = "Synced Count")]
    pub synced_count:      u32,
    /// Timestamp of sync (raw for serialization)
    #[tabled(skip)]
    pub synced_at:         DateTime<Utc>,
    /// Timestamp of sync (formatted for display)
    #[serde(skip)]
    #[tabled(rename = "Synced At")]
    pub synced_at_display: String
}

impl WorkflowsSyncedState {
    pub fn new(
        remote_url: String,
        branch: String,
        commit_id: String,
        synced_count: u32,
        synced_at: DateTime<Utc>
    ) -> Self {
        let synced_at_display = synced_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        Self {
            phase: "Workflows Synced".to_string(),
            remote_url,
            branch,
            commit_id,
            synced_count,
            synced_at,
            synced_at_display
        }
    }
}

/// State after language has been set
#[derive(Debug, Clone, Serialize, Deserialize, Tabled)]
pub struct LanguageSetState {
    /// The phase of the workflow
    #[tabled(rename = "Phase")]
    pub phase:          String,
    /// The language that was set
    #[tabled(rename = "Language")]
    pub language:       String,
    /// Timestamp when language was set (raw for serialization)
    #[tabled(skip)]
    pub set_at:         DateTime<Utc>,
    /// Timestamp when language was set (formatted for display)
    #[serde(skip)]
    #[tabled(rename = "Set At")]
    pub set_at_display: String
}

impl LanguageSetState {
    pub fn new(language: String, set_at: DateTime<Utc>) -> Self {
        let set_at_display = set_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        Self { phase: "Language Set".to_string(), language, set_at, set_at_display }
    }
}
