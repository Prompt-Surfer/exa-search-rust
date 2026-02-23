use crate::types::{
    ContentsResponse, FindSimilarOptions, FindSimilarResponse, GetContentsOptions, SearchOptions,
    SearchResponse,
};
use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Client;

const EXA_BASE_URL: &str = "https://api.exa.ai";

pub struct ExaClient {
    client: Client,
    api_key: String,
}

impl ExaClient {
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self { client, api_key })
    }

    fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        // api_key is ASCII; from_str only fails on non-ASCII/control chars
        #[allow(clippy::expect_used)]
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.api_key).expect("api_key must be valid ASCII"),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    pub async fn search(&self, opts: SearchOptions) -> Result<SearchResponse> {
        let url = format!("{EXA_BASE_URL}/search");
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&opts)
            .send()
            .await
            .context("HTTP request failed")?;
        self.parse_response(resp).await
    }

    pub async fn find_similar(&self, opts: FindSimilarOptions) -> Result<FindSimilarResponse> {
        let url = format!("{EXA_BASE_URL}/findSimilar");
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&opts)
            .send()
            .await
            .context("HTTP request failed")?;
        self.parse_response(resp).await
    }

    pub async fn get_contents(&self, opts: GetContentsOptions) -> Result<ContentsResponse> {
        let url = format!("{EXA_BASE_URL}/contents");
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .json(&opts)
            .send()
            .await
            .context("HTTP request failed")?;
        self.parse_response(resp).await
    }

    /// Parse a response: propagate HTTP errors, then deserialize JSON.
    async fn parse_response<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T> {
        let status = resp.status();
        let body = resp.text().await.context("Failed to read response body")?;
        if !status.is_success() {
            anyhow::bail!("Exa API error {status}: {body}");
        }
        serde_json::from_str(&body).with_context(|| format!("Failed to parse API response: {body}"))
    }
}
