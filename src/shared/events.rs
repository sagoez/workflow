//! Domain events for the workflow application
//!
//! This module contains all domain events that represent state changes in the system.

use std::any::Any;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::{Event, WorkflowState};

/// Workflow discovery event - emitted when workflows are discovered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDiscoveredEvent {
    pub event_id:       String,
    pub timestamp:      DateTime<Utc>,
    pub workflow_name:  String,
    pub file_path:      String,
    pub description:    String,
    pub argument_count: usize,
    pub tags:           Vec<String>
}

impl WorkflowDiscoveredEvent {
    pub fn new(
        workflow_name: String,
        file_path: String,
        description: String,
        argument_count: usize,
        tags: Vec<String>
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow_name,
            file_path,
            description,
            argument_count,
            tags
        }
    }
}

impl Event for WorkflowDiscoveredEvent {
    fn apply(&self, current_state: Option<&dyn Any>) -> Option<Box<dyn Any>> {
        let mut state = current_state.and_then(|s| s.downcast_ref::<WorkflowState>()).cloned().unwrap_or_default();

        state.add_workflow(
            self.workflow_name.clone(),
            self.file_path.clone(),
            self.description.clone(),
            self.argument_count,
            self.tags.clone()
        );

        Some(Box::new(state))
    }

    fn event_type(&self) -> &'static str {
        "WorkflowDiscovered"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "WorkflowState"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

/// Workflow selection event - emitted when a workflow is selected for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSelectedEvent {
    pub event_id:      String,
    pub timestamp:     DateTime<Utc>,
    pub workflow_name: String,
    pub file_path:     String,
    pub user:          String,
    pub session_id:    String
}

impl WorkflowSelectedEvent {
    pub fn new(workflow_name: String, file_path: String, user: String, session_id: String) -> Self {
        Self { event_id: Uuid::new_v4().to_string(), timestamp: Utc::now(), workflow_name, file_path, user, session_id }
    }
}

impl Event for WorkflowSelectedEvent {
    fn apply(&self, current_state: Option<&dyn Any>) -> Option<Box<dyn Any>> {
        let mut state = current_state.and_then(|s| s.downcast_ref::<WorkflowState>()).cloned().unwrap_or_default();

        state.set_selected_workflow(self.workflow_name.clone(), self.file_path.clone());
        Some(Box::new(state))
    }

    fn event_type(&self) -> &'static str {
        "WorkflowSelected"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "WorkflowState"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

/// Configuration initialization event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationInitializedEvent {
    pub event_id:      String,
    pub timestamp:     DateTime<Utc>,
    pub config_dir:    String,
    pub workflows_dir: String,
    pub i18n_dir:      String
}

impl ConfigurationInitializedEvent {
    pub fn new(config_dir: String, workflows_dir: String, i18n_dir: String) -> Self {
        Self { event_id: Uuid::new_v4().to_string(), timestamp: Utc::now(), config_dir, workflows_dir, i18n_dir }
    }
}

impl Event for ConfigurationInitializedEvent {
    fn apply(&self, _current_state: Option<&dyn Any>) -> Option<Box<dyn Any>> {
        // Configuration events might have their own state or be stateless
        // For now, return empty state to indicate success
        Some(Box::new(()))
    }

    fn event_type(&self) -> &'static str {
        "ConfigurationInitialized"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "ConfigState"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

/// Default implementation for WorkflowState
impl Default for WorkflowState {
    fn default() -> Self {
        Self { discovered_workflows: Vec::new(), selected_workflow: None, execution_history: Vec::new() }
    }
}

// Additional events that were missing
/// Workflow started event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStartedEvent {
    pub event_id:      String,
    pub timestamp:     DateTime<Utc>,
    pub workflow_name: String,
    pub command:       String,
    pub user:          String,
    pub hostname:      String,
    pub session_id:    String
}

