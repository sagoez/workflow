use async_trait::async_trait;
use chrono::Utc;
use clipboard::ClipboardProvider;
use tabled::{
    builder::Builder,
    settings::{Color, Modify, Style, object::Rows}
};
use uuid::Uuid;

use crate::{
    AppContext,
    adapter::{resolver::ArgumentResolver, storage::EventStoreType},
    domain::{
        command::{
            CompleteWorkflowCommand, GetCurrentLanguageCommand,
            GetCurrentStorageCommand,
            ListAggregatesCommand, ListLanguagesCommand,
            RecordSyncResultCommand, ReplayAggregateCommand, ResolveArgumentsCommand, ResolveArgumentsData,
            SetLanguageCommand, SetStorageCommand, StartWorkflowCommand, SyncWorkflowsCommand, WorkflowCommand
        },
        engine::EngineContext,
        error::WorkflowError,
        event::{
            AggregateReplayedEvent, LanguageSetEvent, SyncRequestedEvent,
            WorkflowArgumentsResolvedEvent, WorkflowCompletedEvent, WorkflowEvent,
            WorkflowStartedEvent
        },
        state::{StateDisplay, WorkflowState}
    },
    i18n::Language,
    port::{command::Command, git::CloneOptions},
    t, t_params
};

pub mod discover;
pub mod list;
pub mod purge;
pub mod select;
pub mod sync_record;

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
        app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let workflow = match current_state {
            WorkflowState::WorkflowStarted(state) => state.selected_workflow.clone(),
            _ => return Err(WorkflowError::Validation(t!("error_no_workflow_started_to_resolve_arguments")))
        };

        let resolved_arguments =
            ArgumentResolver::resolve_workflow_arguments(
                &workflow.arguments,
                &*app_context.prompt,
                &*app_context.executor
            ).await.map_err(|e| {
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
        Self::display_state(&state);
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

impl ReplayAggregateCommand {
    fn display_state(state: &WorkflowState) {
        let mut builder = Builder::default();

        builder.push_record([t!("state_table_property"), t!("state_table_value")]);

        builder.push_record([t!("state_field_phase"), state.phase_name()]);

        let table_rows = state.table_rows();
        let num_standard_fields = 2; // Phase + other standard fields (e.g., Workflow, Execution ID, etc.)

        for (key, value) in &table_rows {
            builder.push_record([key, value]);
        }

        let mut table = builder.build();
        table.with(
            Style::modern().corner_bottom_left('╰').corner_bottom_right('╯').corner_top_left('╭').corner_top_right('╮')
        );

        // Make argument rows bold (rows after standard fields)
        // Row 0 is header, Row 1 is Phase, Rows 2+ are from table_rows
        if table_rows.len() > num_standard_fields {
            let arg_start_row = 2 + num_standard_fields;
            table.with(Modify::new(Rows::new(arg_start_row..)).with(Color::FG_BRIGHT_CYAN));
        }

        println!("{}", table);
    }
}
