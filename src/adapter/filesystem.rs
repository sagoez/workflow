use std::path::{Path, PathBuf};

use crate::{domain::error::WorkflowError, port::filesystem::FileSystem};

/// Real implementation wrapping std::fs
pub struct StdFileSystem;

impl StdFileSystem {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystem for StdFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, WorkflowError> {
        std::fs::read_to_string(path).map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    fn write(&self, path: &Path, contents: &str) -> Result<(), WorkflowError> {
        std::fs::write(path, contents).map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), WorkflowError> {
        std::fs::create_dir_all(path).map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    fn remove_file(&self, path: &Path) -> Result<(), WorkflowError> {
        std::fs::remove_file(path).map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), WorkflowError> {
        std::fs::remove_dir_all(path).map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    fn read_dir_entries(&self, path: &Path) -> Result<Vec<PathBuf>, WorkflowError> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| WorkflowError::FileSystem(e.to_string()))? {
            let entry = entry.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
            entries.push(entry.path());
        }
        Ok(entries)
    }
}

#[cfg(test)]
pub mod mock {
    use std::{collections::HashMap, sync::Mutex};

    use super::*;

    /// In-memory filesystem mock
    pub struct MockFileSystem {
        files: Mutex<HashMap<PathBuf, String>>,
        dirs:  Mutex<Vec<PathBuf>>
    }

    impl MockFileSystem {
        pub fn new() -> Self {
            Self {
                files: Mutex::new(HashMap::new()),
                dirs:  Mutex::new(Vec::new())
            }
        }

        pub fn with_files(files: HashMap<PathBuf, String>) -> Self {
            Self {
                files: Mutex::new(files),
                dirs:  Mutex::new(Vec::new())
            }
        }
    }

    impl FileSystem for MockFileSystem {
        fn read_to_string(&self, path: &Path) -> Result<String, WorkflowError> {
            let files = self.files.lock().unwrap();
            files
                .get(path)
                .cloned()
                .ok_or_else(|| WorkflowError::FileSystem(format!("File not found: {}", path.display())))
        }

        fn write(&self, path: &Path, contents: &str) -> Result<(), WorkflowError> {
            let mut files = self.files.lock().unwrap();
            files.insert(path.to_path_buf(), contents.to_string());
            Ok(())
        }

        fn exists(&self, path: &Path) -> bool {
            let files = self.files.lock().unwrap();
            let dirs = self.dirs.lock().unwrap();
            files.contains_key(path) || dirs.contains(&path.to_path_buf())
        }

        fn create_dir_all(&self, path: &Path) -> Result<(), WorkflowError> {
            let mut dirs = self.dirs.lock().unwrap();
            dirs.push(path.to_path_buf());
            Ok(())
        }

        fn remove_file(&self, path: &Path) -> Result<(), WorkflowError> {
            let mut files = self.files.lock().unwrap();
            files
                .remove(path)
                .ok_or_else(|| WorkflowError::FileSystem(format!("File not found: {}", path.display())))?;
            Ok(())
        }

        fn remove_dir_all(&self, path: &Path) -> Result<(), WorkflowError> {
            let mut dirs = self.dirs.lock().unwrap();
            dirs.retain(|d| !d.starts_with(path));
            let mut files = self.files.lock().unwrap();
            files.retain(|p, _| !p.starts_with(path));
            Ok(())
        }

        fn read_dir_entries(&self, path: &Path) -> Result<Vec<PathBuf>, WorkflowError> {
            let files = self.files.lock().unwrap();
            let entries: Vec<PathBuf> = files
                .keys()
                .filter(|p| p.parent() == Some(path))
                .cloned()
                .collect();
            Ok(entries)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{mock::*, *};

    #[test]
    fn mock_fs_write_and_read() {
        let fs = MockFileSystem::new();
        fs.write(Path::new("/tmp/test.txt"), "hello").unwrap();
        let content = fs.read_to_string(Path::new("/tmp/test.txt")).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn mock_fs_read_nonexistent_errors() {
        let fs = MockFileSystem::new();
        let result = fs.read_to_string(Path::new("/nope"));
        assert!(result.is_err());
    }

    #[test]
    fn mock_fs_exists() {
        let fs = MockFileSystem::new();
        assert!(!fs.exists(Path::new("/tmp/test.txt")));
        fs.write(Path::new("/tmp/test.txt"), "data").unwrap();
        assert!(fs.exists(Path::new("/tmp/test.txt")));
    }

    #[test]
    fn mock_fs_create_dir_all() {
        let fs = MockFileSystem::new();
        fs.create_dir_all(Path::new("/tmp/a/b/c")).unwrap();
        assert!(fs.exists(Path::new("/tmp/a/b/c")));
    }

    #[test]
    fn mock_fs_remove_file() {
        let fs = MockFileSystem::new();
        fs.write(Path::new("/tmp/f.txt"), "x").unwrap();
        fs.remove_file(Path::new("/tmp/f.txt")).unwrap();
        assert!(!fs.exists(Path::new("/tmp/f.txt")));
    }

    #[test]
    fn mock_fs_remove_dir_all_clears_children() {
        let fs = MockFileSystem::new();
        fs.create_dir_all(Path::new("/tmp/dir")).unwrap();
        fs.write(Path::new("/tmp/dir/a.txt"), "a").unwrap();
        fs.write(Path::new("/tmp/dir/b.txt"), "b").unwrap();
        fs.remove_dir_all(Path::new("/tmp/dir")).unwrap();
        assert!(!fs.exists(Path::new("/tmp/dir/a.txt")));
        assert!(!fs.exists(Path::new("/tmp/dir")));
    }

    #[test]
    fn mock_fs_read_dir_entries() {
        let fs = MockFileSystem::new();
        fs.write(Path::new("/tmp/dir/a.txt"), "a").unwrap();
        fs.write(Path::new("/tmp/dir/b.txt"), "b").unwrap();
        fs.write(Path::new("/tmp/other/c.txt"), "c").unwrap();
        let entries = fs.read_dir_entries(Path::new("/tmp/dir")).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn mock_fs_with_files_constructor() {
        let mut files = std::collections::HashMap::new();
        files.insert(PathBuf::from("/x.txt"), "content".to_string());
        let fs = MockFileSystem::with_files(files);
        assert_eq!(fs.read_to_string(Path::new("/x.txt")).unwrap(), "content");
    }
}