impl WorkflowStartedEvent {
    pub fn new(workflow_name: String, command: String, user: String, hostname: String, session_id: String) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow_name,
            command,
            user,
            hostname,
            session_id
        }
    }
}

impl Event for WorkflowStartedEvent {
    fn apply(&self, current_state: Option<&dyn Any>) -> Option<Box<dyn Any>> {
        let mut state = current_state.and_then(|s| s.downcast_ref::<WorkflowState>()).cloned().unwrap_or_default();

        state.execution_history.push(crate::shared::WorkflowExecution {
            workflow_name: self.workflow_name.clone(),
            command:       self.command.clone(),
            timestamp:     self.timestamp,
            success:       true, // Will be updated on completion
            user:          self.user.clone(),
            hostname:      self.hostname.clone()
        });

        Some(Box::new(state))
    }

    fn event_type(&self) -> &'static str {
        "WorkflowStarted"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "WorkflowState"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

/// Language changed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageChangedEvent {
    pub event_id:     String,
    pub timestamp:    DateTime<Utc>,
    pub old_language: String,
    pub new_language: String,
    pub user:         String
}

impl LanguageChangedEvent {
    pub fn new(old_language: String, new_language: String, user: String) -> Self {
        Self { event_id: Uuid::new_v4().to_string(), timestamp: Utc::now(), old_language, new_language, user }
    }
}

impl Event for LanguageChangedEvent {
    fn apply(&self, _current_state: Option<&dyn Any>) -> Option<Box<dyn Any>> {
        // Language changes might affect a different state type
        Some(Box::new(()))
    }

    fn event_type(&self) -> &'static str {
        "LanguageChanged"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "ConfigState"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

/// Workflow arguments resolved event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowArgumentsResolvedEvent {
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub workflow_name: String,
    pub arguments: std::collections::HashMap<String, String>,
    pub session_id: String
}

impl WorkflowArgumentsResolvedEvent {
    pub fn new(workflow_name: String, arguments: std::collections::HashMap<String, String>, session_id: String) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow_name,
            arguments,
            session_id
        }
    }
}

impl Event for WorkflowArgumentsResolvedEvent {
    fn apply(&self, current_state: Option<&dyn Any>) -> Option<Box<dyn Any>> {
        let state = current_state
            .and_then(|s| s.downcast_ref::<WorkflowState>())
            .cloned()
            .unwrap_or_default();
        Some(Box::new(state))
    }

    fn event_type(&self) -> &'static str {
        "WorkflowArgumentsResolved"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "WorkflowState"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

/// Workflows synced event with proper fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowsSyncedEvent {
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub repository_url: String,
    pub commit_hash: String,
    pub workflows_count: usize
}

impl WorkflowsSyncedEvent {
    pub fn new(repository_url: String, commit_hash: String, workflows_count: usize) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            repository_url,
            commit_hash,
            workflows_count
        }
    }
}

impl Event for WorkflowsSyncedEvent {
    fn apply(&self, _current_state: Option<&dyn Any>) -> Option<Box<dyn Any>> {
        Some(Box::new(()))
    }

    fn event_type(&self) -> &'static str {
        "WorkflowsSynced"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "SyncState"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

/// Other missing events (simplified implementations)
pub type WorkflowCompletedEvent = WorkflowStartedEvent;
pub type WorkflowFailedEvent = WorkflowStartedEvent;
pub type ResourceUrlChangedEvent = LanguageChangedEvent;

impl WorkflowState {
    pub fn add_workflow(
        &mut self,
        name: String,
        file_path: String,
        description: String,
        argument_count: usize,
        tags: Vec<String>
    ) {
        let workflow_info = crate::shared::WorkflowInfo { name, file_path, description, tags, argument_count };
        self.discovered_workflows.push(workflow_info);
    }

    pub fn set_selected_workflow(&mut self, name: String, file_path: String) {
        self.selected_workflow = Some((name, file_path));
    }
}
