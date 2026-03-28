use async_trait::async_trait;
use chrono::Utc;
use tabled::{
    builder::Builder,
    settings::{Color, Modify, Style, object::Rows}
};
use uuid::Uuid;

use crate::{
    AppContext,
    domain::{
        command::{DeleteAggregateCommand, ListAggregatesCommand, ReplayAggregateCommand},
        engine::EngineContext,
        error::{ValidationError, WorkflowError},
        event::{AggregateReplayedEvent, WorkflowEvent},
        state::{StateDisplay, WorkflowState},
        workflow::Workflow
    },
    port::{command::Command, storage::EventStore},
    t, t_params
};

const BASE62_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Encode a UUID as a compact base62 string (~22 chars).
pub fn uuid_to_short(uuid_str: &str) -> String {
    let uuid = Uuid::parse_str(uuid_str).unwrap_or_default();
    let mut num = u128::from_be_bytes(*uuid.as_bytes());
    if num == 0 {
        return "0".to_string();
    }
    let mut result = Vec::new();
    while num > 0 {
        result.push(BASE62_CHARS[(num % 62) as usize]);
        num /= 62;
    }
    result.reverse();
    String::from_utf8(result).unwrap()
}

/// Decode a base62 string back to a UUID string.
pub fn short_to_uuid(short: &str) -> Result<String, WorkflowError> {
    let mut num: u128 = 0;
    for &b in short.as_bytes() {
        let val = match b {
            b'0'..=b'9' => b - b'0',
            b'A'..=b'Z' => b - b'A' + 10,
            b'a'..=b'z' => b - b'a' + 36,
            _ => return Err(ValidationError::Other(t_params!("storage_aggregate_not_found", &[short])).into())
        };
        num = num
            .checked_mul(62)
            .and_then(|n| n.checked_add(val as u128))
            .ok_or_else(|| WorkflowError::from(ValidationError::Other("ID too long".to_string())))?;
    }
    let bytes = num.to_be_bytes();
    Ok(Uuid::from_bytes(bytes).to_string())
}

