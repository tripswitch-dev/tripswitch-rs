use super::pager::{CursorPage, Pager};
use super::types::*;
use super::AdminClient;

impl AdminClient {
    /// Create a pager that iterates over all notification channels in a project.
    pub fn list_notification_channels_pager(
        &self,
        project_id: impl Into<String>,
        limit: Option<i64>,
    ) -> Pager<NotificationChannel> {
        let client = self.clone();
        let pid = project_id.into();
        Pager::new(Box::new(move |cursor| {
            let client = client.clone();
            let pid = pid.clone();
            let limit = limit;
            Box::pin(async move {
                let params = ListParams { cursor, limit };
                let resp = client
                    .list_notification_channels(&pid, Some(&params))
                    .await?;
                Ok(CursorPage {
                    items: resp.channels,
                    next_cursor: resp.next_cursor,
                })
            })
        }))
    }

    /// Create a pager that iterates over all events in a project.
    pub fn list_events_pager(
        &self,
        project_id: impl Into<String>,
        params: Option<ListEventsParams>,
    ) -> Pager<Event> {
        let client = self.clone();
        let pid = project_id.into();
        Pager::new(Box::new(move |cursor| {
            let client = client.clone();
            let pid = pid.clone();
            let mut p = params.clone().unwrap_or_default();
            p.cursor = cursor;
            Box::pin(async move {
                let resp = client.list_events(&pid, Some(&p)).await?;
                Ok(CursorPage {
                    items: resp.events,
                    next_cursor: resp.next_cursor,
                })
            })
        }))
    }
}
