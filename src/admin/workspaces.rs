use super::errors::AdminError;
use super::types::*;
use super::{AdminClient, RequestOptions};

impl AdminClient {
    pub async fn list_workspaces(&self) -> Result<ListWorkspacesResponse, AdminError> {
        self.list_workspaces_with_opts(None).await
    }

    pub async fn list_workspaces_with_opts(
        &self,
        opts: Option<&RequestOptions>,
    ) -> Result<ListWorkspacesResponse, AdminError> {
        let builder = self
            .http
            .get(self.url("/v1/workspaces"))
            .headers(self.auth_headers());
        self.do_request(builder, opts).await
    }

    pub async fn create_workspace(
        &self,
        input: &CreateWorkspaceInput,
    ) -> Result<Workspace, AdminError> {
        self.create_workspace_with_opts(input, None).await
    }

    pub async fn create_workspace_with_opts(
        &self,
        input: &CreateWorkspaceInput,
        opts: Option<&RequestOptions>,
    ) -> Result<Workspace, AdminError> {
        let builder = self
            .http
            .post(self.url("/v1/workspaces"))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn get_workspace(&self, workspace_id: &str) -> Result<Workspace, AdminError> {
        self.get_workspace_with_opts(workspace_id, None).await
    }

    pub async fn get_workspace_with_opts(
        &self,
        workspace_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<Workspace, AdminError> {
        let builder = self
            .http
            .get(self.url(&format!("/v1/workspaces/{workspace_id}")))
            .headers(self.auth_headers());
        self.do_request(builder, opts).await
    }

    pub async fn update_workspace(
        &self,
        workspace_id: &str,
        input: &UpdateWorkspaceInput,
    ) -> Result<Workspace, AdminError> {
        self.update_workspace_with_opts(workspace_id, input, None)
            .await
    }

    pub async fn update_workspace_with_opts(
        &self,
        workspace_id: &str,
        input: &UpdateWorkspaceInput,
        opts: Option<&RequestOptions>,
    ) -> Result<Workspace, AdminError> {
        let builder = self
            .http
            .patch(self.url(&format!("/v1/workspaces/{workspace_id}")))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn delete_workspace(&self, workspace_id: &str) -> Result<(), AdminError> {
        self.delete_workspace_with_opts(workspace_id, None).await
    }

    pub async fn delete_workspace_with_opts(
        &self,
        workspace_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<(), AdminError> {
        let builder = self
            .http
            .delete(self.url(&format!("/v1/workspaces/{workspace_id}")))
            .headers(self.auth_headers());
        self.do_request_no_content(builder, opts).await
    }
}
