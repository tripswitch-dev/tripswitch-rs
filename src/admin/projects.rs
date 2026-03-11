use super::types::*;
use super::errors::AdminError;
use super::{AdminClient, RequestOptions};

impl AdminClient {
    pub async fn list_projects(
        &self,
        params: Option<&ListParams>,
        opts: Option<&RequestOptions>,
    ) -> Result<ListProjectsResponse, AdminError> {
        let url = self.url("/v1/projects");
        let mut builder = self.http.get(&url).headers(self.auth_headers());
        if let Some(p) = params {
            let pairs = p.to_query_pairs();
            if !pairs.is_empty() {
                builder = builder.query(&pairs);
            }
        }
        self.do_request(builder, opts).await
    }

    pub async fn get_project(
        &self,
        project_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<Project, AdminError> {
        let builder = self
            .http
            .get(self.url(&format!("/v1/projects/{project_id}")))
            .headers(self.auth_headers());
        self.do_request(builder, opts).await
    }

    pub async fn create_project(
        &self,
        input: &CreateProjectInput,
        opts: Option<&RequestOptions>,
    ) -> Result<Project, AdminError> {
        let builder = self
            .http
            .post(self.url("/v1/projects"))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn update_project(
        &self,
        project_id: &str,
        input: &UpdateProjectInput,
        opts: Option<&RequestOptions>,
    ) -> Result<Project, AdminError> {
        let builder = self
            .http
            .patch(self.url(&format!("/v1/projects/{project_id}")))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn delete_project(
        &self,
        project_id: &str,
        confirm_name: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<(), AdminError> {
        let builder = self
            .http
            .delete(self.url(&format!("/v1/projects/{project_id}")))
            .headers(self.auth_headers())
            .json(&serde_json::json!({ "confirm_name": confirm_name }));
        self.do_request_no_content(builder, opts).await
    }

    pub async fn rotate_ingest_secret(
        &self,
        project_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<IngestSecretRotation, AdminError> {
        let builder = self
            .http
            .post(self.url(&format!(
                "/v1/projects/{project_id}/ingest_secret/rotate"
            )))
            .headers(self.auth_headers());
        self.do_request(builder, opts).await
    }
}
