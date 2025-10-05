use std::{collections::HashMap, fs};

use async_trait::async_trait;
use chrono::Utc;
use clipboard::ClipboardProvider;
use inquire::{Select, Text};
use tokio::process::Command as TokioCommand;
use uuid::Uuid;

use crate::{
    AppContext,
    adapter::storage::EventStoreType,
    domain::{
        command::{
            CompleteWorkflowCommand, DiscoverWorkflowsCommand, DiscoverWorkflowsData, GetCurrentLanguageCommand,
            GetCurrentStorageCommand, InteractivelySelectWorkflowCommand, InteractivelySelectWorkflowData,
            ListAggregatesCommand, ListLanguagesCommand, ListWorkflowsCommand, PurgeStorageCommand,
            RecordSyncResultCommand, ReplayAggregateCommand, ResolveArgumentsCommand, ResolveArgumentsData,
            SetLanguageCommand, SetStorageCommand, StartWorkflowCommand, SyncWorkflowsCommand, WorkflowCommand
        },
        engine::EngineContext,
        error::WorkflowError,
        event::{
            AggregateReplayedEvent, AggregatesListedEvent, AvailableWorkflowsListedEvent, LanguageSetEvent,
            SyncRequestedEvent, WorkflowArgumentsResolvedEvent, WorkflowCompletedEvent, WorkflowDiscoveredEvent,
            WorkflowEvent, WorkflowSelectedEvent, WorkflowStartedEvent, WorkflowsSyncedEvent
        },
        state::WorkflowState,
        workflow::{ArgumentType, Workflow, WorkflowArgument}
    },
    i18n::Language,
    port::{command::Command, git::CloneOptions},
    t, t_params
};

// TODO: Move to separate files

/// Macro to implement Command trait for WorkflowCommand enum
/// Similar to the impl_event macro for WorkflowEvent
macro_rules! impl_command {
    ($enum_name:ident { $($variant:ident($field:ident)),* $(,)? }) => {
        #[async_trait]
        impl Command for $enum_name {
            type Error = WorkflowError;
            type LoadedData = Box<dyn std::any::Any + Send + Sync>; // Generic type since each command has
                // different data

            async fn load(
                &self,
                context: &EngineContext,
                app_context: &AppContext,
                current_state: &WorkflowState
            ) -> Result<Self::LoadedData, Self::Error> {
                match self {
                    $(
                        $enum_name::$variant($field) => {
                            let data = $field.load(context, app_context, current_state).await?;
                            Ok(Box::new(data) as Box<dyn std::any::Any + Send + Sync>)
                        }
                    )*
                }
            }

            fn validate(&self, loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
                match self {
                    $(
                        $enum_name::$variant($field) => {
                            let data = loaded_data.downcast_ref().ok_or_else(||
                                WorkflowError::Generic(t!("error_failed_to_downcast_loaded_data").to_string()))?;
                            $field.validate(data)
                        }
                    )*
                }
            }

            async fn emit(
                &self,
                loaded_data: &Self::LoadedData,
                context: &EngineContext,
                app_context: &AppContext,
                current_state: &WorkflowState
            ) -> Result<Vec<WorkflowEvent>, Self::Error> {
                match self {
                    $(
                        $enum_name::$variant($field) => {
                            let data = loaded_data.downcast_ref().ok_or_else(||
                                WorkflowError::Generic(t!("error_failed_to_downcast_loaded_data").to_string()))?;
                            $field.emit(data, context, app_context, current_state).await
                        }
                    )*
                }
            }

            async fn effect(
                &self,
                loaded_data: &Self::LoadedData,
                previous_state: &WorkflowState,
                current_state: &WorkflowState,
                context: &EngineContext,
                app_context: &AppContext
            ) -> Result<(), Self::Error> {
                match self {
                    $(
                        $enum_name::$variant($field) => {
                            let loaded_data = loaded_data.downcast_ref().ok_or_else(||
                                WorkflowError::Generic(t!("error_failed_to_downcast_loaded_data").to_string()))?;
                            $field.effect(loaded_data, previous_state, current_state, context, app_context).await
                        }
                    )*
                }
            }

            fn name(&self) -> &'static str {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.name(),
                    )*
                }
            }

            fn description(&self) -> &'static str {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.description(),
                    )*
                }
            }

            fn is_interactive(&self) -> bool {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.is_interactive(),
                    )*
                }
            }

            fn is_mutating(&self) -> bool {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.is_mutating(),
                    )*
                }
            }
        }
    };
}

