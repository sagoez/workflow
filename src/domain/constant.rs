//! Domain Events - Structured events for internal monitoring and debugging

/// Guardian Actor Events
pub mod guardian {
    pub const GUARDIAN_STARTED: &str = "guardian.started";
    pub const CHILDREN_SPAWNING: &str = "children.spawning";
    pub const CHILDREN_SPAWNED: &str = "children.spawned";
    pub const CHILDREN_SPAWN_FAILED: &str = "children.spawn_failed";
    pub const SYSTEM_INITIALIZED: &str = "system.initialized";
    pub const SYSTEM_SHUTDOWN_STARTED: &str = "system.shutdown_started";
    pub const SYSTEM_SHUTDOWN_COMPLETED: &str = "system.shutdown_completed";
    pub const HEALTH_CHECK_COMPLETED: &str = "health.check_completed";
    pub const COMMAND_SUBMITTED: &str = "command.submitted";
}

/// WorkflowManager Actor Events
pub mod workflow_manager {
    pub const MANAGER_STARTED: &str = "manager.started";
    pub const COMMAND_SUBMITTED: &str = "command.submitted";
    pub const SESSION_CREATED: &str = "session.created";
    pub const SESSION_COMPLETED: &str = "session.completed";
    pub const SESSION_FAILED: &str = "session.failed";
    pub const PROCESSOR_SPAWNED: &str = "processor.spawned";
    pub const PROCESSOR_SPAWN_FAILED: &str = "processor.spawn_failed";
}

/// CommandProcessor Actor Events
pub mod command_processor {
    pub const PROCESSOR_STARTED: &str = "processor.started";
    pub const COMMAND_RECEIVED: &str = "command.received";
    pub const COMMAND_PROCESSED: &str = "command.processed";
    pub const COMMAND_FAILED: &str = "command.failed";
    pub const COMMAND_SCHEDULED: &str = "command.scheduled";
    pub const SESSION_COMPLETED: &str = "session.completed";
    pub const RECOVERY_STARTED: &str = "recovery.started";
    pub const RECOVERY_FAILED: &str = "recovery.failed";
    pub const EVENTS_PERSISTING: &str = "events.persisting";
    pub const EVENTS_PERSISTED: &str = "events.persisted";
    pub const EVENTS_PERSIST_FAILED: &str = "events.persist_failed";
}

/// EventStore Actor Events
pub mod event_store {
    pub const STORE_STARTED: &str = "store.started";
    pub const EVENTS_STORED: &str = "events.stored";
    pub const EVENTS_RETRIEVED: &str = "events.retrieved";
    pub const STATE_RETRIEVED: &str = "state.retrieved";
    pub const STORAGE_FAILED: &str = "storage.failed";
}
