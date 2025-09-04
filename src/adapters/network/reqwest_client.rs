//! Reqwest implementation of network ports

use std::{collections::HashMap, path::Path};

use async_trait::async_trait;

use crate::{
    ports::network::{HttpClient, HttpResponse},
    shared::WorkflowError
};

/// Reqwest implementation of HttpClient
pub struct ReqwestClient {
    client: reqwest::Client
}

impl ReqwestClient {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }

    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new())
        }
    }

    /// Convert reqwest headers to HashMap
    fn headers_to_map(headers: &reqwest::header::HeaderMap) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for (key, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                map.insert(key.to_string(), value_str.to_string());
            }
        }
        map
    }

    /// Add headers to request builder
    fn add_headers(
        mut builder: reqwest::RequestBuilder,
        headers: Option<HashMap<String, String>>
    ) -> reqwest::RequestBuilder {
        if let Some(headers) = headers {
            for (key, value) in headers {
                builder = builder.header(&key, &value);
            }
        }
        builder
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpClient for ReqwestClient {
    async fn get(&self, url: &str, headers: Option<HashMap<String, String>>) -> Result<HttpResponse, WorkflowError> {
        let builder = self.client.get(url);
        let builder = Self::add_headers(builder, headers);

        let response =
            builder.send().await.map_err(|e| WorkflowError::Network(format!("GET request failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let response_headers = Self::headers_to_map(response.headers());

        let body = response
            .text()
            .await
            .map_err(|e| WorkflowError::Network(format!("Failed to read response body: {}", e)))?;

        Ok(HttpResponse::new(status_code, body).with_headers(response_headers))
    }

    async fn post_json(
        &self,
        url: &str,
        body: &str,
        headers: Option<HashMap<String, String>>
    ) -> Result<HttpResponse, WorkflowError> {
        let builder = self.client.post(url).header("Content-Type", "application/json").body(body.to_string());

        let builder = Self::add_headers(builder, headers);

        let response =
            builder.send().await.map_err(|e| WorkflowError::Network(format!("POST request failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let response_headers = Self::headers_to_map(response.headers());

        let response_body = response
            .text()
            .await
            .map_err(|e| WorkflowError::Network(format!("Failed to read response body: {}", e)))?;

        Ok(HttpResponse::new(status_code, response_body).with_headers(response_headers))
    }

    async fn put_json(
        &self,
        url: &str,
        body: &str,
        headers: Option<HashMap<String, String>>
    ) -> Result<HttpResponse, WorkflowError> {
        let builder = self.client.put(url).header("Content-Type", "application/json").body(body.to_string());

        let builder = Self::add_headers(builder, headers);

        let response =
            builder.send().await.map_err(|e| WorkflowError::Network(format!("PUT request failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let response_headers = Self::headers_to_map(response.headers());

        let response_body = response
            .text()
            .await
            .map_err(|e| WorkflowError::Network(format!("Failed to read response body: {}", e)))?;

        Ok(HttpResponse::new(status_code, response_body).with_headers(response_headers))
    }

    async fn delete(&self, url: &str, headers: Option<HashMap<String, String>>) -> Result<HttpResponse, WorkflowError> {
        let builder = self.client.delete(url);
        let builder = Self::add_headers(builder, headers);

        let response =
            builder.send().await.map_err(|e| WorkflowError::Network(format!("DELETE request failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let response_headers = Self::headers_to_map(response.headers());

        let body = response
            .text()
            .await
            .map_err(|e| WorkflowError::Network(format!("Failed to read response body: {}", e)))?;

        Ok(HttpResponse::new(status_code, body).with_headers(response_headers))
    }

    async fn download_file(&self, url: &str, destination: &Path) -> Result<(), WorkflowError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| WorkflowError::Network(format!("Download request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(WorkflowError::Network(format!("Download failed with status: {}", response.status())));
        }

        // Ensure parent directory exists
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| WorkflowError::Network(format!("Failed to read response bytes: {}", e)))?;

        tokio::fs::write(destination, &bytes).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;

        Ok(())
    }

    async fn is_reachable(&self, url: &str) -> Result<bool, WorkflowError> {
        match self.client.head(url).timeout(std::time::Duration::from_secs(10)).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false) // Any error means not reachable
        }
    }
}