/// Load aggregate data from event store: current state and event count.
/// Returns an error if no events exist for the given aggregate ID.
pub async fn load_aggregate_data(
    event_store: &dyn EventStore,
    aggregate_id: &str
) -> Result<(WorkflowState, usize), WorkflowError> {
    let events = event_store.get_events(aggregate_id).await?;
    if events.is_empty() {
        return Err(ValidationError::Other(t_params!("storage_aggregate_not_found", &[aggregate_id])).into());
    }
    let state = event_store.get_current_state(aggregate_id).await?;
    Ok((state, events.len()))
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
            let mut builder = Builder::default();
            builder.push_record(["#", "Workflow", "Status", "ID"]);

            for (i, id) in aggregate_ids.iter().enumerate() {
                let state = app_context.event_store.get_current_state(id).await?;
                let workflow_name = extract_workflow_name(&state);
                let status = state.phase_name();
                let short_id = uuid_to_short(id);

                builder.push_record([&(i + 1).to_string(), &workflow_name, &status, &short_id]);
            }

            let mut table = builder.build();
            table.with(
                Style::modern()
                    .corner_bottom_left('╰')
                    .corner_bottom_right('╯')
                    .corner_top_left('╭')
                    .corner_top_right('╮')
            );
            table.with(Modify::new(Rows::first()).with(Color::FG_BRIGHT_CYAN));

            println!("{}", table);
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
        // Accept both full UUIDs and short base62 IDs
        let full_id = if self.aggregate_id.contains('-') {
            self.aggregate_id.clone()
        } else {
            short_to_uuid(&self.aggregate_id)?
        };
        load_aggregate_data(&*app_context.event_store, &full_id).await
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
        let full_id = if self.aggregate_id.contains('-') {
            self.aggregate_id.clone()
        } else {
            short_to_uuid(&self.aggregate_id)?
        };
        let state = app_context.event_store.get_current_state(&full_id).await?;
        let events = app_context.event_store.get_events(&full_id).await?;

        println!("{}", t_params!("storage_replay_aggregate", &[&uuid_to_short(&full_id)]));
        println!("\n{}", t_params!("storage_replay_events_count", &[&events.len().to_string()]));
        println!("\n{}", t!("storage_replay_state"));
        display_state(&state);

        let command_parts: Option<(&Workflow, &std::collections::HashMap<String, String>)> = match &state {
            WorkflowState::WorkflowArgumentsResolved(s) => Some((&s.selected_workflow, &s.resolved_arguments)),
            WorkflowState::WorkflowCompleted(s) => Some((&s.completed_workflow, &s.resolved_arguments)),
            WorkflowState::Initial(_)
            | WorkflowState::WorkflowsDiscovered(_)
            | WorkflowState::WorkflowsListed(_)
            | WorkflowState::WorkflowSelected(_)
            | WorkflowState::WorkflowStarted(_)
            | WorkflowState::SyncRequested(_)
            | WorkflowState::WorkflowsSynced(_)
            | WorkflowState::LanguageSet(_) => None
        };

        if let Some((workflow, args)) = command_parts {
            if let Ok(rendered) = super::resolve::render_command_template(&workflow.command, args) {
                println!("\n{}\n  {}", t!("storage_replay_command"), rendered);
            }
        }

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

#[async_trait]
impl Command for DeleteAggregateCommand {
    type Error = WorkflowError;
    type LoadedData = String;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let full_id = if self.aggregate_id.contains('-') {
            self.aggregate_id.clone()
        } else {
            short_to_uuid(&self.aggregate_id)?
        };

        let events = app_context.event_store.get_events(&full_id).await?;
        if events.is_empty() {
            return Err(ValidationError::Other(t_params!("storage_aggregate_not_found", &[&self.aggregate_id])).into());
        }

        Ok(full_id)
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
        app_context: &AppContext
    ) -> Result<(), Self::Error> {
        app_context.event_store.delete_aggregate(loaded_data).await?;
        println!("{}", t_params!("storage_aggregate_deleted", &[&uuid_to_short(loaded_data)]));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "delete-aggregate"
    }

    fn description(&self) -> &'static str {
        "Deletes all events for a specific aggregate"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

fn extract_workflow_name(state: &WorkflowState) -> String {
    match state {
        WorkflowState::Initial(_) => "-".to_string(),
        WorkflowState::WorkflowsDiscovered(_) => "-".to_string(),
        WorkflowState::WorkflowsListed(_) => "-".to_string(),
        WorkflowState::WorkflowSelected(s) => s.selected_workflow.name.clone(),
        WorkflowState::WorkflowStarted(s) => s.selected_workflow.name.clone(),
        WorkflowState::WorkflowArgumentsResolved(s) => s.selected_workflow.name.clone(),
        WorkflowState::WorkflowCompleted(s) => s.completed_workflow.name.clone(),
        WorkflowState::SyncRequested(_) => "(sync)".to_string(),
        WorkflowState::WorkflowsSynced(_) => "(sync)".to_string(),
        WorkflowState::LanguageSet(_) => "(language)".to_string()
    }
}

fn display_state(state: &WorkflowState) {
    let mut builder = Builder::default();

    builder.push_record([t!("state_table_property"), t!("state_table_value")]);

    builder.push_record([t!("state_field_phase"), state.phase_name()]);

    let table_rows = state.table_rows();
    let num_standard_fields = 2;

    for (key, value) in &table_rows {
        builder.push_record([key, value]);
    }

    let mut table = builder.build();
    table.with(
        Style::modern().corner_bottom_left('╰').corner_bottom_right('╯').corner_top_left('╭').corner_top_right('╮')
    );

    if table_rows.len() > num_standard_fields {
        let arg_start_row = 2 + num_standard_fields;
        table.with(Modify::new(Rows::new(arg_start_row..)).with(Color::FG_BRIGHT_CYAN));
    }

    println!("{}", table);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapter::storage::InMemoryEventStore,
        domain::{
            event::{WorkflowDiscoveredEvent, WorkflowEvent},
            workflow::Workflow
        }
    };

    fn test_workflow() -> Workflow {
        Workflow {
            name:        "test-wf".to_string(),
            description: "test desc".to_string(),
            command:     "echo test".to_string(),
            arguments:   vec![],
            source_url:  None,
            author:      None,
            author_url:  None,
            shells:      vec![],
            tags:        vec![]
        }
    }

    #[tokio::test]
    async fn load_aggregate_returns_state_and_count() {
        let store = InMemoryEventStore::new();
        let event = WorkflowEvent::WorkflowDiscovered(WorkflowDiscoveredEvent {
            event_id:  "e1".to_string(),
            timestamp: chrono::Utc::now(),
            workflow:  test_workflow(),
            file_path: "test.yaml".to_string()
        });
        store.store_events("session-1", &[event]).await.unwrap();

        let (state, count) = load_aggregate_data(&store, "session-1").await.unwrap();
        assert_eq!(count, 1);
        assert!(matches!(state, WorkflowState::WorkflowsDiscovered(_)));
    }

    #[tokio::test]
    async fn load_aggregate_nonexistent_returns_error() {
        let store = InMemoryEventStore::new();
        let result = load_aggregate_data(&store, "nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_aggregates_returns_stored_ids() {
        let store = InMemoryEventStore::new();
        let event = WorkflowEvent::WorkflowDiscovered(WorkflowDiscoveredEvent {
            event_id:  "e1".to_string(),
            timestamp: chrono::Utc::now(),
            workflow:  test_workflow(),
            file_path: "test.yaml".to_string()
        });
        store.store_events("session-a", &[event.clone()]).await.unwrap();
        store.store_events("session-b", &[event]).await.unwrap();

        let ids = store.list_aggregates().await.unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"session-a".to_string()));
        assert!(ids.contains(&"session-b".to_string()));
    }

    #[test]
    fn uuid_to_short_produces_compact_string() {
        let uuid = "2dc05d2b-70d4-44c5-9950-4bad6c5b35ac";
        let short = uuid_to_short(uuid);
        assert!(!short.is_empty());
        assert!(short.len() <= 22);
        assert!(short.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn short_to_uuid_roundtrips() {
        let original = "2dc05d2b-70d4-44c5-9950-4bad6c5b35ac";
        let short = uuid_to_short(original);
        let recovered = short_to_uuid(&short).unwrap();
        assert_eq!(recovered, original);
    }

    #[test]
    fn short_to_uuid_roundtrips_multiple() {
        let uuids = [
            "00000000-0000-0000-0000-000000000001",
            "ffffffff-ffff-ffff-ffff-ffffffffffff",
            "550e8400-e29b-41d4-a716-446655440000",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
        ];
        for uuid in uuids {
            let short = uuid_to_short(uuid);
            let recovered = short_to_uuid(&short).unwrap();
            assert_eq!(recovered, uuid, "roundtrip failed for {}", uuid);
        }
    }

    #[test]
    fn short_to_uuid_invalid_char_returns_error() {
        let result = short_to_uuid("abc!def");
        assert!(result.is_err());
    }

    #[test]
    fn uuid_to_short_is_deterministic() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(uuid_to_short(uuid), uuid_to_short(uuid));
    }

    #[test]
    fn extract_name_from_completed_state() {
        let state = WorkflowState::WorkflowCompleted(crate::domain::state::WorkflowCompletedState {
            discovered_workflows: vec![],
            completed_workflow:   test_workflow(),
            execution_id:         "exec-1".to_string(),
            resolved_arguments:   std::collections::HashMap::new()
        });
        assert_eq!(extract_workflow_name(&state), "test-wf");
    }

    #[test]
    fn extract_name_from_selected_state() {
        let state = WorkflowState::WorkflowSelected(crate::domain::state::WorkflowSelectedState {
            discovered_workflows: vec![],
            selected_workflow:    test_workflow()
        });
        assert_eq!(extract_workflow_name(&state), "test-wf");
    }

    #[test]
    fn extract_name_from_initial_state() {
        let state = WorkflowState::Initial(crate::domain::state::InitialState);
        assert_eq!(extract_workflow_name(&state), "-");
    }

    #[test]
    fn extract_name_from_sync_state() {
        let state = WorkflowState::WorkflowsSynced(crate::domain::state::WorkflowsSyncedState {
            remote_url:   "https://example.com".to_string(),
            branch:       "main".to_string(),
            commit_id:    "abc123".to_string(),
            synced_count: 5,
            synced_at:    chrono::Utc::now()
        });
        assert_eq!(extract_workflow_name(&state), "(sync)");
    }

    #[tokio::test]
    async fn delete_aggregate_removes_events() {
        let store = InMemoryEventStore::new();
        let event = WorkflowEvent::WorkflowDiscovered(WorkflowDiscoveredEvent {
            event_id:  "e1".to_string(),
            timestamp: chrono::Utc::now(),
            workflow:  test_workflow(),
            file_path: "test.yaml".to_string()
        });
        store.store_events("session-1", &[event]).await.unwrap();

        let ids = store.list_aggregates().await.unwrap();
        assert_eq!(ids.len(), 1);

        store.delete_aggregate("session-1").await.unwrap();

        let ids = store.list_aggregates().await.unwrap();
        assert!(ids.is_empty());

        let events = store.get_events("session-1").await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn delete_nonexistent_aggregate_succeeds() {
        let store = InMemoryEventStore::new();
        let result = store.delete_aggregate("nonexistent").await;
        assert!(result.is_ok());
    }
}