impl_command!(WorkflowCommand {
    DiscoverWorkflows(cmd),
    ListWorkflows(cmd),
    InteractivelySelectWorkflow(cmd),
    StartWorkflow(cmd),
    CompleteWorkflow(cmd),
    ResolveArguments(cmd),
    SyncWorkflows(cmd),
    RecordSyncResult(cmd),
    SetLanguage(cmd),
    GetCurrentLanguage(cmd),
    ListLanguages(cmd),
    SetStorage(cmd),
    GetCurrentStorage(cmd),
    ListAggregates(cmd),
    ReplayAggregate(cmd),
    PurgeStorage(cmd)
});

#[async_trait]
impl Command for DiscoverWorkflowsCommand {
    type Error = WorkflowError;
    type LoadedData = DiscoverWorkflowsData;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let workflows_dir = app_context.config.workflows_dir.clone();

        if !workflows_dir.exists() {
            return Ok(DiscoverWorkflowsData { workflows: vec![] });
        }

        // Run blocking I/O operations in a separate thread pool to avoid blocking the async runtime
        let workflows = tokio::task::spawn_blocking(move || -> Result<Vec<Workflow>, WorkflowError> {
            let entries = fs::read_dir(&workflows_dir).map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
            let mut workflows = Vec::new();

            for entry in entries {
                let entry = entry.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
                let path = entry.path();

                if path.is_file()
                    && let Some(extension) = path.extension()
                    && (extension == "yaml" || extension == "yml")
                {
                    let content = fs::read_to_string(&path).map_err(|e| WorkflowError::FileSystem(e.to_string()))?;

                    let workflow: Workflow =
                        serde_yaml::from_str(&content).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

                    workflows.push(workflow);
                }
            }

            workflows.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(workflows)
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to discover workflows: {}", e)))??;

        Ok(DiscoverWorkflowsData { workflows })
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let mut events = Vec::new();

        for workflow in &loaded_data.workflows {
            let event = WorkflowDiscoveredEvent {
                event_id:  Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                workflow:  workflow.clone(),
                file_path: format!("{}.yaml", workflow.name) // TODO: store actual file path
            };
            events.push(WorkflowEvent::WorkflowDiscovered(event));
        }

        Ok(events)
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        // DiscoverWorkflowsCommand is a data loading command - no output needed
        // Display is handled by ListWorkflowsCommand
        Ok(())
    }

    fn name(&self) -> &'static str {
        "discover-workflows"
    }

    fn description(&self) -> &'static str {
        "Discovers available workflow files"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}

