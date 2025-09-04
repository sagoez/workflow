//! Workflow service - orchestrates workflow-related operations
//!
//! This service handles all workflow-related business logic including:
//! - Discovering and listing workflows
//! - Executing workflows with argument resolution
//! - Managing workflow history and statistics

use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    i18n,
    ports::{
        filesystem::FileSystem,
        storage::{ConfigStore, EventData, EventStore, HistoryStore},
        ui::{ProgressIndicator, Prompter, Renderer, WorkflowInfo}
    },
    shared::{CommandContext, Workflow, WorkflowError, events::*}
};

/// Service for workflow operations  
#[allow(dead_code)]
pub struct WorkflowService {
    filesystem:    Arc<dyn FileSystem>,
    event_store:   Arc<dyn EventStore>,
    history_store: Arc<dyn HistoryStore>,
    config_store:  Arc<dyn ConfigStore>,
    prompter:      Arc<dyn Prompter>,
    renderer:      Arc<dyn Renderer>,
    progress:      Arc<dyn ProgressIndicator>
}

impl WorkflowService {
    pub fn new(
        filesystem: Arc<dyn FileSystem>,
        event_store: Arc<dyn EventStore>,
        history_store: Arc<dyn HistoryStore>,
        config_store: Arc<dyn ConfigStore>,
        prompter: Arc<dyn Prompter>,
        renderer: Arc<dyn Renderer>,
        progress: Arc<dyn ProgressIndicator>
    ) -> Self {
        Self { filesystem, event_store, history_store, config_store, prompter, renderer, progress }
    }

    /// List all available workflows
    pub async fn list_workflows(&self, _context: &CommandContext) -> Result<Vec<WorkflowInfo>, WorkflowError> {
        // Get workflows directory from config
        let config = self.config_store.load_config().await?;
        let workflows_dir = self.get_workflows_dir(&config).await?;

        // Scan for workflow files
        let workflow_files = self.scan_workflow_files(&workflows_dir).await?;

        let mut workflows = Vec::new();
        let mut events = Vec::new();

        // Process each workflow file
        for file_path in workflow_files {
            match self.load_workflow_from_file(&file_path).await {
                Ok(workflow) => {
                    let workflow_info = WorkflowInfo {
                        name:           workflow.name.clone(),
                        file_path:      file_path.clone(),
                        description:    workflow.description.clone(),
                        tags:           workflow.tags.clone(),
                        argument_count: workflow.arguments.len()
                    };

                    // Create discovery event
                    let event = WorkflowDiscoveredEvent::new(
                        workflow.name.clone(),
                        file_path.clone(),
                        workflow.description.clone(),
                        workflow.arguments.len(),
                        workflow.tags.clone()
                    );

                    events.push(event);
                    workflows.push(workflow_info);
                }
                Err(e) => {
                    // Log error but continue processing other files
                    eprintln!("Warning: Failed to load workflow from {}: {}", file_path, e);
                }
            }
        }

        // Store discovery events
        for event in events {
            let event_data = self.event_to_data(&event)?;
            self.event_store.save_event(&event_data).await?;
        }

        // Display workflows using renderer
        self.renderer.display_workflow_list(&workflows).await?;

        Ok(workflows)
    }

    /// Execute a specific workflow
    pub async fn execute_workflow(&self, file_path: &str, context: &CommandContext) -> Result<String, WorkflowError> {
        // Load the workflow
        let workflow = self.load_workflow_from_file(file_path).await?;

        // Show workflow info
        let workflow_info = WorkflowInfo {
            name:           workflow.name.clone(),
            file_path:      file_path.to_string(),
            description:    workflow.description.clone(),
            tags:           workflow.tags.clone(),
            argument_count: workflow.arguments.len()
        };
        self.renderer.display_workflow_info(&workflow_info).await?;

        // Resolve arguments interactively
        let arguments = self.resolve_workflow_arguments(&workflow, context).await?;

        // Render the command template
        let final_command = self.render_workflow_command(&workflow, &arguments).await?;

        // Emit workflow events
        let selected_event = WorkflowSelectedEvent::new(
            workflow.name.clone(),
            file_path.to_string(),
            context.user.clone(),
            context.session_id.clone()
        );

        let args_resolved_event =
            WorkflowArgumentsResolvedEvent::new(workflow.name.clone(), arguments.clone(), context.session_id.clone());

        let started_event = WorkflowStartedEvent::new(
            workflow.name.clone(),
            final_command.clone(),
            context.user.clone(),
            context.hostname.clone(),
            context.session_id.clone()
        );

        // Store events
        let selected_data = self.event_to_data(&selected_event)?;
        let args_data = self.event_to_data(&args_resolved_event)?;
        let started_data = self.event_to_data(&started_event)?;

        self.event_store.save_event(&selected_data).await?;
        self.event_store.save_event(&args_data).await?;
        self.event_store.save_event(&started_data).await?;

        // Display the final command
        self.renderer.display_command(&final_command).await?;

        // TODO: Copy to clipboard using utility function

        Ok(final_command)
    }

