//! Network ports - interfaces for HTTP and network operations

use std::collections::HashMap;

use async_trait::async_trait;

use crate::shared::WorkflowError;

/// Port for HTTP client operations
#[async_trait]
pub trait HttpClient: Send + Sync {
    /// Send a GET request
    async fn get(&self, url: &str, headers: Option<HashMap<String, String>>) -> Result<HttpResponse, WorkflowError>;

    /// Send a POST request with JSON body
    async fn post_json(
        &self,
        url: &str,
        body: &str,
        headers: Option<HashMap<String, String>>
    ) -> Result<HttpResponse, WorkflowError>;

    /// Send a PUT request with JSON body
    async fn put_json(
        &self,
        url: &str,
        body: &str,
        headers: Option<HashMap<String, String>>
    ) -> Result<HttpResponse, WorkflowError>;

    /// Send a DELETE request
    async fn delete(&self, url: &str, headers: Option<HashMap<String, String>>) -> Result<HttpResponse, WorkflowError>;

    /// Download a file from URL
    async fn download_file(&self, url: &str, destination: &std::path::Path) -> Result<(), WorkflowError>;

    /// Check if URL is reachable
    async fn is_reachable(&self, url: &str) -> Result<bool, WorkflowError>;
}

/// HTTP response information
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers:     HashMap<String, String>,
    pub body:        String,
    pub is_success:  bool
}

impl HttpResponse {
    pub fn new(status_code: u16, body: String) -> Self {
        Self { status_code, headers: HashMap::new(), body, is_success: status_code >= 200 && status_code < 300 }
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }
}
