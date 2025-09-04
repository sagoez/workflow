//! Services layer - orchestrates business logic using ports
//!
//! Services coordinate between commands, ports, and adapters to implement
//! the business logic of the workflow system. They provide a clean API
//! for the CLI layer while keeping the domain logic separate from infrastructure.

pub mod config_service;
pub mod sync_service;
pub mod workflow_service;

pub use config_service::*;
pub use sync_service::*;
pub use workflow_service::*;
