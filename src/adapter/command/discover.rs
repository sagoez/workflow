use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    AppContext,
    domain::{
        command::{DiscoverWorkflowsCommand, DiscoverWorkflowsData},
        engine::EngineContext,
        error::{StorageError, WorkflowError},
        event::{WorkflowDiscoveredEvent, WorkflowEvent},
        state::WorkflowState,
        workflow::Workflow
    },
    port::{command::Command, filesystem::FileSystem},
    t_params
};

/// Discover workflow YAML files from a directory using the FileSystem trait.
/// Returns a sorted list of parsed Workflow objects.
pub fn discover_workflows(
    fs: &dyn FileSystem,
    workflows_dir: &std::path::Path
) -> Result<Vec<Workflow>, WorkflowError> {
    if !fs.exists(workflows_dir) {
        return Ok(vec![]);
    }

    let entries = fs.read_dir_entries(workflows_dir)?;
    let mut workflows = Vec::new();

    for path in entries {
        if let Some(extension) = path.extension() {
            if extension == "yaml" || extension == "yml" {
                let content = fs.read_to_string(&path)?;
                let workflow: Workflow = serde_yaml::from_str(&content)
                    .map_err(|e| WorkflowError::from(StorageError::Serialization(e.to_string())))?;
                workflows.push(workflow);
            }
        }
    }

    workflows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(workflows)
}

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
        let fs = app_context.filesystem.clone();

        let workflows =
            tokio::task::spawn_blocking(move || discover_workflows(&*fs, &workflows_dir)).await.map_err(|e| {
                WorkflowError::Storage(StorageError::Io(t_params!(
                    "error_failed_to_discover_workflows",
                    &[&e.to_string()]
                )))
            })??;

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
                file_path: format!("{}.yaml", workflow.name)
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::adapter::filesystem::mock::MockFileSystem;

    fn yaml_content(name: &str) -> String {
        format!("name: {}\ndescription: test workflow\ncommand: echo hello\narguments: []\ntags: []\nshells: []", name)
    }

    #[test]
    fn discovers_yaml_files() {
        let fs = MockFileSystem::new();
        let dir = Path::new("/workflows");
        fs.create_dir_all(dir).unwrap();
        fs.write(&dir.join("alpha.yaml"), &yaml_content("alpha")).unwrap();
        fs.write(&dir.join("beta.yml"), &yaml_content("beta")).unwrap();

        let result = discover_workflows(&fs, dir).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "alpha");
        assert_eq!(result[1].name, "beta");
    }

    #[test]
    fn skips_non_yaml_files() {
        let fs = MockFileSystem::new();
        let dir = Path::new("/workflows");
        fs.create_dir_all(dir).unwrap();
        fs.write(&dir.join("readme.md"), "# readme").unwrap();
        fs.write(&dir.join("wf.yaml"), &yaml_content("wf")).unwrap();
        fs.write(&dir.join("data.json"), "{}").unwrap();

        let result = discover_workflows(&fs, dir).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "wf");
    }

    #[test]
    fn empty_directory_returns_empty_vec() {
        let fs = MockFileSystem::new();
        let dir = Path::new("/workflows");
        fs.create_dir_all(dir).unwrap();

        let result = discover_workflows(&fs, dir).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn nonexistent_directory_returns_empty_vec() {
        let fs = MockFileSystem::new();
        let result = discover_workflows(&fs, Path::new("/nope")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn invalid_yaml_returns_error() {
        let fs = MockFileSystem::new();
        let dir = Path::new("/workflows");
        fs.create_dir_all(dir).unwrap();
        fs.write(&dir.join("bad.yaml"), "not: [valid: workflow").unwrap();

        let result = discover_workflows(&fs, dir);
        assert!(result.is_err());
    }

    #[test]
    fn results_sorted_by_name() {
        let fs = MockFileSystem::new();
        let dir = Path::new("/workflows");
        fs.create_dir_all(dir).unwrap();
        fs.write(&dir.join("zebra.yaml"), &yaml_content("zebra")).unwrap();
        fs.write(&dir.join("alpha.yaml"), &yaml_content("alpha")).unwrap();
        fs.write(&dir.join("middle.yaml"), &yaml_content("middle")).unwrap();

        let result = discover_workflows(&fs, dir).unwrap();
        assert_eq!(result[0].name, "alpha");
        assert_eq!(result[1].name, "middle");
        assert_eq!(result[2].name, "zebra");
    }
}
