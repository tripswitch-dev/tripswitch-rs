use super::types::*;
use super::errors::AdminError;
use super::{AdminClient, RequestOptions};

impl AdminClient {
    pub async fn list_routers(
        &self,
        project_id: &str,
        params: Option<&ListParams>,
        opts: Option<&RequestOptions>,
    ) -> Result<ListRoutersResponse, AdminError> {
        let url = self.url(&format!("/v1/projects/{project_id}/routers"));
        let mut builder = self.http.get(&url).headers(self.auth_headers());
        if let Some(p) = params {
            let pairs = p.to_query_pairs();
            if !pairs.is_empty() {
                builder = builder.query(&pairs);
            }
        }
        self.do_request(builder, opts).await
    }

    pub async fn get_router(
        &self,
        project_id: &str,
        router_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<Router, AdminError> {
        let builder = self
            .http
            .get(self.url(&format!(
                "/v1/projects/{project_id}/routers/{router_id}"
            )))
            .headers(self.auth_headers());
        self.do_request(builder, opts).await
    }

    pub async fn create_router(
        &self,
        project_id: &str,
        input: &CreateRouterInput,
        opts: Option<&RequestOptions>,
    ) -> Result<Router, AdminError> {
        let builder = self
            .http
            .post(self.url(&format!("/v1/projects/{project_id}/routers")))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn update_router(
        &self,
        project_id: &str,
        router_id: &str,
        input: &UpdateRouterInput,
        opts: Option<&RequestOptions>,
    ) -> Result<Router, AdminError> {
        let builder = self
            .http
            .patch(self.url(&format!(
                "/v1/projects/{project_id}/routers/{router_id}"
            )))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn delete_router(
        &self,
        project_id: &str,
        router_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<(), AdminError> {
        let builder = self
            .http
            .delete(self.url(&format!(
                "/v1/projects/{project_id}/routers/{router_id}"
            )))
            .headers(self.auth_headers());
        self.do_request_no_content(builder, opts).await
    }

    pub async fn link_breaker(
        &self,
        project_id: &str,
        router_id: &str,
        input: &LinkBreakerInput,
        opts: Option<&RequestOptions>,
    ) -> Result<(), AdminError> {
        let builder = self
            .http
            .post(self.url(&format!(
                "/v1/projects/{project_id}/routers/{router_id}/breakers"
            )))
            .headers(self.auth_headers())
            .json(input);
        self.do_request_no_content(builder, opts).await
    }

    pub async fn unlink_breaker(
        &self,
        project_id: &str,
        router_id: &str,
        breaker_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<(), AdminError> {
        let builder = self
            .http
            .delete(self.url(&format!(
                "/v1/projects/{project_id}/routers/{router_id}/breakers/{breaker_id}"
            )))
            .headers(self.auth_headers());
        self.do_request_no_content(builder, opts).await
    }

    pub async fn update_router_metadata(
        &self,
        project_id: &str,
        router_id: &str,
        metadata: &serde_json::Value,
        opts: Option<&RequestOptions>,
    ) -> Result<Router, AdminError> {
        let builder = self
            .http
            .patch(self.url(&format!(
                "/v1/projects/{project_id}/routers/{router_id}/metadata"
            )))
            .headers(self.auth_headers())
            .json(&serde_json::json!({ "metadata": metadata }));
        self.do_request(builder, opts).await
    }
}
