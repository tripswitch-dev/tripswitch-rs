use super::types::*;
use super::errors::AdminError;
use super::{AdminClient, RequestOptions};

impl AdminClient {
    pub async fn list_events(
        &self,
        project_id: &str,
        params: Option<&ListEventsParams>,
        opts: Option<&RequestOptions>,
    ) -> Result<ListEventsResponse, AdminError> {
        let url = self.url(&format!("/v1/projects/{project_id}/events"));
        let mut builder = self.http.get(&url).headers(self.auth_headers());
        if let Some(p) = params {
            let pairs = p.to_query_pairs();
            if !pairs.is_empty() {
                builder = builder.query(&pairs);
            }
        }
        self.do_request(builder, opts).await
    }
}
