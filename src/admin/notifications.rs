use super::errors::AdminError;
use super::types::*;
use super::{AdminClient, RequestOptions};

impl AdminClient {
    pub async fn list_notification_channels(
        &self,
        project_id: &str,
        params: Option<&ListParams>,
    ) -> Result<ListNotificationChannelsResponse, AdminError> {
        self.list_notification_channels_with_opts(project_id, params, None)
            .await
    }

    pub async fn list_notification_channels_with_opts(
        &self,
        project_id: &str,
        params: Option<&ListParams>,
        opts: Option<&RequestOptions>,
    ) -> Result<ListNotificationChannelsResponse, AdminError> {
        let url = self.url(&format!("/v1/projects/{project_id}/notification-channels"));
        let mut builder = self.http.get(&url).headers(self.auth_headers());
        if let Some(p) = params {
            let pairs = p.to_query_pairs();
            if !pairs.is_empty() {
                builder = builder.query(&pairs);
            }
        }
        self.do_request(builder, opts).await
    }

    pub async fn get_notification_channel(
        &self,
        project_id: &str,
        channel_id: &str,
    ) -> Result<NotificationChannel, AdminError> {
        self.get_notification_channel_with_opts(project_id, channel_id, None)
            .await
    }

    pub async fn get_notification_channel_with_opts(
        &self,
        project_id: &str,
        channel_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<NotificationChannel, AdminError> {
        let builder = self
            .http
            .get(self.url(&format!(
                "/v1/projects/{project_id}/notification-channels/{channel_id}"
            )))
            .headers(self.auth_headers());
        self.do_request(builder, opts).await
    }

    pub async fn create_notification_channel(
        &self,
        project_id: &str,
        input: &CreateNotificationChannelInput,
    ) -> Result<NotificationChannel, AdminError> {
        self.create_notification_channel_with_opts(project_id, input, None)
            .await
    }

    pub async fn create_notification_channel_with_opts(
        &self,
        project_id: &str,
        input: &CreateNotificationChannelInput,
        opts: Option<&RequestOptions>,
    ) -> Result<NotificationChannel, AdminError> {
        let builder = self
            .http
            .post(self.url(&format!("/v1/projects/{project_id}/notification-channels")))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn update_notification_channel(
        &self,
        project_id: &str,
        channel_id: &str,
        input: &UpdateNotificationChannelInput,
    ) -> Result<NotificationChannel, AdminError> {
        self.update_notification_channel_with_opts(project_id, channel_id, input, None)
            .await
    }

    pub async fn update_notification_channel_with_opts(
        &self,
        project_id: &str,
        channel_id: &str,
        input: &UpdateNotificationChannelInput,
        opts: Option<&RequestOptions>,
    ) -> Result<NotificationChannel, AdminError> {
        let builder = self
            .http
            .patch(self.url(&format!(
                "/v1/projects/{project_id}/notification-channels/{channel_id}"
            )))
            .headers(self.auth_headers())
            .json(input);
        self.do_request(builder, opts).await
    }

    pub async fn delete_notification_channel(
        &self,
        project_id: &str,
        channel_id: &str,
    ) -> Result<(), AdminError> {
        self.delete_notification_channel_with_opts(project_id, channel_id, None)
            .await
    }

    pub async fn delete_notification_channel_with_opts(
        &self,
        project_id: &str,
        channel_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<(), AdminError> {
        let builder = self
            .http
            .delete(self.url(&format!(
                "/v1/projects/{project_id}/notification-channels/{channel_id}"
            )))
            .headers(self.auth_headers());
        self.do_request_no_content(builder, opts).await
    }

    pub async fn test_notification_channel(
        &self,
        project_id: &str,
        channel_id: &str,
    ) -> Result<(), AdminError> {
        self.test_notification_channel_with_opts(project_id, channel_id, None)
            .await
    }

    pub async fn test_notification_channel_with_opts(
        &self,
        project_id: &str,
        channel_id: &str,
        opts: Option<&RequestOptions>,
    ) -> Result<(), AdminError> {
        let builder = self
            .http
            .post(self.url(&format!(
                "/v1/projects/{project_id}/notification-channels/{channel_id}/test"
            )))
            .headers(self.auth_headers());
        self.do_request_no_content(builder, opts).await
    }
}
