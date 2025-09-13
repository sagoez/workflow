//! Typed messages for actor communication

use ractor::{Message, RpcReplyPort};

use crate::domain::{
    command::WorkflowCommand, error::WorkflowError, event::WorkflowEvent, state::WorkflowState,
    workflow::WorkflowContext
};

/// Messages for the Guardian actor (root of actor system)
#[derive(Debug)]
pub enum GuardianMessage {
    /// Initialize the actor system
    Initialize,
    /// Submit a command for processing
    SubmitCommand {
        command: WorkflowCommand,
        context: Box<WorkflowContext>,
        reply:   RpcReplyPort<Result<(), WorkflowError>>
    },
    /// Shutdown the entire system
    Shutdown,
    /// System health check
    HealthCheck { reply: RpcReplyPort<SystemHealth> }
}

/// Messages for the WorkflowManager actor
#[derive(Debug)]
pub enum WorkflowManagerMessage {
    /// Submit a new workflow command for processing
    SubmitCommand {
        command: WorkflowCommand,
        context: Box<WorkflowContext>,
        reply:   RpcReplyPort<Result<(), WorkflowError>>
    },
    /// Session completed successfully
    SessionCompleted { session_id: String },
    /// Session failed
    SessionFailed { session_id: String, error: String },
    /// Get active sessions count
    GetActiveSessions { reply: RpcReplyPort<usize> }
}

/// Messages for CommandProcessor actors (per-session)
#[derive(Debug, Clone)]
pub enum CommandProcessorMessage {
    /// Process a workflow command
    ProcessCommand { command: WorkflowCommand },
    /// Schedule a follow-up command (from within command effects)
    ScheduleCommand { command: WorkflowCommand },
    /// Complete the session
    Complete
}

/// Messages for EventStore actor
#[derive(Debug)]
pub enum EventStoreMessage {
    /// Get current state for a session
    GetState { session_id: String, reply: RpcReplyPort<Result<WorkflowState, WorkflowError>> },
    /// Store events for a session
    StoreEvents {
        session_id: String,
        events:     Vec<WorkflowEvent>,
        reply:      RpcReplyPort<Result<(), WorkflowError>>
    },
    /// Get all events for a session
    GetEvents { session_id: String, reply: RpcReplyPort<Result<Vec<WorkflowEvent>, WorkflowError>> }
}

/// System health information
#[derive(Debug)]
pub struct SystemHealth {
    pub active_sessions:          usize,
    pub total_commands_processed: u64,
    pub uptime_seconds:           u64
}

// Implement Message trait for Ractor
impl Message for GuardianMessage {}
impl Message for WorkflowManagerMessage {}
impl Message for CommandProcessorMessage {}
impl Message for EventStoreMessage {}
