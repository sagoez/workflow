//! Configuration service - manages system configuration
//!
//! This service handles all configuration-related operations including:
//! - Initializing configuration directories
//! - Managing language settings
//! - Managing resource URLs
//! - Loading and saving configuration

use std::sync::Arc;

use crate::{
    i18n,
    ports::{
        filesystem::FileSystem,
        storage::{Config, ConfigStore, EventData, EventStore},
        ui::{ProgressIndicator, Renderer}
    },
    shared::{CommandContext, WorkflowError, events::*}
};

/// Service for configuration operations
pub struct ConfigService {
    config_store: Arc<dyn ConfigStore>,
    event_store:  Arc<dyn EventStore>,
    filesystem:   Arc<dyn FileSystem>,
    renderer:     Arc<dyn Renderer>,
    progress:     Arc<dyn ProgressIndicator>
}

impl ConfigService {
    pub fn new(
        config_store: Arc<dyn ConfigStore>,
        event_store: Arc<dyn EventStore>,
        filesystem: Arc<dyn FileSystem>,
        renderer: Arc<dyn Renderer>,
        progress: Arc<dyn ProgressIndicator>
    ) -> Self {
        Self { config_store, event_store, filesystem, renderer, progress }
    }

    /// Initialize configuration directories and default files
    pub async fn initialize_config(&self, _context: &CommandContext) -> Result<(), WorkflowError> {
        // Initialize configuration using the config store
        self.config_store.init_config().await?;

        // Create and emit initialization event
        let event = ConfigurationInitializedEvent::new(
            "~/.config/workflow-rs".to_string(), // TODO: Get actual paths
            "~/.config/workflow-rs/workflows".to_string(),
            "~/.config/workflow-rs/i18n".to_string()
        );

        // Store the event
        let event_data = self.event_to_data(&event)?;
        self.event_store.save_event(&event_data).await?;

        // Display success message
        self.renderer.display_success(&i18n::t("service_config_initialized")).await?;
        self.renderer.display_message(&i18n::t_params("service_config_directory", &[&event.config_dir])).await?;
        self.renderer.display_message(&i18n::t_params("service_workflows_directory", &[&event.workflows_dir])).await?;
        self.renderer.display_message(&i18n::t_params("service_i18n_directory", &[&event.i18n_dir])).await?;

        Ok(())
    }

    /// Set the system language
    pub async fn set_language(&self, language: &str, context: &CommandContext) -> Result<(), WorkflowError> {
        // Load current configuration
        let mut config = self.config_store.load_config().await?;
        let old_language = config.language.clone();

        // Validate language is supported
        let supported_languages = self.get_supported_languages().await?;
        if !supported_languages.contains(&language.to_string()) {
            let available = supported_languages.join(", ");
            return Err(WorkflowError::Validation(i18n::t_params(
                "service_unsupported_language",
                &[language, &available]
            )));
        }

        // Update configuration
        config.language = language.to_string();
        self.config_store.save_config(&config).await?;

        // Create and emit language changed event
        let event = LanguageChangedEvent::new(old_language, language.to_string(), context.user.clone());

        // Store the event
        let event_data = self.event_to_data(&event)?;
        self.event_store.save_event(&event_data).await?;

        // Display success message
        self.renderer.display_success(&i18n::t_params("service_language_set", &[language])).await?;

        Ok(())
    }

    /// Get the current language
    pub async fn get_current_language(&self) -> Result<String, WorkflowError> {
        let config = self.config_store.load_config().await?;
        Ok(config.language)
    }

    /// List available languages
    pub async fn list_languages(&self) -> Result<Vec<String>, WorkflowError> {
        self.get_supported_languages().await
    }

    /// Set the resource URL for workflows
    pub async fn set_resource_url(&self, url: &str, context: &CommandContext) -> Result<(), WorkflowError> {
        // Load current configuration
        let mut config = self.config_store.load_config().await?;
        let old_url = config.resource_url.clone();

        // Update configuration
        config.resource_url = Some(url.to_string());
        self.config_store.save_config(&config).await?;

        // Create and emit resource URL changed event
        let event = ResourceUrlChangedEvent::new(old_url, url.to_string(), context.user.clone());

        // Store the event
        let event_data = self.event_to_data(&event)?;
        self.event_store.save_event(&event_data).await?;

        // Display success message
        self.renderer.display_success(&i18n::t_params("service_resource_url_set", &[url])).await?;
        self.renderer.display_message(&i18n::t("service_sync_tip")).await?;

        Ok(())
    }

    /// Get the current resource URL
    pub async fn get_resource_url(&self) -> Result<Option<String>, WorkflowError> {
        let config = self.config_store.load_config().await?;
        Ok(config.resource_url)
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> Result<Config, WorkflowError> {
        self.config_store.load_config().await
    }

    // Private helper methods

    async fn get_supported_languages(&self) -> Result<Vec<String>, WorkflowError> {
        // TODO: Scan i18n directory for available language files
        // For now, return hardcoded supported languages
        Ok(vec!["en".to_string(), "es".to_string()])
    }

    fn event_to_data<E: serde::Serialize>(&self, event: &E) -> Result<EventData, WorkflowError> {
        let data = serde_json::to_value(event).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

        Ok(EventData {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: "ConfigEvent".to_string(), // TODO: Extract actual event type
            aggregate_id: Some("config".to_string()),
            timestamp: chrono::Utc::now(),
            data,
            metadata: None
        })
    }
}