#[async_trait]
impl Command for InteractivelySelectWorkflowCommand {
    type Error = WorkflowError;
    type LoadedData = InteractivelySelectWorkflowData;

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        if let WorkflowState::WorkflowsDiscovered(state) = current_state {
            let workflows: Vec<Workflow> = state.discovered_workflows.to_vec();

            let selected_workflow =
                Select::new(&t!("select_workflow"), workflows.clone()).with_page_size(10).prompt().map_err(|e| {
                    WorkflowError::Validation(t_params!("error_selection_failed", &["workflow", &e.to_string()]))
                })?;

            Ok(InteractivelySelectWorkflowData { workflow: selected_workflow.clone() })
        } else {
            Err(WorkflowError::Validation(t!("error_workflows_not_discovered_yet")))
        }
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let event = WorkflowSelectedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  loaded_data.workflow.clone(),
            user:      context.workflow_context.user.clone()
        };

        Ok(vec![WorkflowEvent::WorkflowSelected(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::WorkflowSelected(state) => {
                let workflow = &state.selected_workflow;
                println!("{}", t_params!("cli_selected_workflow", &[&workflow.name]));
                println!("{}", t_params!("cli_selected_workflow_description", &[&workflow.description]));
                if !workflow.arguments.is_empty() {
                    println!(
                        "{}",
                        t_params!("cli_selected_workflow_arguments", &[&workflow.arguments.len().to_string()])
                    );
                }
            }
            _ => {
                println!("{}", t!("error_no_workflow_selected"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "select-workflow"
    }

    fn description(&self) -> &'static str {
        "Selects and loads a specific workflow"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[async_trait]
impl Command for ListWorkflowsCommand {
    type Error = WorkflowError;
    type LoadedData = ();

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let discovered_workflows = match current_state {
            WorkflowState::WorkflowsDiscovered(state) => &state.discovered_workflows,
            WorkflowState::Initial(_) => &vec![],
            _ => return Err(WorkflowError::Validation(t!("error_workflows_not_discovered_yet")))
        };

        let workflow_names: Vec<String> = discovered_workflows.iter().map(|w| w.name.clone()).collect();
        let event = AvailableWorkflowsListedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            workflows: workflow_names
        };

        Ok(vec![WorkflowEvent::AvailableWorkflowsListed(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        println!("{}", t!("cli_available_workflows"));
        println!();
        match current_state {
            WorkflowState::WorkflowsListed(state) => {
                for workflow in &state.discovered_workflows {
                    println!("  - {}", workflow.name);
                }
                if state.discovered_workflows.is_empty() {
                    println!("  {}", t!("no_workflows_found"));
                }
            }
            _ => {
                println!("  {}", t!("no_workflows_found"));
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "list-workflows"
    }

    fn description(&self) -> &'static str {
        "Lists all available workflow YAML files"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}

#[async_trait]
impl Command for StartWorkflowCommand {
    type Error = WorkflowError;
    type LoadedData = ();

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        context: &EngineContext,
        _app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        match current_state {
            WorkflowState::WorkflowSelected(_) => {
                let event = WorkflowStartedEvent {
                    event_id:     Uuid::new_v4().to_string(),
                    timestamp:    Utc::now(),
                    user:         context.workflow_context.user.clone(),
                    hostname:     context.workflow_context.hostname.clone(),
                    execution_id: Uuid::new_v4().to_string()
                };

                Ok(vec![WorkflowEvent::WorkflowStarted(event)])
            }
            _ => Err(WorkflowError::Validation(t!("error_no_workflow_selected_to_start")))
        }
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::WorkflowStarted(state) => {
                let workflow = &state.selected_workflow;
                println!("{}", t_params!("cli_starting_workflow", &[&workflow.name]));
                println!("{}", t_params!("cli_starting_workflow_description", &[&workflow.description]));
                println!("{}", t_params!("cli_starting_workflow_command", &[&workflow.command]));
            }
            _ => {
                println!("{}", t!("error_no_workflow_started"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "start-workflow"
    }

    fn description(&self) -> &'static str {
        "Starts the selected workflow"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[async_trait]
impl Command for ResolveArgumentsCommand {
    type Error = WorkflowError;
    type LoadedData = ResolveArgumentsData;

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let workflow = match current_state {
            WorkflowState::WorkflowStarted(state) => state.selected_workflow.clone(),
            _ => return Err(WorkflowError::Validation(t!("error_no_workflow_started_to_resolve_arguments")))
        };

        let resolved_arguments = resolve_workflow_arguments(&workflow.arguments).await.map_err(|e| {
            WorkflowError::Validation(t_params!("error_failed_to_resolve_arguments", &[&e.to_string()]))
        })?;

        Ok(ResolveArgumentsData { workflow, resolved_arguments })
    }

    fn validate(&self, loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        for arg in &loaded_data.workflow.arguments {
            if !loaded_data.resolved_arguments.contains_key(&arg.name) {
                return Err(WorkflowError::Validation(t_params!("error_argument_not_resolved", &[&arg.name])));
            }
        }
        Ok(())
    }

    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        match current_state {
            WorkflowState::WorkflowStarted(_) => {
                let event = WorkflowArgumentsResolvedEvent {
                    event_id:  Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    arguments: loaded_data.resolved_arguments.clone()
                };

                Ok(vec![WorkflowEvent::WorkflowArgumentsResolved(event)])
            }
            _ => Err(WorkflowError::Validation(t!("error_no_workflow_execution_in_progress")))
        }
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::WorkflowArgumentsResolved(state) => {
                let workflow = &state.selected_workflow;
                println!("{}", t_params!("cli_resolved_arguments_for_workflow", &[&workflow.name]));

                for (key, value) in &state.resolved_arguments {
                    println!("  {} = {}", key, value);
                }

                // Use default Tera instance since we only need render_str (no file templates needed)
                let mut tera = tera::Tera::default();
                let mut context = tera::Context::new();

                for (key, value) in &state.resolved_arguments {
                    context.insert(key, value);
                }

                let rendered_command = tera.render_str(&workflow.command, &context).map_err(|e| {
                    WorkflowError::Validation(t_params!("error_failed_to_render_command_template", &[&e.to_string()]))
                })?;

                println!("{}", t!("cli_generated_command"));
                println!("{}", rendered_command);

                match copy_to_clipboard(&rendered_command) {
                    Ok(()) => {
                        println!("{}", t!("cli_command_copied_to_clipboard"));
                        println!("{}", t!("cli_command_can_now_be_pasted_and_executed_in_terminal"));
                    }
                    Err(e) => {
                        println!("{}", t_params!("cli_failed_to_copy_to_clipboard", &[&e.to_string()]));
                        println!("{}", t!("cli_command_can_now_be_pasted_and_executed_in_terminal"));
                    }
                }
            }
            _ => {
                println!("{}", t!("error_no_arguments_resolved"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "resolve-arguments"
    }

    fn description(&self) -> &'static str {
        "Interactively resolves workflow arguments with dynamic resolution"
    }

    fn is_interactive(&self) -> bool {
        true // This command shows interactive prompts and menus
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[async_trait]
impl Command for CompleteWorkflowCommand {
    type Error = WorkflowError;
    type LoadedData = ();

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        match current_state {
            WorkflowState::WorkflowArgumentsResolved(_) => {
                let event = WorkflowCompletedEvent { event_id: Uuid::new_v4().to_string(), timestamp: Utc::now() };

                Ok(vec![WorkflowEvent::WorkflowCompleted(event)])
            }
            _ => Err(WorkflowError::Validation(t!("error_no_workflow_ready_to_complete")))
        }
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::WorkflowCompleted(state) => {
                let workflow = &state.completed_workflow;
                println!("{}", t_params!("cli_completed_workflow", &[&workflow.name]));
            }
            _ => {
                println!("{}", t!("error_no_workflow_completed"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "complete-workflow"
    }

    fn description(&self) -> &'static str {
        "Marks the current workflow as completed"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

fn resolve_static_enum_argument(arg: &WorkflowArgument, variants: &[String]) -> Result<String, WorkflowError> {
    let prompt = format!("Select {}", arg.name);
    let selection = Select::new(&prompt, variants.to_vec())
        .with_page_size(10)
        .prompt()
        .map_err(|e| WorkflowError::Validation(t_params!("error_selection_failed", &[&arg.name, &e.to_string()])))?;

    Ok(selection)
}

/// Helper function to copy text to clipboard
fn copy_to_clipboard(text: &str) -> Result<(), WorkflowError> {
    let mut ctx = clipboard::ClipboardContext::new().map_err(|e| {
        WorkflowError::Validation(t_params!("error_failed_to_create_clipboard_context", &[&e.to_string()]))
    })?;
    ctx.set_contents(text.to_owned()).map_err(|e| {
        WorkflowError::Validation(t_params!("error_failed_to_set_clipboard_contents", &[&e.to_string()]))
    })?;
    Ok(())
}

/// Helper function to resolve workflow arguments
async fn resolve_workflow_arguments(arguments: &[WorkflowArgument]) -> Result<HashMap<String, String>, WorkflowError> {
    let mut argument_values = HashMap::new();

    for arg in arguments {
        let value = resolve_argument(arg, &argument_values).await?;
        argument_values.insert(arg.name.clone(), value);
    }

    Ok(argument_values)
}

/// Helper function to resolve a workflow argument
async fn resolve_argument(
    arg: &WorkflowArgument,
    current_values: &HashMap<String, String>
) -> Result<String, WorkflowError> {
    match arg.arg_type {
        ArgumentType::Enum => {
            if let Some(enum_variants) = &arg.enum_variants {
                resolve_static_enum_argument(arg, enum_variants)
            } else if let (Some(enum_command), Some(_enum_name)) = (&arg.enum_command, &arg.enum_name) {
                resolve_enum_argument(arg, enum_command, current_values).await
            } else {
                Err(WorkflowError::Validation(t_params!("error_enum_argument_missing_configuration", &[&arg.name])))
            }
        }
        ArgumentType::Text | ArgumentType::Number | ArgumentType::Boolean => resolve_simple_argument(arg)
    }
}

/// Helper function to resolve an enum argument
async fn resolve_enum_argument(
    arg: &WorkflowArgument,
    enum_command: &str,
    current_values: &HashMap<String, String>
) -> Result<String, WorkflowError> {
    let resolved_command = if let Some(ref_arg) = &arg.dynamic_resolution {
        if let Some(ref_value) = current_values.get(ref_arg) {
            enum_command.replace(&format!("{{{{{}}}}}", ref_arg), ref_value)
        } else {
            return Err(WorkflowError::Validation(t_params!("error_dynamic_resolution_failed", &[ref_arg])));
        }
    } else {
        enum_command.to_string()
    };

    println!("Executing: {}", resolved_command);

    let output = TokioCommand::new("sh")
        .arg("-c")
        .arg(&resolved_command)
        .output()
        .await
        .map_err(|e| WorkflowError::Validation(t_params!("error_failed_to_execute_command", &[&e.to_string()])))?;

    if !output.status.success() {
        return Err(WorkflowError::Validation(t_params!(
            "error_command_failed",
            &[&String::from_utf8_lossy(&output.stderr)]
        )));
    }

    let options: Vec<String> = String::from_utf8(output.stdout)
        .map_err(|e| WorkflowError::Validation(t_params!("error_failed_to_parse_command_output", &[&e.to_string()])))?
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if options.is_empty() {
        return Err(WorkflowError::Validation(t_params!("error_no_options_found", &[&arg.name])));
    }

    let prompt = format!("Select {}", arg.name);
    let selection = Select::new(&prompt, options)
        .with_page_size(10)
        .prompt()
        .map_err(|e| WorkflowError::Validation(t_params!("error_selection_failed", &[&arg.name, &e.to_string()])))?;

    Ok(selection)
}

fn resolve_simple_argument(arg: &WorkflowArgument) -> Result<String, WorkflowError> {
    let prompt = format!("Enter {}", arg.name);
    let mut text_input = Text::new(&prompt);

    if let Some(default_value) = &arg.default_value
        && !default_value.is_empty()
        && default_value != "~"
    {
        text_input = text_input.with_default(default_value);
    }

    let result = text_input
        .prompt()
        .map_err(|e| WorkflowError::Validation(t_params!("error_input_failed", &[&arg.name, &e.to_string()])))?;

    Ok(result)
}

// **********************
// **********************

#[derive(Debug, Clone)]
pub struct SyncWorkflowsData {
    pub remote_url: String,
    pub branch:     String,
    pub ssh_key:    Option<String>
}

#[async_trait::async_trait]
impl Command for SyncWorkflowsCommand {
    type Error = WorkflowError;
    type LoadedData = SyncWorkflowsData;

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let remote_url =
            self.remote_url.clone().unwrap_or_else(|| "git@github.com:sagoez/workflow-vault.git".to_string());

        Ok(SyncWorkflowsData { remote_url, branch: self.branch.clone(), ssh_key: self.ssh_key.clone() })
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let event = SyncRequestedEvent {
            event_id:   Uuid::new_v4().to_string(),
            timestamp:  chrono::Utc::now(),
            remote_url: loaded_data.remote_url.clone(),
            branch:     loaded_data.branch.clone(),
            ssh_key:    loaded_data.ssh_key.clone()
        };

        Ok(vec![WorkflowEvent::SyncRequested(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::SyncRequested(state) => {
                let workflows_dir = &app_context.config.workflows_dir;
                let clone_options =
                    CloneOptions { ssh_key: state.ssh_key.clone(), branch: Some(state.branch.clone()) };

                let commit_id =
                    app_context.git_client.clone_repository(&state.remote_url, workflows_dir, &clone_options).await?;

                println!("✅ {}", t_params!("cli_sync_completed", &[&state.remote_url]));

                let record_sync_result_command = RecordSyncResultCommand { commit_id: commit_id.clone() };
                context.schedule_command(record_sync_result_command.into()).await?;
            }
            _ => {
                println!("{}", t!("error_sync_not_requested"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "sync-workflows"
    }
}

#[derive(Debug, Clone)]
pub struct RecordSyncResultData {
    pub remote_url:   String,
    pub branch:       String,
    pub commit_id:    String,
    pub synced_count: u32
}

#[async_trait]
impl Command for RecordSyncResultCommand {
    type Error = WorkflowError;
    type LoadedData = RecordSyncResultData;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        // Validate that we can only record sync results after a sync was requested
        let sync_state = match current_state {
            WorkflowState::SyncRequested(sync_state) => sync_state,
            _ => {
                return Err(WorkflowError::Validation("Cannot record sync results: no sync was requested".to_string()));
            }
        };

        let workflows_dir = &app_context.config.workflows_dir;

        // Use the commit ID passed from the SyncWorkflowsCommand
        let commit_id = self.commit_id.clone();

        let synced_count = if workflows_dir.exists() {
            fs::read_dir(workflows_dir)
                .map_err(|e| WorkflowError::FileSystem(e.to_string()))?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if path.is_file()
                        && let Some(ext) = path.extension()
                        && (ext == "yaml" || ext == "yml")
                    {
                        return Some(());
                    }
                    None
                })
                .count() as u32
        } else {
            0
        };

        Ok(RecordSyncResultData {
            remote_url: sync_state.remote_url.clone(),
            branch: sync_state.branch.clone(),
            commit_id,
            synced_count
        })
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let event = WorkflowsSyncedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    chrono::Utc::now(),
            remote_url:   loaded_data.remote_url.clone(),
            branch:       loaded_data.branch.clone(),
            commit_id:    loaded_data.commit_id.clone(),
            synced_count: loaded_data.synced_count
        };

        Ok(vec![WorkflowEvent::WorkflowsSynced(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::WorkflowsSynced(state) => {
                println!("🔄 {}", t_params!("cli_synced_workflows", &[&state.remote_url]));
                println!("🌿 {}", t_params!("cli_synced_branch", &[&state.branch]));
                println!("📝 {}", t_params!("cli_synced_commit", &[&state.commit_id]));
                println!("📁 {}", t_params!("cli_synced_count", &[&state.synced_count.to_string()]));
            }
            _ => {
                println!("{}", t!("error_no_workflows_synced"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "record-sync-result"
    }
}

#[async_trait]
impl Command for SetLanguageCommand {
    type Error = WorkflowError;
    type LoadedData = ();

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Language::try_from(self.language.as_str())?;
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let event = LanguageSetEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            language:  self.language.clone()
        };

        Ok(vec![WorkflowEvent::LanguageSet(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::LanguageSet(state) => {
                let language = Language::try_from(state.language.as_str())?;
                app_context.config.set_current_language(language)?;
                println!("{}", t_params!("lang_set_success", &[&state.language]));
            }
            _ => {
                return Err(WorkflowError::Validation("Invalid state for language set".to_string()));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "set-language"
    }

    fn description(&self) -> &'static str {
        "Sets the current language for the application"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[async_trait]
impl Command for GetCurrentLanguageCommand {
    type Error = WorkflowError;
    type LoadedData = String;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let current_language = app_context.config.get_current_language()?;
        Ok(current_language.code().to_string())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        Ok(vec![])
    }

    async fn effect(
        &self,
        loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        println!("{}", t_params!("lang_current", &[&loaded_data]));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "get-current-language"
    }

    fn description(&self) -> &'static str {
        "Gets the current language setting"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}

#[async_trait]
impl Command for ListLanguagesCommand {
    type Error = WorkflowError;
    type LoadedData = Vec<String>;

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let languages = vec![Language::English.code().to_string(), Language::Spanish.code().to_string()];
        Ok(languages)
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        Ok(vec![])
    }

    async fn effect(
        &self,
        loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        println!("{}", t!("lang_available_languages"));
        println!();
        for language in loaded_data {
            println!("  - {}", language);
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "list-languages"
    }

    fn description(&self) -> &'static str {
        "Lists all available languages"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}

#[async_trait]
impl Command for SetStorageCommand {
    type Error = WorkflowError;
    type LoadedData = ();

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        app_context.config.set_current_storage(self.backend)?;
        Ok(vec![])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        println!("{}", t_params!("storage_set_success", &[self.backend.as_str()]));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "set-storage"
    }

    fn description(&self) -> &'static str {
        "Sets the storage backend"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[async_trait]
impl Command for GetCurrentStorageCommand {
    type Error = WorkflowError;
    type LoadedData = EventStoreType;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        app_context.config.get_current_storage()
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        Ok(vec![])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        let current = _app_context.config.get_current_storage()?;
        println!("{}", t_params!("storage_current", &[current.as_str()]));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "get-current-storage"
    }

    fn description(&self) -> &'static str {
        "Gets the current storage backend"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}

#[async_trait]
impl Command for ListAggregatesCommand {
    type Error = WorkflowError;
    type LoadedData = Vec<String>;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        app_context.event_store.list_aggregates().await
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let aggregate_ids = loaded_data.clone();
        let event = AggregatesListedEvent {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            aggregate_count: aggregate_ids.len(),
            aggregate_ids
        };
        Ok(vec![WorkflowEvent::AggregatesListed(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error> {
        let aggregate_ids = app_context.event_store.list_aggregates().await?;

        if aggregate_ids.is_empty() {
            println!("{}", t!("storage_no_aggregates"));
        } else {
            println!("{}", t_params!("storage_aggregates_count", &[&aggregate_ids.len().to_string()]));
            for id in aggregate_ids {
                println!("  {}", id);
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "list-aggregates"
    }

    fn description(&self) -> &'static str {
        "Lists all aggregate IDs"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}

#[async_trait]
impl Command for PurgeStorageCommand {
    type Error = WorkflowError;
    type LoadedData = ();

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(())
    }

    fn validate(&self, _loaded: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        Ok(vec![])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error> {
        use crate::adapter::storage::EventStoreType;

        if app_context.config.storage_type != EventStoreType::RocksDb {
            println!("{}", t!("storage_purge_only_rocksdb"));
            return Ok(());
        }

        let db_path = &app_context.config.database_path;

        drop(app_context.event_store.clone());

        tokio::task::spawn_blocking({
            let db_path = db_path.clone();
            move || -> Result<(), WorkflowError> {
                if db_path.exists() {
                    std::fs::remove_dir_all(&db_path)
                        .map_err(|e| WorkflowError::FileSystem(format!("Failed to remove database: {}", e)))?;
                }
                Ok(())
            }
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to purge storage: {}", e)))??;

        println!("{}", t!("storage_purge_success"));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "purge_storage"
    }

    fn description(&self) -> &'static str {
        "Purge all data from storage"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[async_trait]
impl Command for ReplayAggregateCommand {
    type Error = WorkflowError;
    type LoadedData = (WorkflowState, usize);

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let state = app_context.event_store.get_current_state(&self.aggregate_id).await?;
        let events = app_context.event_store.get_events(&self.aggregate_id).await?;
        Ok((state, events.len()))
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let (_state, events_count) = loaded_data;
        let event = AggregateReplayedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    Utc::now(),
            aggregate_id: self.aggregate_id.clone(),
            events_count: *events_count
        };
        Ok(vec![WorkflowEvent::AggregateReplayed(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error> {
        let state = app_context.event_store.get_current_state(&self.aggregate_id).await?;
        let events = app_context.event_store.get_events(&self.aggregate_id).await?;

        println!("{}", t_params!("storage_replay_aggregate", &[&self.aggregate_id]));
        println!("\n{}", t_params!("storage_replay_events_count", &[&events.len().to_string()]));
        println!("\n{}", t!("storage_replay_state"));
        println!("{:#?}", state);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "replay-aggregate"
    }

    fn description(&self) -> &'static str {
        "Replays events for a specific aggregate ID"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}