    /// Select and execute a workflow interactively
    pub async fn select_and_execute_workflow(&self, context: &CommandContext) -> Result<String, WorkflowError> {
        // List available workflows
        let workflows = self.list_workflows(context).await?;

        if workflows.is_empty() {
            return Err(WorkflowError::Validation(i18n::t("service_no_workflows_available")));
        }

        // Prompt user to select a workflow
        let workflow_names: Vec<String> = workflows.iter().map(|w| w.name.clone()).collect();
        let selected_name = self.prompter.prompt_select("Select a workflow", workflow_names).await?;

        // Find the selected workflow
        let selected_workflow = workflows
            .iter()
            .find(|w| w.name == selected_name)
            .ok_or_else(|| WorkflowError::Validation(i18n::t("service_selected_workflow_not_found")))?;

        // Execute the selected workflow
        self.execute_workflow(&selected_workflow.file_path, context).await
    }

    // Private helper methods

    async fn get_workflows_dir(&self, _config: &crate::ports::storage::Config) -> Result<String, WorkflowError> {
        // TODO: Get workflows directory from config or use default
        Ok("./resource".to_string()) // Temporary default
    }

    async fn scan_workflow_files(&self, workflows_dir: &str) -> Result<Vec<String>, WorkflowError> {
        let dir_path = Path::new(workflows_dir);

        if !dir_path.exists() {
            return Err(WorkflowError::Configuration(i18n::t_params(
                "service_workflows_dir_not_found",
                &[workflows_dir]
            )));
        }

        let entries = self.filesystem.read_dir(dir_path).await?;
        let mut workflow_files = Vec::new();

        for entry in entries {
            if let Some(extension) = entry.extension() {
                if extension == "yaml" || extension == "yml" {
                    // Skip config.yaml as it's not a workflow file
                    if let Some(file_name) = entry.file_name() {
                        if file_name != "config.yaml" {
                            workflow_files.push(entry.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(workflow_files)
    }

    async fn load_workflow_from_file(&self, file_path: &str) -> Result<Workflow, WorkflowError> {
        let content = self.filesystem.read_to_string(Path::new(file_path)).await?;
        Workflow::from_yaml(&content)
            .map_err(|e| WorkflowError::Validation(i18n::t_params("service_invalid_workflow_yaml", &[&e.to_string()])))
    }

    async fn resolve_workflow_arguments(
        &self,
        workflow: &Workflow,
        _context: &CommandContext
    ) -> Result<HashMap<String, String>, WorkflowError> {
        let mut resolved_args = HashMap::new();

        for arg in &workflow.arguments {
            let value = match &arg.default_value {
                Some(default) => {
                    // Prompt with default value
                    self.prompter.prompt_text(&arg.description, Some(default)).await?
                }
                None => {
                    // Prompt without default
                    self.prompter.prompt_text(&arg.description, None).await?
                }
            };

            resolved_args.insert(arg.name.clone(), value);
        }

        Ok(resolved_args)
    }

    async fn render_workflow_command(
        &self,
        workflow: &Workflow,
        arguments: &HashMap<String, String>
    ) -> Result<String, WorkflowError> {
        // TODO: Use Tera templating engine to render the command
        // For now, simple string replacement
        let mut command = workflow.command.clone();

        for (key, value) in arguments {
            let placeholder = format!("{{{{{}}}}}", key);
            command = command.replace(&placeholder, value);
        }

        Ok(command)
    }

    fn event_to_data<E: serde::Serialize>(&self, event: &E) -> Result<EventData, WorkflowError> {
        // Helper to convert any event to EventData
        // This is a simplified version - in practice you'd want more sophisticated serialization
        let data = serde_json::to_value(event).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

        Ok(EventData {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: "GenericEvent".to_string(), // TODO: Extract from event
            aggregate_id: None,
            timestamp: chrono::Utc::now(),
            data,
            metadata: None
        })
    }
}
