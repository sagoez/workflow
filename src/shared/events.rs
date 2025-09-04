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
        let mut state = current_state
            .and_then(|s| s.downcast_ref::<WorkflowState>())
            .cloned()
            .unwrap_or_default();

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
    pub event_id:       String,
    pub timestamp:      DateTime<Utc>,
    pub workflow_name:  String,
    pub file_path:      String,
    pub user:           String,
    pub session_id:     String
}

impl WorkflowSelectedEvent {
    pub fn new(workflow_name: String, file_path: String, user: String, session_id: String) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow_name,
            file_path,
            user,
            session_id
        }
    }
}

impl Event for WorkflowSelectedEvent {
    fn apply(&self, current_state: Option<&dyn Any>) -> Option<Box<dyn Any>> {
        let mut state = current_state
            .and_then(|s| s.downcast_ref::<WorkflowState>())
            .cloned()
            .unwrap_or_default();

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
    pub event_id:       String,
    pub timestamp:      DateTime<Utc>,
    pub config_dir:     String,
    pub workflows_dir:  String,
    pub i18n_dir:       String
}

impl ConfigurationInitializedEvent {
    pub fn new(config_dir: String, workflows_dir: String, i18n_dir: String) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            config_dir,
            workflows_dir,
            i18n_dir
        }
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
        Self {
            discovered_workflows: Vec::new(),
            selected_workflow: None,
            execution_history: Vec::new()
        }
    }
}

impl WorkflowState {
    pub fn add_workflow(
        &mut self,
        name: String,
        file_path: String,
        description: String,
        argument_count: usize,
        tags: Vec<String>
    ) {
        // Add logic to update discovered workflows
        // This is a simplified version - you'd implement the full logic
    }

    pub fn set_selected_workflow(&mut self, name: String, file_path: String) {
        self.selected_workflow = Some((name, file_path));
    }
}
