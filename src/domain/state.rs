//! Application state management with phase-based workflow progression
//!
//! This module defines the different phases of workflow execution and their corresponding
//! state types, ensuring type safety and proper state transitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::workflow::Workflow;

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
    LanguageSet(LanguageSetState),
    /// Current language has been retrieved
    CurrentLanguageRetrieved(CurrentLanguageRetrievedState),
    /// Available languages have been listed
    AvailableLanguagesListed(AvailableLanguagesListedState)
}

impl Default for WorkflowState {
    fn default() -> Self {
        WorkflowState::Initial(InitialState)
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

// **********************
// Phase-Specific State Types
// **********************

/// Initial state - no workflows have been discovered yet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialState;

/// State after workflows have been discovered from the filesystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowsDiscoveredState {
    /// All workflows discovered from the filesystem
    pub discovered_workflows: Vec<Workflow>
}

/// State after workflows have been listed/displayed to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowsListedState {
    /// All workflows discovered from the filesystem
    pub discovered_workflows: Vec<Workflow>
}

/// State after a specific workflow has been selected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSelectedState {
    /// All workflows discovered from the filesystem
    pub discovered_workflows: Vec<Workflow>,
    /// The workflow that has been selected (guaranteed non-null)
    pub selected_workflow:    Workflow
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
    pub resolved_arguments:   std::collections::HashMap<String, String>
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
    pub resolved_arguments:   std::collections::HashMap<String, String>
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

// **********************
// Language Management States
// **********************

/// State after language has been set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageSetState {
    /// The language that was set
    pub language: String,
    /// Timestamp when language was set
    pub set_at:   DateTime<Utc>
}

/// State after current language has been retrieved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentLanguageRetrievedState {
    /// The current language
    pub language:     String,
    /// Timestamp when language was retrieved
    pub retrieved_at: DateTime<Utc>
}

/// State after available languages have been listed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableLanguagesListedState {
    /// List of available languages
    pub languages: Vec<String>,
    /// Timestamp when languages were listed
    pub listed_at: DateTime<Utc>
}
