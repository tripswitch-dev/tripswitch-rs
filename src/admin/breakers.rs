use std::collections::HashMap;

use super::errors::AdminError;
use super::types::*;
use super::{AdminClient, RequestOptions};

impl AdminClient {
    pub async fn list_breakers(
        &self,
        project_id: &str,
        params: Option<&ListParams>,
    ) -> Result<ListBreakersResponse, AdminError> {
        self.list_breakers_with_opts(project_id, params, None).await
    }

    pub async fn list_breakers_with_opts(
        &self,
        project_id: &str,
        params: Option<&ListParams>,
        opts: Option<&RequestOptions>,
    ) -> Result<ListBreakersResponse, AdminError> {
        let url = self.url(&format!("/v1/projects/{project_id}/breakers"));
        let mut builder = self.http.get(&url).headers(self.auth_headers());
        if let Some(p) = params {
            let pairs = p.to_query_pairs();
            if !pairs.is_empty() {
                builder = builder.query(&pairs);
            }
        }
        self.do_request(builder, opts).await
    }

    pub async fn get_breaker(
        &self,
        project_id: &str,
        breaker_id: &str,
    ) -> Result<Breaker, AdminError> {
        self.get_breaker_with_opts(project_id, breaker_id, None)
            .await
    }

    pub async fn get_breaker_with_opts(
        &self,
        project_id: &str,
        breaker_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<Breaker, AdminError> {
        let builder = self
            .http
            .get(self.url(&format!("/v1/projects/{project_id}/breakers/{breaker_id}")))
            .headers(self.auth_headers());
        self.do_request(builder, opts).await
    }

    pub async fn create_breaker(
        &self,
        project_id: &str,
        input: &CreateBreakerInput,
    ) -> Result<Breaker, AdminError> {
        self.create_breaker_with_opts(project_id, input, None).await
    }

    pub async fn create_breaker_with_opts(
        &self,
        project_id: &str,
        input: &CreateBreakerInput,
        opts: Option<&RequestOptions>,
    ) -> Result<Breaker, AdminError> {
        let builder = self
            .http
            .post(self.url(&format!("/v1/projects/{project_id}/breakers")))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn update_breaker(
        &self,
        project_id: &str,
        breaker_id: &str,
        input: &UpdateBreakerInput,
    ) -> Result<Breaker, AdminError> {
        self.update_breaker_with_opts(project_id, breaker_id, input, None)
            .await
    }

    pub async fn update_breaker_with_opts(
        &self,
        project_id: &str,
        breaker_id: &str,
        input: &UpdateBreakerInput,
        opts: Option<&RequestOptions>,
    ) -> Result<Breaker, AdminError> {
        let builder = self
            .http
            .patch(self.url(&format!("/v1/projects/{project_id}/breakers/{breaker_id}")))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn delete_breaker(
        &self,
        project_id: &str,
        breaker_id: &str,
    ) -> Result<(), AdminError> {
        self.delete_breaker_with_opts(project_id, breaker_id, None)
            .await
    }

    pub async fn delete_breaker_with_opts(
        &self,
        project_id: &str,
        breaker_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<(), AdminError> {
        let builder = self
            .http
            .delete(self.url(&format!("/v1/projects/{project_id}/breakers/{breaker_id}")))
            .headers(self.auth_headers());
        self.do_request_no_content(builder, opts).await
    }

    pub async fn sync_breakers(
        &self,
        project_id: &str,
        input: &SyncBreakersInput,
    ) -> Result<Vec<Breaker>, AdminError> {
        self.sync_breakers_with_opts(project_id, input, None).await
    }

    pub async fn sync_breakers_with_opts(
        &self,
        project_id: &str,
        input: &SyncBreakersInput,
        opts: Option<&RequestOptions>,
    ) -> Result<Vec<Breaker>, AdminError> {
        let builder = self
            .http
            .put(self.url(&format!("/v1/projects/{project_id}/breakers")))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn get_breaker_state(
        &self,
        project_id: &str,
        breaker_id: &str,
    ) -> Result<BreakerState, AdminError> {
        self.get_breaker_state_with_opts(project_id, breaker_id, None)
            .await
    }

    pub async fn get_breaker_state_with_opts(
        &self,
        project_id: &str,
        breaker_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<BreakerState, AdminError> {
        let builder = self
            .http
            .get(self.url(&format!(
                "/v1/projects/{project_id}/breakers/{breaker_id}/state"
            )))
            .headers(self.auth_headers());
        self.do_request(builder, opts).await
    }

    pub async fn batch_get_breaker_states(
        &self,
        project_id: &str,
        input: &BatchGetBreakerStatesInput,
    ) -> Result<Vec<BreakerState>, AdminError> {
        self.batch_get_breaker_states_with_opts(project_id, input, None)
            .await
    }

    pub async fn batch_get_breaker_states_with_opts(
        &self,
        project_id: &str,
        input: &BatchGetBreakerStatesInput,
        opts: Option<&RequestOptions>,
    ) -> Result<Vec<BreakerState>, AdminError> {
        let builder = self
            .http
            .post(self.url(&format!("/v1/projects/{project_id}/breakers/state:batch")))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn update_breaker_metadata(
        &self,
        project_id: &str,
        breaker_id: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<Breaker, AdminError> {
        self.update_breaker_metadata_with_opts(project_id, breaker_id, metadata, None)
            .await
    }

    pub async fn update_breaker_metadata_with_opts(
        &self,
        project_id: &str,
        breaker_id: &str,
        metadata: &HashMap<String, String>,
        opts: Option<&RequestOptions>,
    ) -> Result<Breaker, AdminError> {
        let builder = self
            .http
            .patch(self.url(&format!(
                "/v1/projects/{project_id}/breakers/{breaker_id}/metadata"
            )))
            .headers(self.auth_headers())
            .json(&serde_json::json!({ "metadata": metadata }));
        self.do_request(builder, opts).await
    }
}
