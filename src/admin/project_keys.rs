use super::types::*;
use super::errors::AdminError;
use super::{AdminClient, RequestOptions};

impl AdminClient {
    pub async fn list_project_keys(
        &self,
        project_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<ListProjectKeysResponse, AdminError> {
        let builder = self
            .http
            .get(self.url(&format!("/v1/projects/{project_id}/keys")))
            .headers(self.auth_headers());
        self.do_request(builder, opts).await
    }

    pub async fn create_project_key(
        &self,
        project_id: &str,
        input: &CreateProjectKeyInput,
        opts: Option<&RequestOptions>,
    ) -> Result<CreateProjectKeyResponse, AdminError> {
        let builder = self
            .http
            .post(self.url(&format!("/v1/projects/{project_id}/keys")))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn delete_project_key(
        &self,
        project_id: &str,
        key_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<(), AdminError> {
        let builder = self
            .http
            .delete(self.url(&format!(
                "/v1/projects/{project_id}/keys/{key_id}"
            )))
            .headers(self.auth_headers());
        self.do_request_no_content(builder, opts).await
    }
}
