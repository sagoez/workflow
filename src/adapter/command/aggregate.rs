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
        command::{ListAggregatesCommand, ReplayAggregateCommand},
        engine::EngineContext,
        error::WorkflowError,
        event::{AggregateReplayedEvent, WorkflowEvent},
        state::{StateDisplay, WorkflowState}
    },
    port::{command::Command, storage::EventStore},
    t, t_params
};

/// Load aggregate data from event store: current state and event count.
pub async fn load_aggregate_data(
    event_store: &dyn EventStore,
    aggregate_id: &str
) -> Result<(WorkflowState, usize), WorkflowError> {
    let state = event_store.get_current_state(aggregate_id).await?;
    let events = event_store.get_events(aggregate_id).await?;
    Ok((state, events.len()))
}

// ==================== ListAggregatesCommand ====================

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

// ==================== ReplayAggregateCommand ====================

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
        load_aggregate_data(&*app_context.event_store, &self.aggregate_id).await
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
        display_state(&state);
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
    use crate::adapter::storage::InMemoryEventStore;
    use crate::domain::event::{WorkflowDiscoveredEvent, WorkflowEvent};
    use crate::domain::workflow::Workflow;

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
    async fn load_aggregate_empty_returns_default_state() {
        let store = InMemoryEventStore::new();
        let (state, count) = load_aggregate_data(&store, "nonexistent").await.unwrap();
        assert_eq!(count, 0);
        assert!(matches!(state, WorkflowState::Initial(_)));
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
}
